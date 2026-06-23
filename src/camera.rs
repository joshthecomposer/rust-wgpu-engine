#![allow(dead_code, clippy::single_match)]

use glam::{vec3, Mat4, Vec3};
use winit::{dpi::PhysicalPosition, keyboard::KeyCode};

use crate::{
    entity_manager::EntityManager, enums_types::CameraState, input::InputState,
    physics::PhysicsState,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub inv_proj: [[f32; 4]; 4],
    pub light_view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            inv_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            light_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkyCameraUniform {
    pub view_rot: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
}
impl SkyCameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        // strip translation from view so the box stays at the origin in view space.
        let mut view_rot = camera.view;
        view_rot.w_axis = glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
        Self {
            view_rot: view_rot.to_cols_array_2d(),
            proj: camera.projection.to_cols_array_2d(),
        }
    }
}

#[derive(Debug)]
pub struct CamMoveBasis {
    pub fwd_flat: glam::Vec3,
    pub right_flat: glam::Vec3,
}

pub struct Camera {
    pub yaw: f64,
    pub pitch: f64,
    pub direction: Vec3,
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub target: Vec3,
    pub right: Vec3,
    pub fovy: f32,
    pub movement_speed: f32,
    pub sensitivity: f64,

    pub first_mousing: bool,
    pub last_x: f64,
    pub last_y: f64,

    pub z_near: f32,
    pub z_far: f32,

    pub projection: Mat4,
    pub view: Mat4,
    pub light_space: Mat4,

    pub last_f_state: bool,

    pub move_state: CameraState,

    pub distance_from_target: f32,

    pub locked_position: Vec3,
    pub locked_target: Vec3,

    pub desired_position: Vec3,
    pub desired_target: Vec3,

    // interpolation cache
    pub prev_pos: Vec3,
    pub prev_forward: Vec3,
    pub prev_up: Vec3,
    pub prev_target: Vec3,

    pub uniform: CameraUniform,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            yaw: -90.0,
            pitch: 0.0,
            direction: vec3(0.0, 0.0, -1.0),
            position: vec3(0.0, 0.0, 15.0),
            forward: vec3(0.0, 0.0, -1.0),
            up: vec3(0.0, 1.0, 0.0),
            target: vec3(0.0, 0.0, 0.0),
            right: vec3(0.0, 0.0, 0.0),
            fovy: 45.0_f32.to_radians(),
            movement_speed: 25.0,
            sensitivity: 0.1,
            first_mousing: true,
            last_x: 0.0,
            last_y: 0.0,

            z_near: 0.1,
            z_far: 2000.0,

            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            light_space: Mat4::IDENTITY,

            last_f_state: true,

            move_state: CameraState::Free,

            distance_from_target: 5.0,

            locked_position: vec3(0.0, 15.0, 0.0),
            locked_target: vec3(2.5, 0.0, 0.0),

            desired_position: vec3(0.0, 15.0, 0.0),
            desired_target: vec3(2.5, 0.0, 0.0),

            prev_pos: vec3(0.0, 0.0, 15.0),
            prev_forward: vec3(0.0, 0.0, -1.0),
            prev_up: vec3(0.0, 1.0, 0.0),
            prev_target: vec3(0.0, 0.0, 0.0),

