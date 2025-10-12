#![allow(dead_code, clippy::single_match)]
use glam::{vec3, Mat4, Vec3};
use glfw::{Action, Key, PWindow, WindowEvent};

use crate::{entity_manager::EntityManager, enums_types::{CameraState, Faction, PlayerState}, input::InputState, physics::PhysicsState, renderer};

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
            z_far: 10000.0,

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
        }
    }

    pub fn update(&mut self, _em: &EntityManager, dt: f32, ps: &PhysicsState, alpha: f32, input: &InputState) {
        match self.move_state {
            CameraState::Free => {
                self.forward = self.direction.normalize();
            }
            CameraState::Third => {
                if let Some(player_key) = _em.factions.iter().find(|e| e.value() == &Faction::Player) {

                    let player_transform = _em.transforms.get(player_key.key()).unwrap();

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
            CameraState::Locked => {
                self.target = self.locked_target;
                self.position = self.locked_position;
                self.forward = Vec3::normalize(self.target - self.position);
            }
        }

        self.right = self.forward.cross(vec3(0.0, 1.0, 0.0)).normalize();
        self.up = self.right.cross(self.forward).normalize();

        self.process_key_event(dt, input);
    }

    pub fn get_view_matrix(&mut self) {
        self.view = Mat4::look_at_rh(self.position, self.target, self.up);
    }

    pub fn reset_matrices(&mut self, aspect: f32) {
        self.projection = Mat4::IDENTITY;
        self.projection = Mat4::perspective_rh_gl(self.fovy, aspect, self.z_near, self.z_far);
        
        self.view = Mat4::IDENTITY;
        self.target = self.position + self.forward;

        self.view = Mat4::look_at_rh(self.position, self.target, self.up);
    }

    pub fn sync_mouse_position(&mut self, window: &PWindow) {
        let (x, y) = window.get_cursor_pos();
        self.last_x = x;
        self.last_y = y;
        self.first_mousing = true;
    }

    pub fn process_mouse_input(&mut self, window: &PWindow, event: &WindowEvent) {
        if self.move_state == CameraState::Free {
            match event {
                // Pitch yaw stuff
                glfw::WindowEvent::CursorPos(xpos, ypos) => {
                    if self.first_mousing {
                        self.last_x = *xpos;
                        self.last_y = *ypos;
                        self.first_mousing = false;
                        return;
                    }

                    let mut x_offset = xpos - self.last_x;
                    let mut y_offset = self.last_y - ypos;

                    self.last_x = *xpos;
                    self.last_y = *ypos;

                    x_offset *= self.sensitivity; 
                    y_offset *= self.sensitivity;
                    self.yaw += x_offset;
                    self.pitch += y_offset;

                    if self.yaw >= 360.0 { 
                        self.yaw -= 360.0;
                    } else if self.yaw < 0.0 {
                        self.yaw += 360.0;
                    }
                    
                    self.pitch = self.pitch.clamp(-89.0, 89.0);

                    self.direction.x = (self.yaw.to_radians().cos() * self.pitch.to_radians().cos()) as f32;
                    self.direction.y = self.pitch.to_radians().sin() as f32;
                    self.direction.z = (self.yaw.to_radians().sin() * self.pitch.to_radians().cos()) as f32;
                    self.direction = self.direction.normalize();

                    self.forward = self.direction;
                },
                _ => {}

            }

            // Zoom

        }

        if self.move_state == CameraState::Third{
            match event {
                // Pitch yaw stuff
                glfw::WindowEvent::CursorPos(xpos, ypos) => {
                    if self.first_mousing {
                        self.last_x = *xpos;
                        self.last_y = *ypos;
                        self.first_mousing = false;
                        return;
                    }

                    let mut x_offset = xpos - self.last_x;
                    let mut y_offset = self.last_y - ypos;

                    self.last_x = *xpos;
                    self.last_y = *ypos;

                    x_offset *= self.sensitivity; 
                    y_offset *= self.sensitivity;
                    self.yaw += x_offset;
                    self.pitch -= y_offset;

                    if self.yaw >= 360.0 { 
                        self.yaw -= 360.0;
                    } else if self.yaw < 0.0 {
                        self.yaw += 360.0;
                    }
                    
                    self.pitch = self.pitch.clamp(-89.0, 89.0);

                    self.direction.x = (self.yaw.to_radians().cos() * self.pitch.to_radians().cos()) as f32;
                    self.direction.y = self.pitch.to_radians().sin() as f32;
                    self.direction.z = (self.yaw.to_radians().sin() * self.pitch.to_radians().cos()) as f32;
                    self.direction = self.direction.normalize();

                    self.forward = self.direction;
                },
                _ => {}

            }

            // Zoom

        }
    }

    pub fn process_key_event(&mut self, delta: f32, input: &InputState) {
        if input.just_pressed(Key::F) {
            match self.move_state {
                CameraState::Free => {
                    self.move_state = CameraState::Third;
                }
                CameraState::Third => {
                    self.move_state = CameraState::Locked;
                }
                CameraState::Locked => {
                    self.move_state = CameraState::Free;
                }

            }
        }

        if self.move_state == CameraState::Free {
            if input.is_down(Key::W) {
                self.position += (self.movement_speed * self.forward) * delta;
            }
            if input.is_down(Key::S) {
                self.position -= (self.movement_speed * self.forward) * delta;
            }
            if input.is_down(Key::A) {
                self.position += ((self.up.cross(self.forward).normalize()) * self.movement_speed) * delta;
            }
            if input.is_down(Key::D) {
                self.position -= ((self.up.cross(self.forward).normalize()) * self.movement_speed) * delta;
            }
        }

        if input.is_down(Key::U) {
            self.fovy = 5.0_f32.to_radians();
        } else {
            self.fovy = 45.0_f32.to_radians();
        }
        
        if input.just_pressed(Key::L){
            self.locked_target = self.target;
            self.locked_position  = self.position;
        }
    }
}
