#![allow(dead_code)]
use std::collections::HashMap;

use glam::{vec2, vec3, Vec3, Vec4};
use image::{ImageBuffer, Luma};
use nalgebra::Point3;
use rapier3d::prelude::{
    ColliderBuilder, ColliderSet, InteractionGroups, RigidBodyHandle, RigidBodySet,
};
use wgpu::util::DeviceExt;

use crate::{
    assets::load_binary,
    enums_types::{TextureProfile, TextureType},
    util::constants::{GROUP_TERRAIN, MAX_BONE_INFLUENCE},
    wgpu_backend::{
        material::Material,
        model::Model,
        render_context::RenderContext,
        texture::{self, Texture},
        vertex::Vertex,
    },
};

pub struct Terrain {
    vertices: Vec<[f32; 3]>,
    indices: Vec<u32>,
    normals: Vec<[f32; 3]>,

    height_map: Vec<f32>,
    width: u32,
    height: u32,
    max_height: f32,
}

impl Terrain {
    pub fn from_height_map(
        y_amplitude: f32,
        width: u32,
        height: u32,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    ) -> Self {
        let height_map: Vec<f32> = img
            .pixels()
            // TODO: replace 10.0 with some max height value
            .map(|p| p[0] as f32 / 255.0 * y_amplitude)
            .collect();

        // VERTICES
        let mut vertices = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) as usize;
                let z = height_map[i] - y_amplitude / 2.0;
                let x_pos = x as f32 - width as f32 / 2.0;
                let z_pos = y as f32 - height as f32 / 2.0;
                vertices.push([x_pos, z, z_pos])
            }
        }

        // INDICES
        let mut indices = Vec::new();
        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let top_left = y * width + x;
                let top_right = top_left + 1;
                let bottom_left = top_left + width;
                let bottom_right = bottom_left + 1;

                // first tri
                indices.push(top_left);
                indices.push(bottom_left);
                indices.push(top_right);

                // second tri
                indices.push(top_right);
                indices.push(bottom_left);
                indices.push(bottom_right);
            }
        }

        // NORMALS
        let mut normals = vec![[0.0; 3]; (width * height) as usize];

        for z in 0..height {
            for x in 0..width {
                let idx = (z * width + x) as usize;

                // surrounding heights
                let h_l = if x > 0 {
                    height_map[(z * width + (x - 1)) as usize]
                } else {
                    height_map[idx]
                };
                let h_r = if x < width - 1 {
                    height_map[(z * width + (x + 1)) as usize]
                } else {
                    height_map[idx]
                };
                let h_d = if z > 0 {
                    height_map[((z - 1) * width + x) as usize]
                } else {
                    height_map[idx]
                };
                let h_u = if z < height - 1 {
                    height_map[((z + 1) * width + x) as usize]
                } else {
                    height_map[idx]
                };

                // central difference
                let dx = h_l - h_r;
                let dz = h_d - h_u;

                let normal = Vec3::new(dx, 2.0, dz).normalize();

                normals[idx] = [normal.x, normal.y, normal.z];
            }
        }

        Self {
            vertices,
            indices,
            normals,

            height_map,

            width,
            height,
            max_height: 10.0,
        }
    }

    pub fn heights_from_image(
        y_amplitude: f32,
        img: &ImageBuffer<Luma<u8>, Vec<u8>>,
        width: u32,
        height: u32,
    ) -> (nalgebra::DMatrix<f32>, usize, usize) {
        let ncols = width as usize;
        let nrows = height as usize;

        let mut data = Vec::with_capacity(nrows * ncols);

        for row in 0..nrows {
            for col in 0..ncols {
                let p = img.get_pixel(col as u32, row as u32).0[0] as f32 / 255.0;
                data.push(p * y_amplitude - y_amplitude * 0.5); // this centers it around 0
            }
        }

        let heights = nalgebra::DMatrix::<f32>::from_row_slice(nrows, ncols, &data);

        (heights, nrows, ncols)
    }

    pub fn into_model(&mut self, rdr_ctx: &RenderContext) -> Model {
        let mut vertices = vec![];

        for (i, v) in self.vertices.iter().enumerate() {
            let n = self.normals[i];

            let tile_scale = 25.0;

            let uv = [
                ((v[0] + self.width as f32 / 2.0) / self.width as f32) * tile_scale,
                ((v[2] + self.height as f32 / 2.0) / self.height as f32) * tile_scale,
            ];

            vertices.push(Vertex {
                position: *v,
                normal: n,
                uv,

                bone_ids: [-1; MAX_BONE_INFLUENCE],
                bone_weights: [0.0; MAX_BONE_INFLUENCE],
            });
        }

        let vertex_buffer = rdr_ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&"Terrain Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = rdr_ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Terrain Index Buffer"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let directory = "resources/models/static/terrain".to_string();
        let file_name = directory.clone() + "/" + "ai_slop/dark_dirt_pixelated.png";

        let data = load_binary(&file_name);

        let texture = Texture::from_bytes(&rdr_ctx, &data, "terrain");

        let diffuse_bind_group = rdr_ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &rdr_ctx.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            });

        Model {
            vertex_buffer,
            index_buffer,

            vertices,
            indices: self.indices.clone(),

            num_elements: self.indices.len() as u32,
            directory: directory.clone(),
            full_path: directory.clone(),
            material: Material {
                diffuse_texture: texture,
                bind_group: diffuse_bind_group,
            },
        }
    }

    // pub fn get_height_at(&self, x: f32, z: f32) -> f32 {
    //     let terrain_x = x + self.width as f32 / 2.0;
    //     let terrain_z = z + self.height as f32 / 2.0;

    //     let x0 = terrain_x.floor() as i32;
    //     let z0 = terrain_z.floor() as i32;
    //     let x1 = x0 + 1;
    //     let z1 = z0 + 1;

    //     if x0 < 0 || z0 < 0 || x1 >= self.width as i32 || z1 >= self.height as i32 {
    //         return 0.0; // out of bounds so return 0.
    //     }

    //     let h00 = self.height_map[(z0 as u32 * self.width + x0 as u32) as usize];
    //     let h10 = self.height_map[(z0 as u32 * self.width + x1 as u32) as usize];
    //     let h01 = self.height_map[(z1 as u32 * self.width + x0 as u32) as usize];
    //     let h11 = self.height_map[(z1 as u32 * self.width + x1 as u32) as usize];

    //     let tx = terrain_x - x0 as f32;
    //     let tz = terrain_z - z0 as f32;

    //     let h0 = h00 * (1.0 - tx) + h10 * tx;
    //     let h1 = h01 * (1.0 - tx) + h11 * tx;
    //     let height = h0 * (1.0 - tz) + h1 * tz;

    //     height - (self.max_height / 2.0)
    // }

    // pub fn create_collider(&self) -> Collider {
    //     // Convert terrain vertex positions to Point3
    //     let vertices: Vec<Point3<f32>> = self
    //         .vertices
    //         .iter()
    //         .map(|v| Point3::new(v[0], v[1], v[2]))
    //         .collect();

    //     // Convert triangle indices (assumed u32 or usize)
    //     let indices: Vec<[u32; 3]> = self
    //         .indices
    //         .chunks(3)
    //         .map(|tri| [tri[0] as u32, tri[1] as u32, tri[2] as u32])
    //         .collect();

    //     ColliderBuilder::trimesh(vertices, indices)
    //         .expect("Some shit went wrong")
    //         .build()
    // }
}