            uniform: CameraUniform::new(),
        }
    }

    pub fn basis_for_sim(&self) -> CamMoveBasis {
        let yaw = self.yaw.to_radians() as f32;

        let f = glam::Vec3::new(-yaw.cos(), 0.0, -yaw.sin()).normalize();
        let r = f.cross(glam::Vec3::Y).normalize();

        CamMoveBasis {
            fwd_flat: f,
            right_flat: r,
        }
    }

    pub fn update(
        &mut self,
        em: &EntityManager,
        dt: f32,
        _ps: &PhysicsState,
        alpha: f32,
        input: &InputState,
        aspect: f32,
    ) {
        match self.move_state {
            CameraState::Free => {
                self.forward = self.direction.normalize();
                self.target = self.position + self.forward;
            }
            CameraState::Third => {
                if let Some(player_key) = em.factions.iter().find(|e| e.value() == "Player") {
                    let player_transform = em.transforms.get(player_key.key()).unwrap();

                    self.desired_target = player_transform.position + vec3(0.0, 1.1, 0.0);

                    let yaw_rad = self.yaw.to_radians() as f32;
                    let pitch_rad = self.pitch.to_radians() as f32;

                    let x = self.distance_from_target * yaw_rad.cos() * pitch_rad.cos();
                    let y = self.distance_from_target * pitch_rad.sin();
                    let z = self.distance_from_target * yaw_rad.sin() * pitch_rad.cos();

                    self.desired_position = self.desired_target + vec3(x, y, z);

                    self.position = self.desired_position;
                    self.target = self.desired_target;

                    self.forward = (self.target - self.position).normalize();
                }
            }
            CameraState::Locked | CameraState::Gallery => {
                self.target = self.locked_target;
                self.position = self.locked_position;
                self.forward = (self.target - self.position).normalize();
            }
        }

        self.right = self.forward.cross(vec3(0.0, 1.0, 0.0)).normalize();
        self.up = self.right.cross(self.forward).normalize();

        self.process_key_event(dt, input);

        self.projection = glam::Mat4::perspective_rh(self.fovy, aspect, self.z_near, self.z_far);

        match self.move_state {
            CameraState::Free | CameraState::Locked | CameraState::Gallery => {
                let p = self.prev_pos.lerp(self.position, alpha);
                let f = self.prev_forward.lerp(self.forward, alpha).normalize();
                let r = f.cross(glam::Vec3::Y).normalize();
                let u = r.cross(f).normalize();
                self.view = glam::Mat4::look_at_rh(p, p + f, u);
            }
            CameraState::Third => {
                let pos = self.prev_pos.lerp(self.position, alpha);
                let target = self.prev_target.lerp(self.target, alpha);
                let up = self.prev_up.lerp(self.up, alpha).normalize();
                self.view = glam::Mat4::look_at_rh(pos, target, up);
            }
        }

        self.update_uniform();
    }

    pub fn get_view_matrix(&mut self) {
        self.view = Mat4::look_at_rh(self.position, self.target, self.up);
    }

    // for wgpu uniform
    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        self.projection * self.view
    }

    pub fn update_uniform(&mut self) {
        self.uniform.view_proj = self.build_view_projection_matrix().to_cols_array_2d();
        self.uniform.inv_proj = self.projection.inverse().to_cols_array_2d();
    }

    // Call this when switching camera modes to reset the "first mouse" delta.
    pub fn sync_mouse_position_from_input(&mut self, input: &InputState) {
        self.last_x = input.mouse_pos_current.x as f64;
        self.last_y = input.mouse_pos_current.y as f64;
        self.first_mousing = true;
    }

    pub fn process_mouse_input(&mut self, dx: f64, dy: f64) {
        match self.move_state {
            CameraState::Locked | CameraState::Gallery => {}
            CameraState::Third => {
                let mut x_offset = dx as f64;
                let mut y_offset = dy as f64; // invert y

                x_offset *= self.sensitivity;
                y_offset *= self.sensitivity;

                self.yaw += x_offset;
                self.pitch += y_offset;

                self.clamp_angles();

                self.update_direction_from_angles();
            }
            CameraState::Free => {
                let mut x_offset = dx as f64;
                let mut y_offset = -dy as f64; // invert y

                x_offset *= self.sensitivity;
                y_offset *= self.sensitivity;

                self.yaw += x_offset;
                self.pitch += y_offset;

                self.clamp_angles();

                self.update_direction_from_angles();
            }
        }
    }

    pub fn process_mouse_input_movement(&mut self, _position: PhysicalPosition<f64>) {
        match self.move_state {
            CameraState::Free => {}
            CameraState::Third => {}
            CameraState::Locked | CameraState::Gallery => {}
        }
    }

    fn handle_mouse_delta_free(&mut self, xpos: f64, ypos: f64) {
        if self.first_mousing {
            self.last_x = xpos;
            self.last_y = ypos;
            self.first_mousing = false;
            return;
        }

        let mut x_offset = xpos - self.last_x;
        let mut y_offset = self.last_y - ypos; // invert y

        self.last_x = xpos;
        self.last_y = ypos;

        x_offset *= self.sensitivity;
        y_offset *= self.sensitivity;

        self.yaw += x_offset;
        self.pitch += y_offset;

        self.clamp_angles();

        self.update_direction_from_angles();
    }

    fn handle_mouse_delta_third(&mut self, xpos: f64, ypos: f64) {
        if self.first_mousing {
            self.last_x = xpos;
            self.last_y = ypos;
            self.first_mousing = false;
            return;
        }

        let mut x_offset = xpos - self.last_x;
        let mut y_offset = self.last_y - ypos;

        self.last_x = xpos;
        self.last_y = ypos;

        x_offset *= self.sensitivity;
        y_offset *= self.sensitivity;

        self.yaw += x_offset;
        self.pitch -= y_offset; // note: flipped sign vs Free

        self.clamp_angles();

        self.update_direction_from_angles();
    }

    fn clamp_angles(&mut self) {
        if self.yaw >= 360.0 {
            self.yaw -= 360.0;
        } else if self.yaw < 0.0 {
            self.yaw += 360.0;
        }

        self.pitch = self.pitch.clamp(-89.0, 89.0);
    }

    fn update_direction_from_angles(&mut self) {
        let yaw_rad = self.yaw.to_radians();
        let pitch_rad = self.pitch.to_radians();

        self.direction.x = (yaw_rad.cos() * pitch_rad.cos()) as f32;
        self.direction.y = pitch_rad.sin() as f32;
        self.direction.z = (yaw_rad.sin() * pitch_rad.cos()) as f32;
        self.direction = self.direction.normalize();
    }

    pub fn process_key_event(&mut self, delta: f32, input: &InputState) {
        use KeyCode::*;

        if self.move_state == CameraState::Free {
            if input.is_down(KeyW) {
                self.position += (self.movement_speed * self.forward) * delta;
            }
            if input.is_down(KeyS) {
                self.position -= (self.movement_speed * self.forward) * delta;
            }
            if input.is_down(KeyA) {
                self.position +=
                    (self.up.cross(self.forward).normalize() * self.movement_speed) * delta;
            }
            if input.is_down(KeyD) {
                self.position -=
                    (self.up.cross(self.forward).normalize() * self.movement_speed) * delta;
            }
        }

        if input.is_down(KeyU) {
            self.fovy = 5.0_f32.to_radians();
        } else {
            self.fovy = 45.0_f32.to_radians();
        }

        if input.just_pressed(KeyL) {
            self.locked_target = self.target;
            self.locked_position = self.position;
        }
    }
}
