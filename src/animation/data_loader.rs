#![allow(clippy::useless_vec)]
use core::f32;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use std::{ffi::c_void, path::Path, str::Lines};

use crate::{
    animation::{
        animation::Animation,
        animator::Animator,
        model::{Model, Texture, Vertex},
        skellington::{Bone, BoneJoinInfo, BoneTransformTrack},
    },
    enums_types::{AnimationType, TextureProfile, TextureType},
    gl_call,
    util::constants::MAX_BONE_INFLUENCE,
};

pub fn import_bone_data(
    file_path: &str,
    flip_180: bool,
    weapon_bone: Option<&str>,
) -> (Bone, Animator, Animation, Option<usize>) {
    let data = match std::fs::read_to_string(file_path) {
        Ok(data) => data,
        Err(_) => panic!("{}", file_path),
    };

    let mut lines = data.lines();

    let mut bones_no_children = Vec::new();
    let mut bone_idx = 0;
    let mut bone_count: u32 = 0;

    let mut rh_bone_id = None;

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

                if let Some(weapon_bone) = weapon_bone {
                    if name == weapon_bone {
                        rh_bone_id = Some(bone_idx);
                    }
                }
                let parsed_parent: i32 = lines
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .collect::<Vec<&str>>()[1]
                    .parse()
                    .unwrap();

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
                    global_transform: Mat4::default(),
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
        model_animation_join.push(BoneJoinInfo {
            name: b.name.clone(),
            // offset: b.offset,
        });

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

                    if current_anim_str == "Death"
                        || current_anim_str == "Slash"
                        || current_anim_str == "Slash2"
                        || current_anim_str == "DashF"
                        || current_anim_str == "Jump"
                        || current_anim_str == "Flinch"
                        || current_anim_str == "Block"
                        || current_anim_str == "Basic1"
                        || current_anim_str == "Basic2"
                        || current_anim_str == "Basic3"
                        || current_anim_str == "Freefall"
                        || current_anim_str == "OSBasic1"
                        || current_anim_str == "OSBasic2"
                        || current_anim_str == "OSBasic3"
                        || current_anim_str == "Roll"
                    {
                        println!("Found {}, setting looping to false", &current_anim_str);
                        animation.looping = false;
                    }

                    animator.animations.insert(
                        AnimationType::from_str(current_anim_str).unwrap(),
                        animation.clone(),
                    );
                }

                animation = Animation::default();
                animation.bone_transforms =
                    vec![BoneTransformTrack::default(); bone_count as usize];
                current_anim_str = parts[1].trim();

                dbg!(&current_anim_str);

                for b in &bones_no_children {
                    animation.current_pose.push(b.offset);
                }
            }
            "DURATION:" => animation.duration = parts[1].parse().unwrap(),
            "FPS:" => ticks_per_second = parts[1].parse().unwrap(),
            "TIMESTAMP:" => {
                let time_stamp = parts[1].parse().unwrap();

                // let mut skipped_bones = HashSet::new();

                for i in 0..bone_count {
                    let track = &mut animation.bone_transforms[i as usize];

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
    animator.animations.insert(
        AnimationType::from_str(current_anim_str).unwrap(),
        animation.clone(),
    );

    if !current_anim_str.is_empty() {
        animation.model_animation_join = model_animation_join.clone();
        animation.ticks_per_second = ticks_per_second;

        if current_anim_str == "Death"
            || current_anim_str == "Slash"
            || current_anim_str == "Slash2"
            || current_anim_str == "DashF"
            || current_anim_str == "Flinch"
            || current_anim_str == "Jump"
            || current_anim_str == "Block"
            || current_anim_str == "Basic1"
            || current_anim_str == "Basic2"
            || current_anim_str == "Basic3"
            || current_anim_str == "Freefall"
            || current_anim_str == "OSBasic1"
            || current_anim_str == "OSBasic2"
            || current_anim_str == "OSBasic3"
            || current_anim_str == "Roll"
        {
            println!("Found {}, setting looping to false", &current_anim_str);
            animation.looping = false;
        }

        animator.animations.insert(
            AnimationType::from_str(current_anim_str).unwrap(),
            animation.clone(),
        );
    }

    for (_, animation) in animator.animations.iter_mut() {
        for track in animation.bone_transforms.iter_mut() {
            if !track.positions.is_empty() {
                track.positions.remove(0);
                track.position_timestamps.remove(0);
                track.rotations.remove(0);
                track.rotation_timestamps.remove(0);
                track.scales.remove(0);
                track.scale_timestamps.remove(0);
            }
        }
    }

    for b in &bones_no_children {
        let t = &animation.bone_transforms[b.id as usize];
        if t.positions.is_empty() || t.rotations.is_empty() || t.scales.is_empty() {
            eprintln!("WARN: empty track for bone {:?}", b.name);
        }
    }

    (bone, animator, animation, rh_bone_id)
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

    let mut use_color_for_texture = false; // header toggle (if present)
    let mut saw_any_color = false; // infer from data

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
                texture_prof =
                    TextureProfile::from_str(parts[1]).unwrap_or(TextureProfile::BroadDefault);
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
                let indices: Vec<u32> = lines
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .map(|n| n.parse().unwrap())
                    .collect();

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
                texture_from_file(
                    &mut model,
                    path,
                    TextureType::Specular,
                    texture_prof.clone(),
                );
            }
            "TEXTURE_EMISSIVE:" => {
                let path = parts[1].to_string();
                texture_from_file(
                    &mut model,
                    path,
                    TextureType::Emissive,
                    texture_prof.clone(),
                );
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

// TODO: !IMPORTANT! Only diffuse etc should be SRGB8, everything else should be linear!!!!
pub fn texture_from_file(
    model: &mut Model,
    path: String,
    texture_type: TextureType,
    texture_prof: TextureProfile,
) {
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
                    let mut imgbuf = ImageBuffer::new(1, 1);
                    let color_u8 = [198, 198, 198, 255];

                    for pixel in imgbuf.pixels_mut() {
                        *pixel = Rgba(color_u8);
                    }

                    let color_path = format!(
                        "{:.3}-{:.3}-{:.3}.png",
                        color_u8[0], color_u8[1], color_u8[2]
                    );
                    let save_loc = format!("{}/{}", model.directory, color_path);

                    imgbuf.save(save_loc).expect("Failed to save texture image");

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
                gl::SRGB8 as i32,
                img_width as i32,
                img_height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                raw.as_ptr() as *const c_void
            ));

            match texture_prof {
                TextureProfile::DecalCrisp => {
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_WRAP_S,
                        gl::CLAMP_TO_EDGE as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_WRAP_T,
                        gl::CLAMP_TO_EDGE as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::NEAREST as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::NEAREST as i32
                    ));
                    // gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                }
                TextureProfile::BroadDefault => {
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_WRAP_S,
                        gl::REPEAT as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_WRAP_T,
                        gl::REPEAT as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::NEAREST_MIPMAP_LINEAR as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::NEAREST as i32
                    ));
                    gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                }
                TextureProfile::AlphaMasked => {
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_WRAP_S,
                        gl::CLAMP_TO_EDGE as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_WRAP_T,
                        gl::CLAMP_TO_EDGE as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::LINEAR_MIPMAP_LINEAR as i32
                    ));
                    gl_call!(gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::LINEAR as i32
                    ));
                    gl_call!(gl::GenerateMipmap(gl::TEXTURE_2D));
                }
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

fn build_tree_node(bone_id: usize, bones: &[Bone], children_of: &[Vec<usize>]) -> Bone {
    let original = &bones[bone_id as usize];
    let mut node = Bone {
        id: original.id,
        parent_index: original.parent_index,
        name: original.name.clone(),
        offset: original.offset,
        children: Vec::new(),
        global_transform: Mat4::default(),
    };

    for &child_id in &children_of[bone_id as usize] {
        let child = build_tree_node(child_id, bones, children_of);
        node.children.push(child);
    }

    node
}
