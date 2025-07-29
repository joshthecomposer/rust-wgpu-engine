use std::collections::HashSet;

use glam::{vec2, vec3, vec4, Mat4, Vec2, Vec3, Vec3Swizzles, Vec4Swizzles};
use glfw::MouseButton;

use rapier3d::{data::Index, prelude::*};

use crate::{camera::{self, Camera}, entity_manager::EntityManager, enums_types::{AnimationType, Faction}, physics::PhysicsState};

pub struct InputState {
    pub keys_current: HashSet<glfw::Key>, // Held this frame
    pub keys_previous: HashSet<glfw::Key>, // held last frame
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_current: HashSet::new(),
            keys_previous: HashSet::new(),
        }
    }

    pub fn just_pressed(&self, key: glfw::Key) -> bool {
        self.keys_current.contains(&key) && !self.keys_previous.contains(&key)
    }

    pub fn just_released(&self, key: glfw::Key) -> bool {
        !self.keys_current.contains(&key) && self.keys_previous.contains(&key)
    }
    
    pub fn is_down(&self, key: glfw::Key) -> bool {
        self.keys_current.contains(&key)
    }

    pub fn update(&mut self) {
        self.keys_previous = self.keys_current.clone();
    }
}

pub fn handle_keyboard_input(key: glfw::Key, action: glfw::Action, input_state: &mut InputState) {
    match action {
        glfw::Action::Press => { input_state.keys_current.insert(key); }
        glfw::Action::Release => { input_state.keys_current.remove(&key); }
        _=> ()
    }
}

pub fn handle_mouse_motion() {
}

pub fn handle_mouse_input(button: MouseButton, action: glfw::Action, cursor_pos: Vec2, screen_size: Vec2, camera: &Camera, em: &mut EntityManager, input_state: &InputState, physics: &PhysicsState) {
    let pressed_keys = &input_state.keys_current;
    match action {
        glfw::Action::Press => { 
            if button == glfw::MouseButtonLeft {

                if !pressed_keys.contains(&glfw::Key::LeftShift) {
                    em.selected.clear();


                    let player_id = em.factions.iter().filter(|f| *f.value() == Faction::Player).last().unwrap().key();
                    let animator = em.animators.get_mut(player_id).unwrap();

                    animator.set_next_animation(AnimationType::Slash);
                }

                let (ray_origin, ray_dir) = mouse_ray_from_screen(cursor_pos, screen_size, camera);

                let ray = Ray::new(point![ray_origin.x, ray_origin.y, ray_origin.z], vector![ray_dir.x, ray_dir.y, ray_dir.z]);
                let query_pipeline = &physics.query_pipeline.as_ref().unwrap();
                let colliders = &physics.collider_set;
                let bodies = &physics.rigid_body_set;

                let max_toi = 100.0;
                let solid = true;

                if let Some((handle, _)) = query_pipeline.cast_ray(
                    bodies,
                    colliders,
                    &ray,
                    max_toi,
                    solid,
                    InteractionGroups::all().into(),
                ) {
                    if let Some(&entity_id) = em.collider_to_entity.get(&handle) {
                        em.selected.push(entity_id);
                    }
                }

                
            }
        },
        glfw::Action::Release => (),
        _ => ()
   }
}

fn mouse_ray_from_screen(
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
