use core::f32;
use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use std::{path::Path, str::Lines};
use wgpu::util::DeviceExt;

use crate::{
    animation::{
        animation::Animation,
        animator::Animator,
        skellington::{Bone, BoneJoinInfo, BoneTransformTrack},
    },
    assets::{self, try_load_binary},
    enums_types::{AnimationType, TextureProfile},
    util::constants::MAX_BONE_INFLUENCE,
    wgpu_backend::{
        material::Material, model::Model, render_context::RenderContext, texture::Texture,
        vertex::Vertex,
    },
};

pub fn import_bone_data(
    file_path: &str,
    flip_180: bool,
    weapon_bone: Option<&str>,
) -> (Bone, Animator, Animation, Option<usize>) {
    let data = match assets::read_text(file_path) {
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
                        || current_anim_str == "Stagger"
                        || current_anim_str == "Stabby"
                        || current_anim_str == "Spin2Win"
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
            || current_anim_str == "Stagger"
            || current_anim_str == "Stabby"
            || current_anim_str == "Spin2Win"
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

pub fn import_model_data(file_path: &str, animation: &Animation, rdr_ctx: &RenderContext) -> Model {
    let data = assets::read_text(file_path).unwrap_or_else(|error| {
        panic!("Failed to read model file '{file_path}': {error}");
    });
    let mut lines = data.lines().peekable();

    let mut model: Option<Model> = None;
    let mut texture: Option<Texture> = None;

    let mut vertices = vec![];
    let mut indices = vec![];

    let directory = Path::new(file_path).parent().unwrap().to_str().unwrap();
    println!("Directory of Model is: {}", &directory);
    println!("=============================================================");

    let full_path = file_path;

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
                let position = {
                    let mut parts = lines.next().unwrap().split_whitespace();
                    [
                        parts.next().unwrap().parse::<f32>().unwrap(),
                        parts.next().unwrap().parse::<f32>().unwrap(),
                        parts.next().unwrap().parse::<f32>().unwrap(),
                    ]
                };

                let normal = {
                    let mut parts = lines.next().unwrap().split_whitespace();
                    [
                        parts.next().unwrap().parse::<f32>().unwrap(),
                        parts.next().unwrap().parse::<f32>().unwrap(),
                        parts.next().unwrap().parse::<f32>().unwrap(),
                    ]
                };

                let uv = {
                    let mut parts = lines.next().unwrap().split_whitespace();
                    [
                        parts.next().unwrap().parse::<f32>().unwrap(),
                        parts.next().unwrap().parse::<f32>().unwrap(),
                    ]
                };

                let mut vertex = Vertex {
                    position,
                    normal,
                    uv,
                    bone_ids: [-1; MAX_BONE_INFLUENCE],
                    bone_weights: [0.0; MAX_BONE_INFLUENCE],
                };

                let weight_parts: Vec<&str> = lines.next().unwrap().split_whitespace().collect();

                if let Some(first) = weight_parts.first() {
                    if *first != "None" {
                        for (i, pair) in weight_parts.chunks(2).enumerate() {
                            // Some exporters can emit a trailing bone name without a weight.
                            // Ignore malformed pairs instead of panicking.
                            let Some(bone_name) = pair.get(0).copied() else {
                                continue;
                            };
                            let Some(weight_str) = pair.get(1).copied() else {
                                continue;
                            };
                            let weight: f32 = weight_str.parse().unwrap_or(0.0);

                            let mut bone_id: i32 = -1;

                            for (j, info) in animation.model_animation_join.iter().enumerate() {
                                if info.name == bone_name {
                                    bone_id = j as i32;
                                }
                            }

                            vertex.bone_ids[i] = bone_id;
                            vertex.bone_weights[i] = weight;
                        }
                        let sum: f32 = vertex.bone_weights.iter().sum();
                        if sum > 0.0 {
                            for w in vertex.bone_weights.iter_mut() {
                                *w /= sum;
                            }
                        }
                    }
                }

                vertices.push(vertex);
            }
            "INDEX_COUNT:" => {
                let index_count: u32 = parts[1].parse().unwrap();
                indices = lines
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .map(|n| n.parse().unwrap())
                    .collect::<Vec<u32>>();

                dbg!(indices.len());
                dbg!(index_count);
                assert!(index_count == indices.len() as u32);
            }
            "TEXTURE_DIFFUSE:" => {
                let raw = parts[1];
                // If the texture path is just a filename (e.g. "diff.png"), resolve it relative
                // to the model file directory. If it already contains separators, treat it as a
                // path the asset loader can handle.
                let path = if raw.contains('/') || raw.contains('\\') {
                    raw.replace('\\', "/")
                } else {
                    format!("{}/{}", directory.replace('\\', "/"), raw)
                };

                match try_load_binary(&path) {
                    Some(data) => {
                        texture = Some(Texture::from_bytes(rdr_ctx, &data, file_path));
                    }
                    None => {
                        eprintln!(
                            "WARN: missing diffuse texture '{path}' for model '{file_path}'; using fallback texture"
                        );
                        texture = Some(Texture::from_solid_rgba8_srgb(
                            rdr_ctx.device,
                            rdr_ctx.queue,
                            [255, 0, 255, 255],
                            None,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    let vertex_buffer = rdr_ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", file_path)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let index_buffer = rdr_ctx
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", file_path)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

    let diffuse_texture = texture.unwrap_or_else(|| {
        eprintln!(
            "WARN: model '{file_path}' did not specify TEXTURE_DIFFUSE; using fallback texture"
        );
        Texture::from_solid_rgba8_srgb(rdr_ctx.device, rdr_ctx.queue, [255, 0, 255, 255], None)
    });

    let bind_group = rdr_ctx
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            layout: rdr_ctx.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

    model = Some(Model {
        vertex_buffer,
        index_buffer,

        vertices,
        indices: indices.clone(),

        num_elements: indices.len() as u32,

        material: Material {
            diffuse_texture: diffuse_texture,
            bind_group,
        },
        directory: directory.to_string(),
        full_path: full_path.to_string(),
    });

    model.unwrap()
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
        parts[0].parse::<f32>().unwrap(),
        parts[1].parse::<f32>().unwrap(),
        parts[2].parse::<f32>().unwrap(),
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
