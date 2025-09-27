#![allow(clippy::useless_vec)]
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use rapier3d::prelude::Cuboid;
use core::f32;
use std::{collections::HashMap, ffi::c_void, mem::{self, offset_of}, path::Path, ptr, str::Lines};

use crate::{enums_types::{AnimationType, FrameActivation, TextureProfile, TextureType, ANIMATION_EPSILON}, gl_call, shaders::Shader, some_data::MAX_BONE_INFLUENCE, sound::sound_manager::{ContinuousSound, OneShot}};

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
            "Diffuse"  => self.textures[0].as_ref(),
            "Specular" => self.textures[1].as_ref(),
            "Emissive" => self.textures[2].as_ref(),
            "Opacity"  => self.textures[3].as_ref(),
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
            if let Some(diff) = self.get_tex(1) { // Diffuse
                unsafe { gl::ActiveTexture(gl::TEXTURE1); gl::BindTexture(gl::TEXTURE_2D, diff.id); }
            }
            if let Some(spec) = self.get_tex(2) { // Specular
                unsafe { gl::ActiveTexture(gl::TEXTURE2); gl::BindTexture(gl::TEXTURE_2D, spec.id); }
            }
            if let Some(emis) = self.get_tex(3) { // Emissive
                unsafe { gl::ActiveTexture(gl::TEXTURE3); gl::BindTexture(gl::TEXTURE_2D, emis.id); }
            }
            if let Some(opac) = self.get_tex(8) {
                shader.set_bool("has_opacity_texture", true);
                unsafe { gl::ActiveTexture(gl::TEXTURE4); gl::BindTexture(gl::TEXTURE_2D, opac.id); }
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
    id: u32,
    parent_index: Option<u32>,
    name: String,
    offset: Mat4,
    children: Vec<Bone>,
}

#[derive(Debug, Clone)]
pub struct BoneJoinInfo {
    pub name: String,
    // offset: Mat4,
}

#[derive(Debug, Clone)]
pub struct BoneTransformTrack {
    position_timestamps: Vec<f32>,
    rotation_timestamps: Vec<f32>,
    scale_timestamps: Vec<f32>,

    positions: Vec<Vec3>,
    rotations: Vec<Quat>,
    scales: Vec<Vec3>,
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

#[derive(Debug)]
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
            blend_time: 0.2,
            restarted: false,
        }
    }

    pub fn get_current_animation(&self) -> Option<&Animation> {
        self.animations
            .get(&self.current_animation)
            .or_else(|| self.animations.get(&AnimationType::Idle))
            .or_else(|| self.animations.values().next())
    }

    pub fn set_current_animation(&mut self, input: AnimationType) {
        self.current_animation = input;
    }

    pub fn set_next_animation(&mut self, input: AnimationType) {
        self.next_animation = input;
    }

    pub fn update(&mut self, skellington: &mut Bone, dt: f32) {

        // Check death condition:
        if self.current_animation == AnimationType::Death {
            if let Some(anim) = self.animations.get(&AnimationType::Death) {
                if anim.current_time >= anim.duration {
                    return;
                }
            }
        }
        
        // TODO: Use the custom implementation of HasmapGetPairMut from items.rs#L64
        if self.current_animation != self.next_animation {
            self.blend_factor += dt / self.blend_time;
            if self.blend_factor >= 1.0 {
                self.blend_factor = 0.0;
                self.current_animation = self.next_animation.clone();
            }
        }

        let curr_key = self.current_animation.clone();
        let next_key = self.next_animation.clone();

        if curr_key != next_key {
            if let (Some(mut current), Some(mut next)) = (
                self.animations.remove(&curr_key),
                self.animations.remove(&next_key)
            ) {
                current.update(skellington, Some(&mut next), self.blend_factor, dt,);
                self.animations.insert(curr_key, current);
                self.animations.insert(next_key, next);
            }
        } else if let Some(mut current) = self.animations.remove(&curr_key) {
            current.update(skellington, None, self.blend_factor, dt);
            self.animations.insert(curr_key, current);
        }
    }
}


#[derive(Debug, Clone)]
pub struct Animation {
    pub duration: f32,
    ticks_per_second: f32,
    pub model_animation_join: Vec<BoneJoinInfo>,
    pub bone_transforms: HashMap<String, BoneTransformTrack>,
    pub current_pose: Vec<Mat4>,

    pub current_segment: u32,
    pub one_shots: Vec<OneShot>,
    pub continuous_sounds: Vec<ContinuousSound>,
    pub hurtbox_activation: Option<FrameActivation>,

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

