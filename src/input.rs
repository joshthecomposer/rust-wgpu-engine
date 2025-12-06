use std::collections::HashSet;

use glam::{vec2, vec3, vec4, Mat4, Vec2, Vec3, Vec3Swizzles, Vec4Swizzles};

use rapier3d::{data::Index, prelude::*};
use winit::{event::{ElementState, MouseButton}, keyboard::KeyCode};

use crate::{camera::{self, Camera}, entity_manager::EntityManager, enums_types::{AnimationType, CameraState, Faction}, physics::PhysicsState, some_data::GROUP_TERRAIN};

pub struct InputState {
    pub keys_current: HashSet<KeyCode>,           // Held this frame
    pub keys_previous: HashSet<KeyCode>,          // held last frame

    pub mouse_current: HashSet<MouseButton>,  // Held this frame
    pub mouse_previous: HashSet<MouseButton>, // held last frame

    pub mouse_pos_current: Vec2,

    pub ray_just_hit: bool,
    pub ray_pos: Vec3,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_current: HashSet::new(),
            keys_previous: HashSet::new(),

            mouse_current: HashSet::new(),
            mouse_previous: HashSet::new(),

            mouse_pos_current: Vec2::splat(0.0),

            ray_just_hit: false,
            ray_pos: Vec3::splat(0.0),
        }
    }

    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.keys_current.contains(&key) && !self.keys_previous.contains(&key)
    }

    pub fn just_released(&self, key: KeyCode) -> bool {
        !self.keys_current.contains(&key) && self.keys_previous.contains(&key)
    }

    pub fn space_just_pressed(&self) -> bool {
        self.keys_current.contains(&KeyCode::Space) && !self.keys_previous.contains(&KeyCode::Space)
    }
    
    pub fn is_down(&self, key: KeyCode) -> bool {
        self.keys_current.contains(&key)
    }

    pub fn wasd_is_down(&self) -> bool {
        self.keys_current.contains(&KeyCode::KeyW)
       || self.keys_current.contains(&KeyCode::KeyS)
       || self.keys_current.contains(&KeyCode::KeyA)
       || self.keys_current.contains(&KeyCode::KeyD)
    }
    
    pub fn shift_is_down(&self) -> bool {
        self.keys_current.contains(&KeyCode::ShiftLeft)
    }

    pub fn mouse_just_pressed(&self, b: MouseButton) -> bool {
        self.mouse_current.contains(&b) && !self.mouse_previous.contains(&b)
    }

    pub fn mouse_just_released(&self, b: MouseButton) -> bool {
        !self.mouse_current.contains(&b) && self.mouse_previous.contains(&b)
    }

    pub fn left_mouse_just_pressed(&self) -> bool {
        self.mouse_current.contains(&MouseButton::Left) && !self.mouse_previous.contains(&MouseButton::Left)
    }

    pub fn left_mouse_just_released(&self) -> bool {
        !self.mouse_current.contains(&MouseButton::Left) && self.mouse_previous.contains(&MouseButton::Left)
    }

    pub fn right_mouse_just_pressed(&self) -> bool {
        self.mouse_current.contains(&MouseButton::Right) && !self.mouse_previous.contains(&MouseButton::Right)
    }

    pub fn right_mouse_just_released(&self) -> bool {
        !self.mouse_current.contains(&MouseButton::Right) && self.mouse_previous.contains(&MouseButton::Right)
    }

    pub fn mouse_is_down(&self, b: MouseButton) -> bool {
        self.mouse_current.contains(&b)
    }

    pub fn right_mouse_is_down(&self) -> bool {
        self.mouse_current.contains(&MouseButton::Right)
    }


    pub fn update(&mut self) {
        self.keys_previous = self.keys_current.clone();
        self.mouse_previous = self.mouse_current.clone();
    }
}

pub fn handle_keyboard_input(key: KeyCode, action: ElementState, input_state: &mut InputState) {
    match action {
        ElementState::Pressed => { input_state.keys_current.insert(key); }
        ElementState::Released => { input_state.keys_current.remove(&key); }
        _=> ()
    }
}

pub fn handle_mouse_motion() {
}

