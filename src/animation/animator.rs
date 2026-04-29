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
        self.current_animation = input;
    }

    pub fn set_next_animation(&mut self, input: AnimationType) {
        if !self.animations.contains_key(&input) {
            println!(
                "WARNING: passed in an animation that wasn't found: {}",
                &input
            );
            self.next_animation = self.current_animation.clone();
            self.blend_factor = 0.0;
            self.root_motion_state.reset_tracking();
            return;
        }

        if self.next_animation == input {
            return;
        }

        self.next_animation = input.clone();
        self.blend_factor = 0.0;
        self.root_motion_state.reset_tracking();

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
            self.blend_factor += dt / self.blend_time;
            if self.blend_factor >= 1.0 {
                self.blend_factor = 0.0;
                self.set_current_animation(self.next_animation.clone());
            }
        }

        let curr_key = self.current_animation.clone();
        let next_key = self.next_animation.clone();

        if curr_key != next_key {
            if let Some((current, next)) = self.animations.get_pair_mut(&curr_key, &next_key) {
                current.update(
                    skellington,
                    Some(next),
                    self.blend_factor,
                    dt,
                    &mut self.root_motion_state,
                );
            }
        } else if let Some(current) = self.animations.get_mut(&curr_key) {
            current.update(
                skellington,
                None,
                self.blend_factor,
                dt,
                &mut self.root_motion_state,
            );
        }
    }
}

#[derive(Debug, Clone)]
pub struct RootMotionState {
    pub root_bone: String,
    pub last_root_pos: Option<Vec3>,
    pub frame_root_delta: Vec3,
}

impl RootMotionState {
    pub fn begin_frame(&mut self) {
        self.frame_root_delta = Vec3::ZERO;
    }

    pub fn reset_tracking(&mut self) {
        self.last_root_pos = None;
        self.frame_root_delta = Vec3::ZERO;
    }
}