            current_segment: 0,
            one_shots: vec![],
            continuous_sounds: vec![],
            hurtbox_activation: None,

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
        let (local_position, local_rot, local_scale) = self.get_bone_local_transform(skeleton, delta);
        let local_transform = Mat4::from_scale_rotation_translation(local_scale, local_rot, local_position);
        let global_transform = parent_transform * local_transform;

        self.current_pose[skeleton.id as usize] =
            match 0 {
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

        let local_transform = Mat4::from_scale_rotation_translation(final_scale, final_rot, final_pos);
        let global_transform = parent_transform * local_transform;

        self.current_pose[skeleton.id as usize] =
            match 0 {
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
            self.calculate_pose_blended(child, global_transform, global_inverse_transform, other_animation, blend_factor);

        }
    }

    fn get_bone_local_transform(&mut self, skeleton: &Bone, delta: f32) -> (Vec3, Quat, Vec3) {
        let btt = match self.bone_transforms.get(&skeleton.name) {
            Some(name) => name,
            _=> {
                dbg!(skeleton);
                panic!("skeleton name not found")
            }
        };

        let (segment, fraction) = get_time_fraction(&btt.position_timestamps, delta);

        self.current_segment = segment;

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

            (interpolated_position, interpolated_rotation, interpolated_scale)
        }
    }

    pub fn get_raw_global_bone_transform_by_name(
        &mut self,
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

            if let Some(found) = self.get_raw_global_bone_transform_by_name(bone_name, child, next_parent) {
                return Some(found);
            }
        }

        None
    }

    pub fn get_raw_global_bone_transform_by_name_blended(
        &mut self,
        bone_name: &str,
        skeleton: &Bone,
        parent_transform: Mat4,
        other_animation: &mut Animation,
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

    pub fn update(&mut self, skellington: &mut Bone, other_animation: Option<&mut Animation>, blend_factor: f32, dt: f32) {
        self.current_time += dt;
        if self.current_time > self.duration {
            if self.looping {
                self.current_time = 0.0;
            } else {
                // self.current_time = self.duration;
                self.current_time = self.duration - ANIMATION_EPSILON;
            }
        }

        // self.calculate_pose(
        //     skellington, 
        //     Mat4::IDENTITY,
        //     Mat4::IDENTITY, 
        // );

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
            self.calculate_pose(
                skellington, 
                Mat4::IDENTITY,
                Mat4::IDENTITY, 
            );
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

pub fn import_bone_data(file_path: &str, flip_180: bool) -> (Bone, Animator, Animation) {
    let data = match std::fs::read_to_string(file_path) {
        Ok(data) => data,
        Err(_) => panic!("{}", file_path),
    };

    let mut lines = data.lines();

    let mut bones_no_children = Vec::new();
    let mut bone_idx = 0;
    let mut bone_count: u32 = 0;


    // =============================================================
    // Get Starting Bones
    // ============================================================
    while let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "WiseModel" => {
                // name = "DefaultAnimation".to_string();
            }
            "SKELETON_DATA" => {
                println!("Found the beginning of skeleton data, beginning parse.");
            }
            "BONECOUNT:" => {
                bone_count = parts[1].parse().unwrap();
            }
            "BONE_NAME:" => {
                let name = parts[1].to_string();
                dbg!(&name);
                let parsed_parent: i32 = lines.next().unwrap().split_whitespace().collect::<Vec<&str>>()[1].parse().unwrap();

                let parent_index = match parsed_parent {
                    -1 => None,
                    _ => Some(parsed_parent as u32),
                };

                lines.next();
                let offset = parse_bone_offset(&mut lines);

                bones_no_children.push(Bone {
                    id: bone_idx,
                    parent_index,
                    name,
                    offset,
                    children: vec![],
                });

                bone_idx += 1;
            }
            _ => {}
        }
    }

    let bone = build_bone_hierarchy_top_down(bones_no_children.clone());
    // =============================================================
    // Get Animation Data
    // ============================================================
    lines = data.lines();
    let mut animation = Animation::default();
    let mut current_anim_str = "";

    // Get gpu bone info to use for later to gather a final matrix array
    let mut model_animation_join = vec![];

    for b in &bones_no_children {
        model_animation_join.push(
            BoneJoinInfo {
                name: b.name.clone(),
                // offset: b.offset,
            }
        );

        animation.current_pose.push(b.offset);
        assert!(model_animation_join[b.id as usize].name == b.name);
        assert!(model_animation_join.len() == animation.current_pose.len());

        // T pose
    }

    let mut animator = Animator::new();
    let mut ticks_per_second = 0.0;
    
    // THis assumes we always have ANIMATION_DATA after bone data.
    while let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "ANIMATION_DATA" => {
                println!("Found the beginning of animation data, beginning parse.");
            }
            "ANIMATION_NAME:" => {
                dbg!(&parts);
                if !current_anim_str.is_empty() {
                    // Save the previous animation before creating a new one
                    animation.model_animation_join = model_animation_join.clone();
                    animation.ticks_per_second = ticks_per_second;

                    if current_anim_str == "Death" || current_anim_str == "Slash" || current_anim_str == "Slash2" || current_anim_str == "Jump" {
                        println!("Found {}, setting looping to false", &current_anim_str);
                        animation.looping = false;
                    }

                    animator.animations.insert(AnimationType::from_str(current_anim_str).unwrap(), animation.clone());
                }

                animation = Animation::default();
                current_anim_str = parts[1].trim();

                dbg!(&current_anim_str);

                for b in &bones_no_children {
                    animation.current_pose.push(b.offset);
                }
            }
            "DURATION:" => {
                animation.duration = parts[1].parse().unwrap()
            }
            "FPS:" => {
                ticks_per_second = parts[1].parse().unwrap()
            }
            "TIMESTAMP:" => {
                let time_stamp = parts[1].parse().unwrap();

                // let mut skipped_bones = HashSet::new(); 

                for i in 0..bone_count {
                    let bone_name = model_animation_join[i as usize].name.clone();

                    let track = animation
                        .bone_transforms
                        .entry(bone_name.clone())
                        .or_insert_with(BoneTransformTrack::default);

                    let mut position = parse_vec3(lines.next().unwrap());
                    let mut rotation = parse_quat(lines.next().unwrap());
                    rotation = rotation.normalize();

                    if flip_180 && i == 0 {
                        let correction = Quat::from_rotation_y(std::f32::consts::PI);
                        position = correction * position;
                        rotation = correction * rotation;
                    }
                    let scale = parse_vec3(lines.next().unwrap());

                    lines.next();

                    //   if !skipped_bones.contains(&bone_name) {
                    //       skipped_bones.insert(bone_name);
                    //       continue;
                    //   }

                    track.position_timestamps.push(time_stamp);
                    track.rotation_timestamps.push(time_stamp);
                    track.scale_timestamps.push(time_stamp);


                    track.positions.push(position);
                    track.rotations.push(rotation);
                    track.scales.push(scale);

                }

            }
            _ => {}
        }
    }

    animation.model_animation_join = model_animation_join.clone();
    animation.ticks_per_second = ticks_per_second;
    
    animator.set_current_animation(AnimationType::from_str(current_anim_str).unwrap());
    animator.set_next_animation(AnimationType::from_str(current_anim_str).unwrap());
    animator.animations.insert(AnimationType::from_str(current_anim_str).unwrap(), animation.clone());

    if !current_anim_str.is_empty() {
        animation.model_animation_join = model_animation_join.clone();
        animation.ticks_per_second = ticks_per_second;
        if current_anim_str == "Death" || current_anim_str == "Slash" {
            println!("Found {}, setting looping to false", &current_anim_str);
            animation.looping = false;
        }

    animator.animations.insert(
        AnimationType::from_str(current_anim_str).unwrap(),
        animation.clone(),
    );
}

    for (_, animation) in animator.animations.iter_mut() {
        for (_, track) in animation.bone_transforms.iter_mut() {
            track.positions.remove(0);
            track.position_timestamps.remove(0);
            track.rotations.remove(0);
            track.rotation_timestamps.remove(0);
            track.scales.remove(0);
            track.scale_timestamps.remove(0);
        }
    }

    for b in &bones_no_children {
        if !animation.bone_transforms.contains_key(&b.name) {
            eprintln!("WARN: no track for bone {:?}", b.name);
        }
    }

    (bone, animator, animation)
}

