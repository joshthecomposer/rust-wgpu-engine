use std::{
    collections::{HashMap, HashSet},
    mem::size_of,
};

use wgpu::util::DeviceExt;

use crate::{
    debug::gizmos::GizmoMesh,
    entity_manager::EntityManager,
    enums_types::{InstanceUniform, Transform},
    wgpu_backend::pipelines::gizmo::GizmoPipeline,
};

const MAX_GIZMO_INSTANCES: u64 = 4_096;

/// GPU residency for one entity's `GizmoMesh`. Vertices are uploaded as the
/// shared `Vertex` layout (matching `static_model`); the index buffer holds
/// deduplicated edges (`LineList`) derived from the source triangle indices.
struct GpuGizmoMesh {
    vertex_buffer: wgpu::Buffer,
    line_index_buffer: wgpu::Buffer,
    line_index_count: u32,
}

/// One queued line draw recorded by `prepare`, consumed by `render`.
struct GizmoDraw {
    entity_id: usize,
    instance_offset: wgpu::BufferAddress,
}

pub struct GizmoRenderer {
    pipeline: GizmoPipeline,
    /// Per-entity GPU mesh, lazily uploaded on first sight.
    /// Each entity owns a unique `GizmoMesh` (see `EntityManager::collider_gizmos`),
    /// so caching is keyed by entity id rather than mesh contents.
    mesh_cache: HashMap<usize, GpuGizmoMesh>,
    instance_buffer: wgpu::Buffer,
    draws: Vec<GizmoDraw>,
}

impl GizmoRenderer {
    pub fn new(pipeline: GizmoPipeline, device: &wgpu::Device) -> Self {
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gizmo_instance_buffer"),
            size: MAX_GIZMO_INSTANCES * size_of::<InstanceUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            mesh_cache: HashMap::new(),
            instance_buffer,
            draws: Vec::new(),
        }
    }

    /// Walks every live gizmo entity, lazily uploads any missing GPU meshes,
    /// evicts cache entries whose entities have been despawned, and packs the
    /// per-instance model matrices into `instance_buffer`.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        em: &EntityManager,
        alpha: f32,
    ) {
        self.draws.clear();

        let ids = em.get_gizmo_ids();

        // Evict GPU meshes for entities that no longer have a gizmo (despawned).
        if self.mesh_cache.len() > ids.len() {
            let live: HashSet<usize> = ids.iter().copied().collect();
            self.mesh_cache.retain(|id, _| live.contains(id));
        }

        let stride = size_of::<InstanceUniform>() as wgpu::BufferAddress;
        let mut offset: wgpu::BufferAddress = 0;
        let mut instances: Vec<InstanceUniform> = Vec::with_capacity(ids.len());

        for id in ids {
            let (Some(curr), Some(prev), Some(mesh)) = (
                em.collider_transforms.get(id),
                em.prev_collider_transforms.get(id),
                em.collider_gizmos.get(id),
            ) else {
                continue;
            };

            self.mesh_cache
                .entry(id)
                .or_insert_with(|| upload_gizmo_mesh(device, mesh));

            let instance = Transform::interpolated(prev, curr, alpha).to_instance_uniform();
            instances.push(instance);

            self.draws.push(GizmoDraw {
                entity_id: id,
                instance_offset: offset,
            });

            offset += stride;
        }

        if instances.is_empty() {
            return;
        }

        let bytes = bytemuck::cast_slice(&instances);

        debug_assert!(
            bytes.len() as wgpu::BufferAddress <= self.instance_buffer.size(),
            "gizmo instance_buffer too small ({} entities)",
            instances.len(),
        );

        queue.write_buffer(&self.instance_buffer, 0, bytes);
    }

    /// Issues one draw per gizmo. The camera bind group is bound at slot 0
    /// (matching `pipelines::gizmo`'s pipeline layout) *after* `set_pipeline`
    /// because switching pipelines with differing bind-group layouts at slot 0
    /// invalidates whatever was previously bound there.
    pub fn render(&self, rp: &mut wgpu::RenderPass, camera_bg: &wgpu::BindGroup) {
        if self.draws.is_empty() {
            return;
        }

        rp.set_pipeline(&self.pipeline.pipeline);
        rp.set_bind_group(0, camera_bg, &[]);

        let stride = size_of::<InstanceUniform>() as wgpu::BufferAddress;

        for draw in &self.draws {
            let Some(mesh) = self.mesh_cache.get(&draw.entity_id) else {
                continue;
            };

            rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            rp.set_vertex_buffer(
                1,
                self.instance_buffer
                    .slice(draw.instance_offset..draw.instance_offset + stride),
            );
            rp.set_index_buffer(mesh.line_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..mesh.line_index_count, 0, 0..1);
        }
    }
}

fn upload_gizmo_mesh(device: &wgpu::Device, mesh: &GizmoMesh) -> GpuGizmoMesh {
    let line_indices = derive_line_indices(&mesh.indices);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("gizmo_vertex_buffer"),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let line_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("gizmo_line_index_buffer"),
        contents: bytemuck::cast_slice(&line_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    GpuGizmoMesh {
        vertex_buffer,
        line_index_buffer,
        line_index_count: line_indices.len() as u32,
    }
}

/// Derive a deduplicated `LineList` index buffer from a `TriangleList` index
/// buffer. Each triangle contributes its three edges; duplicates (shared edges
/// between adjacent triangles) collapse into one line via the `(min,max)` key.
fn derive_line_indices(triangle_indices: &[u32]) -> Vec<u32> {
    let mut seen: HashSet<(u32, u32)> = HashSet::with_capacity(triangle_indices.len());
    let mut out: Vec<u32> = Vec::with_capacity(triangle_indices.len());

    for tri in triangle_indices.chunks_exact(3) {
        let edges = [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])];
        for (a, b) in edges {
            if a == b {
                continue;
            }
            let key = if a < b { (a, b) } else { (b, a) };
            if seen.insert(key) {
                out.push(a);
                out.push(b);
            }
        }
    }

    out
}
