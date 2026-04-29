#![allow(clippy::useless_vec)]
use core::f32;
use std::collections::HashMap;

use glam::Vec3;

use crate::{
    animation::{animation::Animation, skellington::Bone},
    enums_types::AnimationType,
    util::data_structure::HashMapGetPairMut,
};

#[derive(Debug, Clone)]
pub struct Animator {
    pub current_animation: AnimationType,
    pub next_animation: AnimationType,
    pub animations: HashMap<AnimationType, Animation>,
    pub blend_factor: f32,
    pub blend_time: f32,
    pub root_motion_state: RootMotionState,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            current_animation: AnimationType::Idle,
            next_animation: AnimationType::Idle,
            animations: HashMap::new(),
            blend_factor: 0.0,
            blend_time: 0.14,
            root_motion_state: RootMotionState {
                root_bone: "".to_string(),
                last_root_pos: None,
                frame_root_delta: Vec3::ZERO,
                active_source: None,
            },
        }
    }

    pub fn get_current_animation(&self) -> Option<&Animation> {
        self.animations.get(&self.current_animation)
    }

    pub fn get_next_animation(&self) -> Option<&Animation> {
        self.animations.get(&self.next_animation)
    }

    pub fn set_current_animation(&mut self, input: AnimationType) {
        if self.current_animation == input && self.next_animation == input {
            return;
        }

        self.current_animation = input.clone();
        self.next_animation = input;
        self.blend_factor = 0.0;
        self.root_motion_state.use_source(None);
    }

    pub fn set_next_animation(&mut self, input: AnimationType) {
        if !self.animations.contains_key(&input) {
            println!(
                "WARNING: passed in an animation that wasn't found: {}",
                &input
            );

            self.next_animation = self.current_animation.clone();
            self.blend_factor = 0.0;
            self.root_motion_state.use_source(None);
            return;
        }

        if self.next_animation == input {
            return;
        }

        self.next_animation = input.clone();
        self.blend_factor = 0.0;

        // New target clip means any previous root-bone position belongs to
        // a different animation source and must not be compared.
        self.root_motion_state.use_source(None);

        if let Some(anim) = self.animations.get_mut(&input) {
            if anim.reset_on_change {
                anim.current_time = 0.0;
            }
        }
    }

    pub fn update(&mut self, skellington: &mut Bone, dt: f32) {
        self.root_motion_state.begin_frame();

        if self.current_animation == AnimationType::Death {
            if let Some(anim) = self.animations.get(&AnimationType::Death) {
                if anim.current_time >= anim.duration {
                    return;
                }
            }
        }

        if self.current_animation != self.next_animation {
            self.blend_factor += dt / self.blend_time.max(0.0001);

            if self.blend_factor >= 1.0 {
                self.finish_blend();
            }
        }

        let curr_key = self.current_animation.clone();
        let next_key = self.next_animation.clone();

        if curr_key != next_key {
            if let Some((current, next)) = self.animations.get_pair_mut(&curr_key, &next_key) {
                current.update_blended(
                    skellington,
                    next,
                    self.blend_factor,
                    dt,
                    curr_key,
                    next_key,
                    &mut self.root_motion_state,
                );
            }
        } else if let Some(current) = self.animations.get_mut(&curr_key) {
            current.update_single(skellington, dt, curr_key, &mut self.root_motion_state);
        }
    }

    fn finish_blend(&mut self) {
        self.blend_factor = 0.0;
        self.current_animation = self.next_animation.clone();

        let source = self
            .animations
            .get(&self.current_animation)
            .and_then(|anim| {
                if anim.do_root_motion {
                    Some(self.current_animation.clone())
                } else {
                    None
                }
            });

        // If the incoming animation was already the active root-motion source
        // during the blend, this preserves tracking. If not, it resets safely.
        self.root_motion_state.use_source(source);
    }
}

#[derive(Debug, Clone)]
pub struct RootMotionState {
    pub root_bone: String,
    pub last_root_pos: Option<Vec3>,
    pub frame_root_delta: Vec3,
    pub active_source: Option<AnimationType>,
}

impl RootMotionState {
    pub fn begin_frame(&mut self) {
        self.frame_root_delta = Vec3::ZERO;
    }

    pub fn reset_tracking(&mut self) {
        self.last_root_pos = None;
        self.frame_root_delta = Vec3::ZERO;
    }

    pub fn use_source(&mut self, source: Option<AnimationType>) {
        if self.active_source != source {
            self.active_source = source;
            self.reset_tracking();
        }
    }

    pub fn sample_root(&mut self, authored_root_pos: Vec3) {
        if let Some(last) = self.last_root_pos {
            let mut delta = authored_root_pos - last;
            delta.y = 0.0;
            self.frame_root_delta += delta;
        }

        self.last_root_pos = Some(authored_root_pos);
    }
}
