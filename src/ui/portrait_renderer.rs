//! Portrait renderer for the player HUD.
//! Renders the player model to an offscreen FBO from a third-person view.
//!
//! On WebAssembly, framebuffer/renderbuffer/delete/get calls go through `platform::web_canvas`
//! shims backed by [`web_sys::WebGl2RenderingContext`] so bindings match JS.

/// Size of the portrait texture in pixels.
pub const PORTRAIT_SIZE: u32 = 64;

mod native {
    use glam::{Mat4, Vec3};

    use crate::animation::model::Model;
    use crate::entity_manager::EntityManager;
    use crate::enums_types::Transform;
    use crate::gl_call;
    use crate::lights::Lights;
    use crate::renderer::Renderer;
    use crate::shaders::Shader;

    /// Update interval in seconds - only render portrait this often.
    /// 30 FPS is plenty smooth for a small portrait with animations.
    const UPDATE_INTERVAL: f64 = 1.0 / 30.0;

    /// Renders entity portraits to an offscreen FBO.
    pub struct PortraitRenderer {
        fbo: u32,
        texture: u32,
        depth_rbo: u32,
        last_update_time: f64,
    }

    impl PortraitRenderer {
        /// Create a new portrait renderer with its own FBO.
        pub fn new() -> Self {
            let mut fbo = 0;
            let mut texture = 0;
            let mut depth_rbo = 0;

            unsafe {
                gl_call!(gl::GenFramebuffers(1, &mut fbo));
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, fbo));