pub fn insert_chunked_terrain_colliders(
    model: &Model, // render mesh built from the heightmap
    width: u32,
    height: u32, // heightmap dimensions (verts laid out row-major)
    chunk_w: u32,
    chunk_h: u32,
    body_handle: RigidBodyHandle,
    colliders: &mut ColliderSet,
    bodies: &mut RigidBodySet,
) {
    // Global arrays
    let g_vertices: Vec<Point3<f32>> = model.vertices.iter().map(|v| v.position.into()).collect();
    let g_indices: &[[u32; 3]] = &model
        .indices
        .chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect::<Vec<_>>();

    let w = width as u32;
    let h = height as u32;

    let x_chunks = ((w.saturating_sub(1)) + chunk_w - 1) / chunk_w;
    let z_chunks = ((h.saturating_sub(1)) + chunk_h - 1) / chunk_h;

    for cz in 0..z_chunks {
        for cx in 0..x_chunks {
            // tile bounds in grid indices +1 to include the border column/row so no cracks
            let x0 = cx * chunk_w;
            let z0 = cz * chunk_h;
            let x1 = ((x0 + chunk_w).min(w - 1)).max(x0 + 1);
            let z1 = ((z0 + chunk_h).min(h - 1)).max(z0 + 1);

            let mut local_vertices: Vec<Point3<f32>> = Vec::new();
            let mut local_indices: Vec<[u32; 3]> = Vec::new();
            let mut remap: HashMap<u32, u32> = HashMap::new();

            // Helper: does triangle AABB in grid space intersect this tile?
            let mut tri_buf = [0u32; 3];

            for tri in g_indices.iter().copied() {
                tri_buf.copy_from_slice(&tri);
                let mut minx = u32::MAX;
                let mut maxx = 0u32;
                let mut minz = u32::MAX;
                let mut maxz = 0u32;

                for &vi in &tri_buf {
                    let gx = vi % w;
                    let gz = vi / w;
                    minx = minx.min(gx);
                    maxx = maxx.max(gx);
                    minz = minz.min(gz);
                    maxz = maxz.max(gz);
                }

                let overlaps = !(maxx < x0 || minx > x1 || maxz < z0 || minz > z1);
                if !overlaps {
                    continue;
                }

                let mut tri_local = [0u32; 3];
                for (k, &gvi) in tri_buf.iter().enumerate() {
                    let li = *remap.entry(gvi).or_insert_with(|| {
                        let new_i = local_vertices.len() as u32;
                        local_vertices.push(g_vertices[gvi as usize]);
                        new_i
                    });
                    tri_local[k] = li;
                }
                local_indices.push(tri_local);
            }

            if local_indices.is_empty() {
                continue;
            }

            // vertices are already in world space
            let col = ColliderBuilder::trimesh(local_vertices, local_indices)
                .unwrap()
                .collision_groups(InteractionGroups::new(
                    GROUP_TERRAIN.into(),
                    u32::MAX.into(),
                ))
                .build();
            colliders.insert_with_parent(col, body_handle, bodies);
        }
    }
}