pub fn import_model_data(file_path: &str, animation: &Animation) -> Model {
    let data = std::fs::read_to_string(file_path).unwrap();
    let mut lines = data.lines().peekable();

    let mut model = Model::new();

    let directory = Path::new(file_path).parent().unwrap().to_str().unwrap();
    println!("Directory of Model is: {}", &directory);
    println!("=============================================================");

    model.directory = directory.to_string();
    model.full_path = file_path.to_string();

    let mut use_color_for_texture = false;   // header toggle (if present)
    let mut saw_any_color = false;           // infer from data

    let mut texture_prof = TextureProfile::BroadDefault;



    while let Some(line) = lines.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "MESH_DATA" => {
                    println!("Found the beginning of skeleton data, beginning parse.");
            }
            "USE_COLOR_FOR_TEXTURE" => {}
            "TEXTURE_PROFILE:" => {
                texture_prof = TextureProfile::from_str(parts[1]).unwrap_or(TextureProfile::BroadDefault);
            }
            "MEME" => {}
            "VERT:" => {
                let position = parse_vec3(lines.next().unwrap());
                

                let normal = parse_vec3(lines.next().unwrap());
                let uv = parse_vec2(lines.next().unwrap());

                let mut base_color = glam::Vec4::splat(1.0);

                if let Some(peek) = lines.peek() {
                    if peek.trim_start().starts_with("COLOR:") {
                        let color_line = lines.next().unwrap(); // consume it
                        let col_str = color_line.trim_start_matches("COLOR:").trim();
                        let parsed = parse_vec4(col_str);
                        base_color = parsed;

                        // mark that we saw color data
                        saw_any_color = true;

                        // If you want presence of COLOR to auto-enable usage:
                        // (remove this if you only want to respect the header directive)
                        use_color_for_texture = true;
                    }
                }

                let mut vertex = Vertex {
                    position,
                    normal,
                    uv,
                    base_color,
                    bone_ids: [-1; MAX_BONE_INFLUENCE],
                    bone_weights: [0.0; MAX_BONE_INFLUENCE],
                };

                let weight_parts: Vec<&str> = lines.next().unwrap().split_whitespace().collect();

                if !weight_parts.first().unwrap().eq(&"None") {
                    for (i, pair) in weight_parts.chunks(2).enumerate() {
                        let bone_name = pair[0];
                        let weight: f32 = pair[1].parse().unwrap_or(0.0);

                        let mut bone_id: i32 = -1;

                        for (j, info) in animation.model_animation_join.iter().enumerate() {
                            if info.name == bone_name {
                                bone_id = j as i32;
                            }
                        }

                        vertex.bone_ids[i] = bone_id;
                        vertex.bone_weights[i] = weight;

                        // let total_weight = vertex.bone_weights.iter().sum::<f32>();
                        // if total_weight > 0.0 {
                        //     for w in vertex.bone_weights.iter_mut() {
                        //         *w /= total_weight;
                        //     }
                        // }
                    }
                    let sum: f32 = vertex.bone_weights.iter().sum();
                    if sum > 0.0 {
                        for w in vertex.bone_weights.iter_mut() {
                            *w /= sum;
                        }
                    }
                }

                model.vertices.push(vertex);
            }
            "INDEX_COUNT:" => {
                let index_count: u32 = parts[1].parse().unwrap();
                let indices: Vec<u32> = lines.next().unwrap().split_whitespace().map(|n| n.parse().unwrap()).collect();

                dbg!(indices.len());
                dbg!(index_count);
                assert!(index_count == indices.len() as u32);
                model.indices = indices;
            }
            "TEXTURE_DIFFUSE:" => {
                let path = parts[1].to_string();
                texture_from_file(&mut model, path, TextureType::Diffuse, texture_prof.clone());
            }
            "TEXTURE_SPECULAR:" => {
                let path = parts[1].to_string();
                texture_from_file(&mut model, path, TextureType::Specular, texture_prof.clone());
            }
            "TEXTURE_EMISSIVE:" => {
                let path = parts[1].to_string();
                texture_from_file(&mut model, path, TextureType::Emissive, texture_prof.clone());
            }
            "TEXTURE_OPACITY:" => {
                let path = parts[1].to_string();
                texture_from_file(&mut model, path, TextureType::Opacity, texture_prof.clone());
            }
            _ => {}
        }
    }

    model.color_for_texture = use_color_for_texture || saw_any_color;

    model.setup_opengl();

    model
}

