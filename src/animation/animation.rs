#![allow(clippy::useless_vec)]
use core::f32;
use glam::{Mat4, Quat, Vec3};

use crate::{
    animation::{
        animator::RootMotionState,
        skellington::{Bone, BoneJoinInfo, BoneTransformTrack},
    },
    enums_types::{AnimationType, FrameActivation, ANIMATION_EPSILON},
    sound::sound_manager::{ContinuousSound, OneShot},
};

#[derive(Debug, Clone)]
pub struct Animation {
    pub duration: f32,
    pub ticks_per_second: f32,
    pub model_animation_join: Vec<BoneJoinInfo>,
    pub bone_transforms: Vec<BoneTransformTrack>,
    pub current_pose: Vec<Mat4>,

    pub current_segment: std::cell::Cell<u32>,
    pub one_shots: Vec<OneShot>,
    pub continuous_sounds: Vec<ContinuousSound>,
    pub hurtbox_activation: Option<FrameActivation>,
    pub hold_frame: Option<u32>,
    pub do_hold: bool,

    pub current_time: f32,
    pub looping: bool,

    pub lod_skip: u32,
    pub lod_counter: u32,
    pub interrupt_frame: Option<u32>,
    pub reset_on_change: bool,
    pub do_root_motion: bool,
}

impl Animation {
    pub fn default() -> Self {
        Self {
            duration: 0.0,
            ticks_per_second: 0.0,
            model_animation_join: vec![],
            bone_transforms: vec![],
            current_pose: vec![],

            current_segment: std::cell::Cell::new(0),
            one_shots: vec![],
            continuous_sounds: vec![],
            hurtbox_activation: None,
            hold_frame: None,
            do_hold: false,

            current_time: 0.0,
            looping: true,

            lod_skip: 0,
            lod_counter: 0,
            interrupt_frame: None,
            reset_on_change: true,
            do_root_motion: false,
        }
    }

    pub fn can_interrupt(&self) -> bool {
        if let Some(f) = self.interrupt_frame {
            self.current_segment.get() >= f
        } else {
            true
        }
    }

