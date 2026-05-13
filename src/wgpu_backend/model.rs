use crate::wgpu_backend::{material::Material, vertex::Vertex};

#[derive(Clone)]
pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,

    pub num_elements: u32,

    pub material: Material,

    pub directory: String,
    pub full_path: String,
}

pub trait DrawModel<'a> {
    fn draw_model(&mut self, model: &'a Model);
    fn draw_model_instanced(&mut self, model: &'a Model, instances: std::ops::Range<u32>);
    fn draw_model_animated(
        &mut self,
        model: &'a Model,
        bone_bind_group: &'a wgpu::BindGroup,
        bones_dynamic_offset: wgpu::DynamicOffset,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a> {
    fn draw_model(&mut self, model: &'b Model) {
        self.draw_model_instanced(model, 0..1)
    }

    fn draw_model_instanced(&mut self, model: &'b Model, instances: std::ops::Range<u32>) {
        self.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        self.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &model.material.bind_group, &[]);
        self.draw_indexed(0..model.num_elements, 0, instances);
    }

    fn draw_model_animated(
        &mut self,
        model: &'b Model,
        bone_bind_group: &'b wgpu::BindGroup,
        bones_dynamic_offset: wgpu::DynamicOffset,
    ) {
        self.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        self.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &model.material.bind_group, &[]);
        self.set_bind_group(2, bone_bind_group, &[bones_dynamic_offset]);
        self.draw_indexed(0..model.num_elements, 0, 0..1);
    }
}

/// Depth-only draws for the shadow prepass (no material texture bind group).
pub trait DrawDepthOnly<'a> {
    fn draw_model_depth_only(&mut self, model: &'a Model, instances: std::ops::Range<u32>);
    fn draw_model_depth_only_animated(
        &mut self,
        model: &'a Model,
        bone_bind_group: &'a wgpu::BindGroup,
        bones_dynamic_offset: wgpu::DynamicOffset,
        bones_bind_group_index: u32,
    );
}

impl DrawDepthOnly<'_> for wgpu::RenderPass<'_> {
    fn draw_model_depth_only(&mut self, model: &Model, instances: std::ops::Range<u32>) {
        self.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        self.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..model.num_elements, 0, instances);
    }

    fn draw_model_depth_only_animated(
        &mut self,
        model: &Model,
        bone_bind_group: &wgpu::BindGroup,
        bones_dynamic_offset: wgpu::DynamicOffset,
        bones_bind_group_index: u32,
    ) {
        self.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        self.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(
            bones_bind_group_index,
            bone_bind_group,
            &[bones_dynamic_offset],
        );
        self.draw_indexed(0..model.num_elements, 0, 0..1);
    }
}