pub fn texture_from_file(model: &mut Model, path: String, texture_type: TextureType, texture_prof: TextureProfile) {
    println!("texture is {}", &path);
    let file_name = model.directory.clone() + "/" + path.as_str();

    dbg!(&path);
    dbg!(&file_name);

    let mut texture_id = 0;
    unsafe {
        gl_call!(gl::GenTextures(1, &mut texture_id));

        let img = match image::open(file_name.clone()) {
            Ok(data) => Some(data),
            Err(_) => {
                if texture_type == TextureType::Diffuse {
                    // TODO: Parse BSDF color instead or something.
                    let mut imgbuf = ImageBuffer::new(1,1);
                    let color_u8 = [
                        198,
                        198,
                        198,
                        255,
                    ];

                    for pixel in imgbuf.pixels_mut() {
                        *pixel = Rgba(color_u8);
                    }

                    let color_path = format!("{:.3}-{:.3}-{:.3}.png" ,color_u8[0], color_u8[1], color_u8[2]);
                    let save_loc = format!("{}/{}", model.directory, color_path);

                    imgbuf
                        .save(save_loc)
                        .expect("Failed to save texture image");

                    Some(DynamicImage::ImageRgba8(imgbuf))
                } else {
                    None
                }
            }
        };

        if let Some(img) = img {
            let (img_width, img_height) = img.dimensions();
            let rgba = img.to_rgba8();
            let raw = rgba.as_raw();

            gl_call!(gl::BindTexture(gl::TEXTURE_2D, texture_id));
            gl_call!(gl::TexImage2D(
                gl::TEXTURE_2D, 
                0, 
                gl::RGBA as i32, 
                img_width as i32, 
                img_height as i32, 
                0, 
                gl::RGBA, 
                gl::UNSIGNED_BYTE, 
                raw.as_ptr() as *const c_void
            ));

            match texture_prof {
                TextureProfile::DecalCrisp => {
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
                    // gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                }, 
                TextureProfile::BroadDefault => {
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST_MIPMAP_LINEAR as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
                    gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                },
                TextureProfile::AlphaMasked => {
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as i32));
                    gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32));
                    gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                },
            }

            let texture = Texture {
                id: texture_id,
                _type: texture_type.clone().to_string(),
                path: file_name,
            };

            match texture_type {
                TextureType::Diffuse => {
                    model.textures[1] = Some(texture);
                }
                TextureType::Specular => {
                    model.textures[2] = Some(texture);
                }
                TextureType::Emissive => {
                    model.textures[3] = Some(texture);
                }
                TextureType::NormalMap => {
                    model.textures[4] = Some(texture);
                }
                TextureType::Roughness => {
                    model.textures[5] = Some(texture);
                }
                TextureType::Metalness => {
                    model.textures[6] = Some(texture);
                }
                TextureType::Displacement => {
                    model.textures[7] = Some(texture);
                }
                TextureType::Opacity => {
                    model.textures[8] = Some(texture);
                }
            }
        }
    }
}

