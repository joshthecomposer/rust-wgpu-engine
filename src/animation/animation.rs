#![allow(clippy::useless_vec)]
use core::f32;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use rapier3d::prelude::Cuboid;
use std::{
    any::Any,
    collections::HashMap,
    ffi::c_void,
    mem::{self, offset_of},
    path::Path,
    ptr,
    str::Lines,
};

use crate::{
    animation::skellington::{Bone, BoneJoinInfo, BoneTransformTrack},
    enums_types::{AnimationType, FrameActivation, TextureProfile, TextureType, ANIMATION_EPSILON},
    gl_call,
    shaders::Shader,
    sound::sound_manager::{ContinuousSound, OneShot},
};

#[derive(Debug, Clone)]
pub struct Animation {
    pub duration: f32,
    pub ticks_per_second: f32,
    pub model_animation_join: Vec<BoneJoinInfo>,
    pub bone_transforms: HashMap<String, BoneTransformTrack>,
    pub current_pose: Vec<Mat4>,

    pub current_segment: std::cell::Cell<u32>,
    pub one_shots: Vec<OneShot>,
    pub continuous_sounds: Vec<ContinuousSound>,
    pub hurtbox_activation: Option<FrameActivation>,
    pub hold_frame: Option<u32>,
    pub do_hold: bool,

    pub current_time: f32,
    pub looping: bool,
}

impl Animation {
    pub fn default() -> Self {
        Self {
            duration: 0.0,
            ticks_per_second: 0.0,
            model_animation_join: vec![],
            bone_transforms: HashMap::new(),
            current_pose: vec![],

            current_segment: std::cell::Cell::new(0),
            one_shots: vec![],
            continuous_sounds: vec![],
            hurtbox_activation: None,
            hold_frame: None,
            do_hold: false,

            current_time: 0.0,
            looping: true,
        }
    }

    pub fn calculate_pose(
        &mut self,
        skeleton: &mut Bone,
        parent_transform: Mat4,
        global_inverse_transform: Mat4,
    ) {
        let delta = self.current_time % self.duration;
        let (local_position, local_rot, local_scale) =
            self.get_bone_local_transform(skeleton, delta);
        let local_transform =
            Mat4::from_scale_rotation_translation(local_scale, local_rot, local_position);
        let global_transform = parent_transform * local_transform;

        self.current_pose[skeleton.id as usize] = match 0 {
            0 => global_inverse_transform * global_transform * skeleton.offset,
            1 => global_transform * skeleton.offset * global_inverse_transform,
            2 => global_transform * skeleton.offset,
            3 => global_inverse_transform * global_transform,
            4 => global_inverse_transform * skeleton.offset * global_transform,
            5 => global_transform * global_inverse_transform * skeleton.offset,
            10 => Mat4::IDENTITY,
            _ => global_inverse_transform * global_transform * skeleton.offset,
        };

        for child in skeleton.children.iter_mut() {
            self.calculate_pose(child, global_transform, global_inverse_transform);
        }
    }

    // The idea is that at blend factor 0.0 we are at the self/current animation.
    // At blend factor 1.0 we are fully at the "other" animation.
    // At that point the current animation should be switched to the "other"
    pub fn calculate_pose_blended(
        &mut self,
        skeleton: &mut Bone,
        parent_transform: Mat4,
        global_inverse_transform: Mat4,
        other_animation: &mut Animation,
        blend_factor: f32,
    ) {
        let delta1 = self.current_time % self.duration;
        let delta2 = other_animation.current_time % other_animation.duration;

        let (pos1, rot1, scale1) = self.get_bone_local_transform(skeleton, delta1);
        let (pos2, rot2, scale2) = other_animation.get_bone_local_transform(skeleton, delta2);

        let final_pos = pos1.lerp(pos2, blend_factor);
        let final_rot = rot1.slerp(rot2, blend_factor);
        let final_scale = scale1.lerp(scale2, blend_factor);

        let local_transform =
            Mat4::from_scale_rotation_translation(final_scale, final_rot, final_pos);
        let global_transform = parent_transform * local_transform;

        self.current_pose[skeleton.id as usize] = match 0 {
            0 => global_inverse_transform * global_transform * skeleton.offset,
            1 => global_transform * skeleton.offset * global_inverse_transform,
            2 => global_transform * skeleton.offset,
            3 => global_inverse_transform * global_transform,
            4 => global_inverse_transform * skeleton.offset * global_transform,
            5 => global_transform * global_inverse_transform * skeleton.offset,
            10 => Mat4::IDENTITY,
            _ => global_inverse_transform * global_transform * skeleton.offset,
        };

        for child in skeleton.children.iter_mut() {
            self.calculate_pose_blended(
                child,
                global_transform,
                global_inverse_transform,
                other_animation,
                blend_factor,
            );
        }
    }