pub fn handle_mouse_input(button: MouseButton, action: ElementState, screen_size: Vec2, camera: &Camera, em: &mut EntityManager, input_state: &mut InputState, physics: &mut PhysicsState) {
    let cursor_pos = input_state.mouse_pos_current;
    let pressed_keys = &input_state.keys_current;
    match action {
        ElementState::Pressed => { 
            input_state.mouse_current.insert(button);
            if button == MouseButton::Left {

                if !pressed_keys.contains(&KeyCode::ShiftLeft) {
                    em.empty_selected_and_reset_bodies(physics);
                }

                if camera.move_state != CameraState::Locked {
                    em.empty_selected_and_reset_bodies(physics);
                    return;
                }

                let (ray_origin, ray_dir) = mouse_ray_from_screen(cursor_pos, screen_size, camera);

                let ray = Ray::new(point![ray_origin.x, ray_origin.y, ray_origin.z], vector![ray_dir.x, ray_dir.y, ray_dir.z]);
                let query_pipeline = &physics.query_pipeline.as_ref().unwrap();
                let colliders = &physics.collider_set;
                let bodies = &physics.rigid_body_set;

                let max_toi = 1000.0;
                let solid = true;

                if let Some((handle, toi)) = query_pipeline.cast_ray(
                    bodies,
                    colliders,
                    &ray,
                    max_toi,
                    solid,
                    InteractionGroups::all().into(),
                ) {
                    if let Some(&entity_id) = em.collider_to_entity.get(&handle) {
                        em.selected.push(entity_id);
                        let ph = em.physics_handles.get_mut(entity_id).unwrap();
                        let rb = physics.rigid_body_set.get_mut(ph.rigid_body).unwrap();
                        rb.set_body_type(RigidBodyType::KinematicPositionBased, false);
                        return
                    }

                    let collider = physics.collider_set.get(handle).unwrap();
                    let groups = collider.collision_groups();

                    if groups.memberships & GROUP_TERRAIN.into() != 0.into() {
                        if !input_state.ray_just_hit {
                            let hit_point = ray.point_at(toi);
                            input_state.ray_pos = hit_point.into();
                            println!("HIT THE TERRAIN AT: {:?}", hit_point);
                            input_state.ray_just_hit = true;
                            return;
                        }
                    }
                }
            }
        },
        ElementState::Released => { input_state.mouse_current.remove(&button); },
        _ => ()
   }
}

pub fn mouse_ray_from_screen(
    mouse_pos: Vec2,
    screen_size: Vec2,
    camera: &Camera,
) -> (Vec3, Vec3) {
    let (mouse_x, mouse_y) = (mouse_pos.x, mouse_pos.y);
    let (screen_w, screen_h) = (screen_size.x, screen_size.y);
    
    // Calculate NDC
    // transform x to match opengl left-to-right convention
    let x = (2.0 * mouse_x) / screen_w - 1.0;
    // invert y. Screen space Y is top-down whereas opengl is bottom-up
    let y = 1.0 - (2.0 * mouse_y) / screen_h;
    // the ray goes INTO the screen (negative z)
    let z = -1.0;
    let ray_ndc = vec4(x, y, z, 1.0);

    // we want to reverse the transform pipeline. clip -> view -> world
    let inv_proj = camera.projection.inverse();
    let inv_view = camera.view.inverse();

    let ray_eye = inv_proj * ray_ndc;
    let ray_eye = vec4(ray_eye.x, ray_eye.y, -1.0, 0.0);

    let ray_world = (inv_view * ray_eye).xyz().normalize();
    let camera_pos = camera.position;

    (camera_pos, ray_world)
}

fn ray_hits_cylinder(
    ray_origin: Vec3,
    ray_dir: Vec3,
    cyl_base: Vec3,
    height: f32,
    radius: f32,
) -> Option<f32> {
    // Project onto XZ plane
    let d = vec2(ray_dir.x, ray_dir.z);
    let o = vec2(ray_origin.x - cyl_base.x, ray_origin.z - cyl_base.z);

    let a = d.dot(d);
    let b = 2.0 * o.dot(d);
    let c = o.dot(o) - radius * radius;

    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return None;
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    for &t in &[t1, t2] {
        if t < 0.0 { continue; }

        let y = ray_origin.y + t * ray_dir.y;
        if y >= cyl_base.y && y <= cyl_base.y + height {
            return Some(t);
        }
    }

    None
}
