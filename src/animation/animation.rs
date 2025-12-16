#![allow(clippy::useless_vec)]
use core::f32;
use glam::{Affine3A, Mat4, Quat, Vec2, Vec3, Vec4};
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
    enums_types::{AnimationType, FrameActivation, TextureProfile, TextureType, ANIMATION_EPSILON},
    gl_call,
    shaders::Shader,
    sound::sound_manager::{ContinuousSound, OneShot},
    util::constants::MAX_BONE_INFLUENCE,
    util::data_structure::HashMapGetPairMut,
};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub base_color: Vec4,

    pub bone_ids: [i32; MAX_BONE_INFLUENCE],
    pub bone_weights: [f32; MAX_BONE_INFLUENCE],
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Self {
            position,
            normal,
            uv: Vec2::new(0.0, 0.0),
            base_color: Vec4::new(1.0, 0.0, 0.0, 1.0),

            bone_ids: [-1; MAX_BONE_INFLUENCE],
            bone_weights: [0.0; MAX_BONE_INFLUENCE],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Texture {
    pub id: u32,
    pub _type: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub vao: u32,
    pub vbo: u32,
    pub ebo: u32,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub textures: [Option<Texture>; 9],

    pub directory: String,
    pub full_path: String,

    pub color_for_texture: bool,
}

impl Model {
    pub fn new() -> Self {
        Self {
            vao: 0,
            vbo: 0,
            ebo: 0,

            vertices: vec![],
            indices: vec![],
            textures: [None, None, None, None, None, None, None, None, None],

            directory: String::new(),
            full_path: String::new(),

            color_for_texture: false,
        }
    }

    /// Get a texture by index (0 = Diffuse, 1 = Specular, etc.)
    pub fn get_tex(&self, index: usize) -> Option<&Texture> {
        if index < self.textures.len() {
            self.textures[index].as_ref()
        } else {
            None
        }
    }

    /// Convenience: get by "type" using a fixed mapping
    pub fn get_tex_by_type(&self, tex_type: &str) -> Option<&Texture> {
        match tex_type {
            "Diffuse" => self.textures[0].as_ref(),
            "Specular" => self.textures[1].as_ref(),
            "Emissive" => self.textures[2].as_ref(),
            "Opacity" => self.textures[3].as_ref(),
            _ => None,
        }
    }

    pub fn setup_opengl(&mut self) {
        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut self.vao));
            gl_call!(gl::GenBuffers(1, &mut self.vbo));
            gl_call!(gl::GenBuffers(1, &mut self.ebo));

            gl_call!(gl::BindVertexArray(self.vao));
            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo));

            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<Vertex>() * self.vertices.len()) as isize,
                self.vertices.as_ptr().cast(),
                gl::STATIC_DRAW,
            ));

            gl_call!(gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo));
            gl_call!(gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (mem::size_of::<u32>() * self.indices.len()) as isize,
                self.indices.as_ptr().cast(),
                gl::STATIC_DRAW
            ));

            gl_call!(gl::EnableVertexAttribArray(0));
            gl_call!(gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                ptr::null(),
            ));

            gl_call!(gl::EnableVertexAttribArray(1));
            gl_call!(gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, normal) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(2));
            gl_call!(gl::VertexAttribPointer(
                2,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, uv) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(3));
            gl_call!(gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, base_color) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(4));
            gl_call!(gl::VertexAttribIPointer(
                4,
                4,
                gl::INT,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, bone_ids) as *const _
            ));

            gl_call!(gl::EnableVertexAttribArray(5));
            gl_call!(gl::VertexAttribPointer(
                5,
                4,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, bone_weights) as *const _
            ));

            gl::BindVertexArray(0);
        }
    }

    pub fn draw(&self, shader: &mut Shader) {
        if self.color_for_texture {
            shader.set_bool("use_base_color", true);
            shader.set_bool("has_opacity_texture", false);
        } else {
            shader.set_bool("use_base_color", false);
            if let Some(diff) = self.get_tex(1) {
                // Diffuse
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE1);
                    gl::BindTexture(gl::TEXTURE_2D, diff.id);
                }
            }
            if let Some(spec) = self.get_tex(2) {
                // Specular
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE2);
                    gl::BindTexture(gl::TEXTURE_2D, spec.id);
                }
            }
            if let Some(emis) = self.get_tex(3) {
                // Emissive
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE3);
                    gl::BindTexture(gl::TEXTURE_2D, emis.id);
                }
            }
            if let Some(opac) = self.get_tex(8) {
                shader.set_bool("has_opacity_texture", true);
                unsafe {
                    gl::ActiveTexture(gl::TEXTURE4);
                    gl::BindTexture(gl::TEXTURE_2D, opac.id);
                }
            } else {
                shader.set_bool("has_opacity_texture", false);
            }
        }

        unsafe {
            gl_call!(gl::BindVertexArray(self.vao));
            gl_call!(gl::DrawElements(
                gl::TRIANGLES,
                self.indices.len() as i32,
                gl::UNSIGNED_INT,
                ptr::null(),
            ));

            shader.set_bool("has_opacity_texture", false);
            shader.set_bool("use_base_color", false);
            gl_call!(gl::BindVertexArray(0));
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bone {
    // id will be the position in the final bone array as well.
    pub id: u32,
    pub parent_index: Option<u32>,
    pub name: String,
    pub offset: Affine3A,
    pub children: Vec<Bone>,
}

#[derive(Debug, Clone)]
pub struct BoneJoinInfo {
    pub name: String,
    // offset: Mat4,
}

#[derive(Debug, Clone)]
pub struct BoneTransformTrack {
    pub position_timestamps: Vec<f32>,
    pub rotation_timestamps: Vec<f32>,
    pub scale_timestamps: Vec<f32>,

    pub positions: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub scales: Vec<Vec3>,
}

impl BoneTransformTrack {
    pub fn default() -> Self {
        Self {
            position_timestamps: vec![],
            rotation_timestamps: vec![],
            scale_timestamps: vec![],

            positions: vec![],
            rotations: vec![],
            scales: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Animator {
    pub current_animation: AnimationType,
    pub next_animation: AnimationType,
    pub animations: HashMap<AnimationType, Animation>,
    pub blend_factor: f32,
    pub blend_time: f32,
    pub restarted: bool, // TODO: A hack for determining if we should restart attack animations
}

impl Animator {
    pub fn new() -> Self {
        Self {
            current_animation: AnimationType::Idle,
            next_animation: AnimationType::Idle,
            animations: HashMap::new(),
            blend_factor: 0.0,
            blend_time: 0.14,
            restarted: false,
        }
    }

    pub fn get_current_animation(&self) -> Option<&Animation> {
        self.animations
            .get(&self.current_animation)
            .or_else(|| self.animations.get(&AnimationType::Idle))
        //.or_else(|| self.animations.values().next())
    }

    pub fn set_current_animation(&mut self, input: AnimationType) {
        self.current_animation = input;
    }

    pub fn set_next_animation(&mut self, input: AnimationType) {
        // match self.animations.get_mut(&input) {
        //     Some(anim) => anim.current_time = 0.0,
        //     None => {
        //         println!("WARNING: passed in an animation that wasn't found: {}", &input);
        //         self.next_animation = self.current_animation.clone();
        //     },
        // }
        self.next_animation = input;
    }

    pub fn update(&mut self, skellington: &mut Bone, dt: f32) {
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
                current.update(skellington, Some(next), self.blend_factor, dt);
            }
        } else if let Some(current) = self.animations.get_mut(&curr_key) {
            current.update(skellington, None, self.blend_factor, dt);
        }
    }
}

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

    //pub fn calculate_pose(
    //    &mut self,
    //    skeleton: &mut Bone,
    //    parent_transform: Mat4,
    //    global_inverse_transform: Mat4,
    //) {
    //    let delta = self.current_time % self.duration;
    //    let (local_position, local_rot, local_scale) =
    //        self.get_bone_local_transform(skeleton, delta);
    //    let local_transform =
    //        Mat4::from_scale_rotation_translation(local_scale, local_rot, local_position);
    //    let global_transform = parent_transform * local_transform;

    //    self.current_pose[skeleton.id as usize] = match 0 {
    //        0 => global_inverse_transform * global_transform * skeleton.offset,
    //        1 => global_transform * skeleton.offset * global_inverse_transform,
    //        2 => global_transform * skeleton.offset,
    //        3 => global_inverse_transform * global_transform,
    //        4 => global_inverse_transform * skeleton.offset * global_transform,
    //        5 => global_transform * global_inverse_transform * skeleton.offset,
    //        10 => Mat4::IDENTITY,
    //        _ => global_inverse_transform * global_transform * skeleton.offset,
    //    };

    //    for child in skeleton.children.iter_mut() {
    //        self.calculate_pose(child, global_transform, global_inverse_transform);
    //    }
    //}

    //// The idea is that at blend factor 0.0 we are at the self/current animation.
    //// At blend factor 1.0 we are fully at the "other" animation.
    //// At that point the current animation should be switched to the "other"
    //pub fn calculate_pose_blended(
    //    &mut self,
    //    skeleton: &mut Bone,
    //    parent_transform: Mat4,
    //    global_inverse_transform: Mat4,
    //    other_animation: &mut Animation,
    //    blend_factor: f32,
    //) {
    //    let delta1 = self.current_time % self.duration;
    //    let delta2 = other_animation.current_time % other_animation.duration;

    //    let (pos1, rot1, scale1) = self.get_bone_local_transform(skeleton, delta1);
    //    let (pos2, rot2, scale2) = other_animation.get_bone_local_transform(skeleton, delta2);

    //    let final_pos = pos1.lerp(pos2, blend_factor);
    //    let final_rot = rot1.slerp(rot2, blend_factor);
    //    let final_scale = scale1.lerp(scale2, blend_factor);

    //    let local_transform =
    //        Mat4::from_scale_rotation_translation(final_scale, final_rot, final_pos);
    //    let global_transform = parent_transform * local_transform;

    //    self.current_pose[skeleton.id as usize] = match 0 {
    //        0 => global_inverse_transform * global_transform * skeleton.offset,
    //        1 => global_transform * skeleton.offset * global_inverse_transform,
    //        2 => global_transform * skeleton.offset,
    //        3 => global_inverse_transform * global_transform,
    //        4 => global_inverse_transform * skeleton.offset * global_transform,
    //        5 => global_transform * global_inverse_transform * skeleton.offset,
    //        10 => Mat4::IDENTITY,
    //        _ => global_inverse_transform * global_transform * skeleton.offset,
    //    };

    //    for child in skeleton.children.iter_mut() {
    //        self.calculate_pose_blended(
    //            child,
    //            global_transform,
    //            global_inverse_transform,
    //            other_animation,
    //            blend_factor,
    //        );
    //    }
    //}

    pub fn calculate_pose_affine(&mut self, skeleton: &mut Bone, parent: Affine3A, delta: f32) {
        let (t, r, s) = self.get_bone_local_transform(skeleton, delta);

        let local = Affine3A::from_scale_rotation_translation(s, r, t);
        let global = parent * local;

        self.current_pose[skeleton.id as usize] = Mat4::from(global * skeleton.offset);

        for child in skeleton.children.iter_mut() {
            self.calculate_pose_affine(child, global, delta);
        }
    }

    pub fn calculate_pose_blended_affine(
        &mut self,
        skeleton: &mut Bone,
        parent: Affine3A,
        delta1: f32,
        other_animation: &mut Animation,
        delta2: f32,
        blend_factor: f32,
    ) {
        let (pos1, rot1, scale1) = self.get_bone_local_transform(skeleton, delta1);
        let (pos2, rot2, scale2) = other_animation.get_bone_local_transform(skeleton, delta2);

        let t = pos1.lerp(pos2, blend_factor);
        let r = rot1.slerp(rot2, blend_factor);
        let s = scale1.lerp(scale2, blend_factor);

        let local = Affine3A::from_scale_rotation_translation(s, r, t);
        let global = parent * local;

        self.current_pose[skeleton.id as usize] = Mat4::from(global * skeleton.offset);

        for child in skeleton.children.iter_mut() {
            self.calculate_pose_blended_affine(
                child,
                global,
                delta1,
                other_animation,
                delta2,
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
                self.current_time = self.duration - ANIMATION_EPSILON;
            }
        }

        let delta1 = self.current_time % self.duration;

        if let Some(other) = other_animation {
            let delta2 = other.current_time % other.duration;

            self.calculate_pose_blended_affine(
                skellington,
                Affine3A::IDENTITY,
                delta1,
                other,
                delta2,
                blend_factor,
            );

            // Preserve: you advance other_animation AFTER posing
            other.current_time += dt;
            if other.current_time > other.duration {
                other.current_time = 0.0;
            }
        } else {
            self.calculate_pose_affine(skellington, Affine3A::IDENTITY, delta1);
        }
    }
}

pub fn get_time_fraction(times: &[f32], dt: f32) -> (u32, f32) {
    let mut segment = 0;

    while dt > times[segment] {
        segment += 1;
    }

    if segment == 0 {
        return (0, 0.0); // Avoid accessing times[-1], return first segment with no interpolation
    }

    let start = times[segment - 1];
    let end = times[segment];
    let frac = (dt - start) / (end - start);

    (segment as u32, frac)
}