    fn get_bone_local_transform(&self, skeleton: &Bone, delta: f32) -> (Vec3, Quat, Vec3) {
        let btt = match self.bone_transforms.get(&skeleton.name) {
            Some(name) => name,
            _ => {
                dbg!(skeleton);
                panic!("skeleton name not found")
            }
        };

        let (segment, fraction) = get_time_fraction(&btt.position_timestamps, delta);

        self.current_segment.set(segment);

        if segment == 0 {
            // Use the first keyframe
            let position = btt.positions[0];
            let rotation = btt.rotations[0];
            let scale = btt.scales[0];

            // Mat4::from_scale_rotation_translation(scale, rotation, position)
            (position, rotation, scale)
        } else {
            // Get the two keyframes to interpolate between
            let prev_idx = segment - 1;
            let next_idx = segment.min(btt.positions.len() as u32 - 1); // Prevent out-of-bounds

            let prev_position = btt.positions[prev_idx as usize];
            let next_position = btt.positions[next_idx as usize];

            let prev_rotation = btt.rotations[prev_idx as usize];
            let next_rotation = btt.rotations[next_idx as usize];

            let prev_scale = btt.scales[prev_idx as usize];
            let next_scale = btt.scales[next_idx as usize];

            // Perform linear interpolation for position and scale
            let interpolated_position = prev_position.lerp(next_position, fraction);
            let interpolated_scale = prev_scale.lerp(next_scale, fraction);

            // Perform spherical interpolation (slerp) for rotation (bone rotation)
            let interpolated_rotation = prev_rotation.slerp(next_rotation, fraction);
            // Mat4::from_scale_rotation_translation(interpolated_scale, interpolated_rotation, interpolated_position)

            (
                interpolated_position,
                interpolated_rotation,
                interpolated_scale,
            )
        }
    }

    pub fn get_raw_global_bone_transform_by_name(
        &self,
        bone_name: &str,
        skeleton: &Bone,
        parent_transform: Mat4,
    ) -> Option<Mat4> {
        let delta = self.current_time % self.duration;
        if skeleton.name == bone_name {
            let (pos, rot, scale) = self.get_bone_local_transform(skeleton, delta);
            let local = Mat4::from_scale_rotation_translation(scale, rot, pos);
            return Some(parent_transform * local);
        }

        for child in &skeleton.children {
            let (pos, rot, scale) = self.get_bone_local_transform(skeleton, delta);
            let local = Mat4::from_scale_rotation_translation(scale, rot, pos);
            let next_parent = parent_transform * local;

            if let Some(found) =
                self.get_raw_global_bone_transform_by_name(bone_name, child, next_parent)
            {
                return Some(found);
            }
        }

        None
    }

    pub fn get_raw_global_bone_transform_by_name_blended(
        &self,
        bone_name: &str,
        skeleton: &Bone,
        parent_transform: Mat4,
        other_animation: &Animation,
        blend_factor: f32,
    ) -> Option<Mat4> {
        let delta1 = self.current_time % self.duration;
        let delta2 = other_animation.current_time % other_animation.duration;

        if skeleton.name == bone_name {
            let (pos1, rot1, scale1) = self.get_bone_local_transform(skeleton, delta1);
            let (pos2, rot2, scale2) = other_animation.get_bone_local_transform(skeleton, delta2);

            let final_pos = pos1.lerp(pos2, blend_factor);
            let final_rot = rot1.slerp(rot2, blend_factor);
            let final_scale = scale1.lerp(scale2, blend_factor);

            let local = Mat4::from_scale_rotation_translation(final_scale, final_rot, final_pos);
            return Some(parent_transform * local);
        }

        for child in &skeleton.children {
            let (pos1, rot1, scale1) = self.get_bone_local_transform(skeleton, delta1);
            let (pos2, rot2, scale2) = other_animation.get_bone_local_transform(skeleton, delta2);

            let final_pos = pos1.lerp(pos2, blend_factor);
            let final_rot = rot1.slerp(rot2, blend_factor);
            let final_scale = scale1.lerp(scale2, blend_factor);

            let local = Mat4::from_scale_rotation_translation(final_scale, final_rot, final_pos);
            let next_parent = parent_transform * local;

            if let Some(found) = self.get_raw_global_bone_transform_by_name_blended(
                bone_name,
                child,
                next_parent,
                other_animation,
                blend_factor,
            ) {
                return Some(found);
            }
        }

        None
    }

    pub fn update(
        &mut self,
        skellington: &mut Bone,
        other_animation: Option<&mut Animation>,
        blend_factor: f32,
        dt: f32,
    ) {
        if let Some(hold_frame) = self.hold_frame {
            if self.current_segment.get() == hold_frame && self.do_hold {
                return;
            }
        }

        self.current_time += dt;
        if self.current_time > self.duration {
            if self.looping {
                self.current_time = 0.0;
            } else {
                // self.current_time = self.duration;
                self.current_time = self.duration - ANIMATION_EPSILON;
            }
        }

        if let Some(other_animation) = other_animation {
            self.calculate_pose_blended(
                skellington,
                Mat4::IDENTITY,
                Mat4::IDENTITY,
                other_animation,
                blend_factor,
            );

            other_animation.current_time += dt;
            if other_animation.current_time > other_animation.duration {
                other_animation.current_time = 0.0;
            }
        } else {
            self.calculate_pose(skellington, Mat4::IDENTITY, Mat4::IDENTITY);
        }
    }
}

pub fn get_time_fraction(times: &[f32], dt: f32) -> (u32, f32) {
    let mut segment = 0;

    while dt > times[segment] {
        segment += 1;
    }

    if segment == 0 {
        return (0, 0.0); // avoid accessing times[-1]... maybe this isn't the best
    }

    let start = times[segment - 1];
    let end = times[segment];
    let frac = (dt - start) / (end - start);

    (segment as u32, frac)
}
