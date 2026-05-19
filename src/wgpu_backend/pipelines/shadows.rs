use crate::{camera::Camera, lights::Lights};

pub struct ShadowResources {
    pub pipeline: wgpu::RenderPipeline,
}

impl ShadowResources {
    pub fn draw(&self, light_manager: &Lights, camera: &mut Camera) {
        let near_plane = light_manager.near;
        let far_plane = light_manager.far;
        let half_bound = light_manager.bounds;

        let light_dir = light_manager.dir_light.direction.normalize();
        let light_distance = light_manager.dir_light.distance;

        let camera_forward = camera.forward.normalize();
        let shadow_push = half_bound * 1.2;
        let shadow_center = camera.position + camera_forward * shadow_push;
        let light_pos = shadow_center + light_dir * light_distance;

        let light_projection = glam::Mat4::orthographic_rh(
            -half_bound,
            half_bound,
            -half_bound,
            half_bound,
            near_plane,
            far_plane,
        );

        let light_view =
            glam::Mat4::look_at_rh(light_pos, shadow_center, glam::vec3(0.0, 1.0, 0.0));

        camera.light_space = light_projection * light_view;
    }
}