    pub fn calculate_pose(
        &mut self,
        skeleton: &mut Bone,
        parent_transform: Mat4,
        global_inverse_transform: Mat4,
        root_motion_source: Option<AnimationType>,
        root_motion_state: &mut RootMotionState,
    ) {
        let delta = self.current_time % self.duration;
        let (mut local_position, local_rot, local_scale) =
            self.get_bone_local_transform(skeleton, delta);

        if skeleton.name == root_motion_state.root_bone {
            root_motion_state.use_source(root_motion_source.clone());

            if root_motion_source.is_some() {
                root_motion_state.sample_root(local_position);
                local_position.x = 0.0;
                local_position.z = 0.0;
            }
        }

        let local_transform =
            Mat4::from_scale_rotation_translation(local_scale, local_rot, local_position);
        let global_transform = parent_transform * local_transform;

        skeleton.global_transform = global_transform;

        self.current_pose[skeleton.id as usize] =
            global_inverse_transform * global_transform * skeleton.offset;

        for child in skeleton.children.iter_mut() {
            self.calculate_pose(
                child,
                global_transform,
                global_inverse_transform,
                root_motion_source.clone(),
                root_motion_state,
            );
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
        current_key: AnimationType,
        next_key: AnimationType,
        root_motion_state: &mut RootMotionState,
    ) {
        let delta1 = self.current_time % self.duration;
        let delta2 = other_animation.current_time % other_animation.duration;

        let (pos1, rot1, scale1) = self.get_bone_local_transform(skeleton, delta1);
        let (pos2, rot2, scale2) = other_animation.get_bone_local_transform(skeleton, delta2);

        let mut final_pos = pos1.lerp(pos2, blend_factor);
        let final_rot = rot1.slerp(rot2, blend_factor);
        let final_scale = scale1.lerp(scale2, blend_factor);

        if skeleton.name == root_motion_state.root_bone {
            let source = if other_animation.do_root_motion {
                Some(next_key.clone())
            } else if self.do_root_motion {
                Some(current_key.clone())
            } else {
                None
            };

            root_motion_state.use_source(source.clone());

            match source {
                Some(src) if src == next_key => {
                    root_motion_state.sample_root(pos2);
                }
                Some(src) if src == current_key => {
                    root_motion_state.sample_root(pos1);
                }
                _ => {}
            }

            // Important: even though root motion is sampled from one source clip,
            // the rendered blended root position should still have planar motion removed.
            final_pos.x = 0.0;
            final_pos.z = 0.0;
        }

        let local_transform =
            Mat4::from_scale_rotation_translation(final_scale, final_rot, final_pos);

        let global_transform = parent_transform * local_transform;

        skeleton.global_transform = global_transform;

        self.current_pose[skeleton.id as usize] =
            global_inverse_transform * global_transform * skeleton.offset;

        for child in skeleton.children.iter_mut() {
            self.calculate_pose_blended(
                child,
                global_transform,
                global_inverse_transform,
                other_animation,
                blend_factor,
                current_key.clone(),
                next_key.clone(),
                root_motion_state,
            );
        }
    }

    fn get_bone_local_transform(&self, skeleton: &Bone, delta: f32) -> (Vec3, Quat, Vec3) {
        let btt = match self.bone_transforms.get(skeleton.id) {
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

    pub fn update_single(
        &mut self,
        skellington: &mut Bone,
        dt: f32,
        current_key: AnimationType,
        root_motion_state: &mut RootMotionState,
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
                root_motion_state.reset_tracking();
            } else {
                self.current_time = self.duration - ANIMATION_EPSILON;
            }
        }

        let skip = self.lod_skip;

        if skip > 0 {
            self.lod_counter = self.lod_counter.wrapping_add(1);

            if (self.lod_counter % (skip + 1)) != 0 {
                return;
            }
        }

        let root_motion_source = if self.do_root_motion {
            Some(current_key)
        } else {
            None
        };

        self.calculate_pose(
            skellington,
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            root_motion_source,
            root_motion_state,
        );
    }

    pub fn update_blended(
        &mut self,
        skellington: &mut Bone,
        other_animation: &mut Animation,
        blend_factor: f32,
        dt: f32,
        current_key: AnimationType,
        next_key: AnimationType,
        root_motion_state: &mut RootMotionState,
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
                self.current_time = self.duration - ANIMATION_EPSILON;
            }
        }

        other_animation.current_time += dt;

        if other_animation.current_time > other_animation.duration {
            if other_animation.looping {
                other_animation.current_time = 0.0;
            } else {
                other_animation.current_time = other_animation.duration - ANIMATION_EPSILON;
            }
        }

        let skip = self.lod_skip;

        if skip > 0 {
            self.lod_counter = self.lod_counter.wrapping_add(1);

            if (self.lod_counter % (skip + 1)) != 0 {
                return;
            }
        }

        self.calculate_pose_blended(
            skellington,
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            other_animation,
            blend_factor,
            current_key,
            next_key,
            root_motion_state,
        );
    }
}

pub fn get_time_fraction(times: &[f32], dt: f32) -> (u32, f32) {
    let mut segment = 0;

    while segment + 1 < times.len() && dt > times[segment] {
        segment += 1;
    }
    segment = segment.min(times.len() - 1);

    if segment == 0 {
        return (0, 0.0); // avoid accessing times[-1]... maybe this isn't the best
    }

    let start = times[segment - 1];
    let end = times[segment];
    let frac = (dt - start) / (end - start);

    (segment as u32, frac)
}

//pub fn get_time_fraction(times: &[f32], dt: f32) -> (u32, f32) {
//    let mut segment = 0;
//
//    while dt > times[segment] {
//        segment += 1;
//    }
//
//    if segment == 0 {
//        return (0, 0.0); // avoid accessing times[-1]... maybe this isn't the best
//    }
//
//    let start = times[segment - 1];
//    let end = times[segment];
//    let frac = (dt - start) / (end - start);
//
//    (segment as u32, frac)
//}