                gl_call!(gl::GenTextures(1, &mut texture));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture));
                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA8 as i32,
                    super::PORTRAIT_SIZE as i32,
                    super::PORTRAIT_SIZE as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null()
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MAG_FILTER,
                    gl::LINEAR as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_S,
                    gl::CLAMP_TO_EDGE as i32
                ));
                gl_call!(gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_WRAP_T,
                    gl::CLAMP_TO_EDGE as i32
                ));
                gl_call!(gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    texture,
                    0
                ));

                gl_call!(gl::GenRenderbuffers(1, &mut depth_rbo));
                gl_call!(gl::BindRenderbuffer(gl::RENDERBUFFER, depth_rbo));
                gl_call!(gl::RenderbufferStorage(
                    gl::RENDERBUFFER,
                    gl::DEPTH_COMPONENT24,
                    super::PORTRAIT_SIZE as i32,
                    super::PORTRAIT_SIZE as i32
                ));
                gl_call!(gl::FramebufferRenderbuffer(
                    gl::FRAMEBUFFER,
                    gl::DEPTH_ATTACHMENT,
                    gl::RENDERBUFFER,
                    depth_rbo
                ));

                let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
                if status != gl::FRAMEBUFFER_COMPLETE {
                    panic!("Portrait FBO incomplete: 0x{:x}", status);
                }

                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
            }

            Self {
                fbo,
                texture,
                depth_rbo,
                last_update_time: -999.0,
            }
        }

        pub fn should_update(&self, elapsed_time: f64) -> bool {
            elapsed_time - self.last_update_time >= UPDATE_INTERVAL
        }

        pub fn render_portrait(
            &mut self,
            em: &crate::entity_manager::EntityManager,
            player_id: usize,
            shader: &mut crate::shaders::Shader,
            lights: &crate::lights::Lights,
            defaults: &crate::renderer::DefaultTextures,
            cubemap: u32,
            elapsed_time: f64,
        ) {
            let trans = match em.transforms.get(player_id) {
                Some(t) => t,
                None => return,
            };
            let model = match em.models.get(player_id) {
                Some(m) => m,
                None => return,
            };

            self.last_update_time = elapsed_time;

            let (view, projection) = Self::create_portrait_camera(trans);

            Self::render_to_fbo(
                self,
                em,
                player_id,
                model,
                trans,
                shader,
                &view,
                &projection,
                lights,
                defaults,
                cubemap,
            );
        }

        fn create_portrait_camera(player_trans: &Transform) -> (Mat4, Mat4) {
            let player_pos = player_trans.position;
            let player_forward = player_trans.rotation * Vec3::NEG_Z;

            let camera_distance = 1.0;
            let camera_height_offset = 1.7;
            let look_at_height_offset = 1.7;

            let camera_pos =
                player_pos - player_forward * camera_distance + Vec3::Y * camera_height_offset;
            let look_at = player_pos + Vec3::Y * look_at_height_offset;

            let view = Mat4::look_at_rh(camera_pos, look_at, Vec3::Y);
            let projection = Mat4::perspective_rh(0.8, 1.0, 0.1, 10.0);

            (view, projection)
        }

        fn render_to_fbo(
            &self,
            em: &EntityManager,
            player_id: usize,
            model: &Model,
            trans: &Transform,
            shader: &mut Shader,
            view: &Mat4,
            projection: &Mat4,
            lights: &Lights,
            defaults: &crate::renderer::DefaultTextures,
            cubemap: u32,
        ) {
            let mut prev_viewport: [i32; 4] = [0, 0, 0, 0];
            unsafe {
                gl_call!(gl::GetIntegerv(gl::VIEWPORT, prev_viewport.as_mut_ptr(),));

                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo));
                gl_call!(gl::Viewport(
                    0,
                    0,
                    super::PORTRAIT_SIZE as i32,
                    super::PORTRAIT_SIZE as i32,
                ));

                let draw_buffers: [u32; 1] = [gl::COLOR_ATTACHMENT0];
                gl_call!(gl::DrawBuffers(1, draw_buffers.as_ptr()));

                gl_call!(gl::ClearColor(0.0, 0.0, 0.0, 0.0));
                gl_call!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

                gl_call!(gl::Enable(gl::DEPTH_TEST));
                gl_call!(gl::DepthMask(gl::TRUE));
                gl_call!(gl::Disable(gl::BLEND));
                gl_call!(gl::Enable(gl::CULL_FACE));
                gl_call!(gl::CullFace(gl::BACK));
                gl_call!(gl::FrontFace(gl::CCW));

                gl_call!(gl::ActiveTexture(gl::TEXTURE1));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, defaults.white));
                gl_call!(gl::ActiveTexture(gl::TEXTURE2));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, defaults.black));
                gl_call!(gl::ActiveTexture(gl::TEXTURE3));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, defaults.black));
                gl_call!(gl::ActiveTexture(gl::TEXTURE4));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, defaults.opaque));
                gl_call!(gl::ActiveTexture(gl::TEXTURE7));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, defaults.white));
                gl_call!(gl::ActiveTexture(gl::TEXTURE10));
                gl_call!(gl::BindTexture(gl::TEXTURE_CUBE_MAP, cubemap));
            }

            shader.activate();
            shader.set_mat4("projection", *projection);
            shader.set_mat4("view", *view);
            shader.set_mat4("light_space_mat", Mat4::IDENTITY);
            shader.set_dir_light("dir_light", &lights.dir_light);
            shader.set_float("bias_scalar", lights.bias_scalar);
            shader.set_vec3("view_position", Vec3::ZERO);
            shader.set_int("skybox", 10);
            shader.set_bool("selection_fresnel", false);
            shader.set_bool("do_reg_fresnel", false);
            shader.set_bool("alpha_test_pass", false);
            shader.set_bool("flash_white", false);

            let m_mat =
                Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
            shader.set_mat4("model", m_mat);

            if let Some(animator) = em.animators.get(player_id) {
                if let Some(animation) = animator.get_current_animation() {
                    shader.set_bool("is_animated", true);
                    shader.set_mat4_array("bone_transforms", &animation.current_pose);
                } else {
                    shader.set_bool("is_animated", false);
                }
            } else {
                shader.set_bool("is_animated", false);
            }

            unsafe {
                Renderer::draw_model(model, shader);
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, 0));
                gl_call!(gl::Disable(gl::CULL_FACE));
                gl_call!(gl::BindFramebuffer(gl::FRAMEBUFFER, 0));
                gl_call!(gl::Viewport(
                    prev_viewport[0],
                    prev_viewport[1],
                    prev_viewport[2],
                    prev_viewport[3],
                ));
            }
        }

        pub fn get_texture_id(&self) -> u32 {
            self.texture
        }
    }

    impl Drop for PortraitRenderer {
        fn drop(&mut self) {
            unsafe {
                gl_call!(gl::DeleteFramebuffers(1, &self.fbo));
                gl_call!(gl::DeleteTextures(1, &self.texture));
                gl_call!(gl::DeleteRenderbuffers(1, &self.depth_rbo));
            }
        }
    }
}

pub use native::PortraitRenderer;