fn parse_bone_offset(lines: &mut Lines<'_>) -> Mat4 {
    Mat4 {
        x_axis: parse_vec4(lines.next().unwrap()),
        y_axis: parse_vec4(lines.next().unwrap()),
        z_axis: parse_vec4(lines.next().unwrap()),
        w_axis: parse_vec4(lines.next().unwrap()),
    }
}

fn parse_vec4(input: &str) -> Vec4 {
    let parts: Vec<&str> = input.split_whitespace().collect();
    Vec4::new( 
        parts[0].parse().unwrap(),
        parts[1].parse().unwrap(),
        parts[2].parse().unwrap(),
        parts[3].parse().unwrap(),
    )
}

fn parse_vec3(input: &str) -> Vec3 {
    let parts: Vec<&str> = input.split_whitespace().collect();
    Vec3::new( 
        parts[0].parse().unwrap(),
        parts[1].parse::<f32>().unwrap(),
        parts[2].parse().unwrap(),
    )
}

fn parse_vec2(input: &str) -> Vec2 {
    let parts: Vec<&str> = input.split_whitespace().collect();
    Vec2::new( 
        parts[0].parse::<f32>().unwrap(),
        parts[1].parse::<f32>().unwrap(),
    )
}

fn parse_quat(input: &str) -> Quat {
    let parts: Vec<&str> = input.split_whitespace().collect();
    Quat::from_xyzw(
        parts[0].parse().unwrap(),
        parts[1].parse().unwrap(),
        parts[2].parse().unwrap(),
        parts[3].parse().unwrap(),
    )
}

fn build_bone_hierarchy_top_down(bones: Vec<Bone>) -> Bone {
    let mut children_of = vec![Vec::new(); bones.len()];

    for bone in &bones {
        if let Some(parent_id) = bone.parent_index {
            children_of[parent_id as usize].push(bone.id);
        }
    }

    let root_id = bones
        .iter()
        .find(|b| b.parent_index.is_none())
        .expect("No root bone found!")
    .id;

    build_tree_node(root_id, &bones, &children_of)
}

fn build_tree_node(
    bone_id: u32,
    bones: &[Bone],
    children_of: &[Vec<u32>],
) -> Bone {
    let original = &bones[bone_id as usize];
    let mut node = Bone {
        id: original.id,
        parent_index: original.parent_index,
        name: original.name.clone(),
        offset: original.offset,
        children: Vec::new(),
    };

    for &child_id in &children_of[bone_id as usize] {
        let child = build_tree_node(child_id, bones, children_of);
        node.children.push(child);
    }

    node
}
