        //TEXT STUFF

        // =============================================================
        // text
        // ============================================================
        
        let font_data = include_bytes!("../resources/fonts/JetBrainsMonoNL-Regular.ttf");
        let font = Font::try_from_bytes(font_data).unwrap();

        let scale = Scale::uniform(256.0);
        let v_metrics = font.v_metrics(scale);
        let glyph = font.glyph('B').scaled(scale).positioned(point(0.0, v_metrics.ascent));

        let mut glyph_tex: u32 = 0;
        
        let mut glyph_width = 0.0;
        let mut glyph_height = 0.0;


        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph_width = bb.width() as f32;
            glyph_height = bb.height() as f32;
            let width = bb.width() as usize;
            let height = bb.height() as usize;
            let mut pixel_data = vec![0u8; width * height];

            glyph.draw(|x, y, v| {
                let index = (y as usize * width) + x as usize;
                pixel_data[index] = (v * 255.0) as u8;
            });

            let img = GrayImage::from_vec(width as u32, height as u32, pixel_data.clone())
                .expect("Failed to create image");
            img.save("glyph_debug.png").expect("Failed to save image");

    println!("Saved glyph_debug.png ({}x{})", width, height);

            dbg!(width, height, &pixel_data[..10]); // Debug first few pixel values

            unsafe {
                gl_call!(gl::GenTextures(1, &mut glyph_tex));
                gl_call!(gl::BindTexture(gl::TEXTURE_2D, glyph_tex));

                gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1));

                gl_call!(gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RED as i32,
                    width as i32,
                    height as i32,
                    0,
                    gl::RED,
                    gl::UNSIGNED_BYTE,
                    pixel_data.as_ptr() as *const _,
                ));

                gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32));
                gl_call!(gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32));
                 gl_call!(gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4)); 
            }

        }

        let aspect_ratio = glyph_width / glyph_height;
        let pixel_scale_x = 2.0 / fb_width as f32;
        let pixel_scale_y = 2.0 / fb_height as f32;

        let width_ndc = glyph_width * pixel_scale_x;
        let height_ndc = glyph_height * pixel_scale_y;
        let scale = 1.0;
        let quad_vertices: [f32; 30] = [
            // Positions         // Flipped Texture Coords (Swap Y)
            -scale * aspect_ratio,  scale, 0.0,  0.0, 0.0,  // Top-left (was 1.0)
            -scale * aspect_ratio, -scale, 0.0,  0.0, 1.0,  // Bottom-left (was 0.0)
            scale * aspect_ratio, -scale, 0.0,  1.0, 1.0,  // Bottom-right

            -scale * aspect_ratio,  scale, 0.0,  0.0, 0.0,  // Top-left
            scale * aspect_ratio, -scale, 0.0,  1.0, 1.0,  // Bottom-right
            scale * aspect_ratio,  scale, 0.0,  1.0, 0.0   // Top-right
        ];

        let mut tex_vao = 0;
        let mut tex_vbo = 0;

        unsafe {
            gl_call!(gl::GenVertexArrays(1, &mut tex_vao));
            gl_call!(gl::GenBuffers(1, &mut tex_vbo));

            gl_call!(gl::BindVertexArray(tex_vao));

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, tex_vbo));
            gl_call!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            ));

            // Position attribute
            gl_call!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, (5 * std::mem::size_of::<f32>()) as i32, std::ptr::null()));
            gl_call!(gl::EnableVertexAttribArray(0));

            // Texture coordinate attribute
            gl_call!(gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, (5 * std::mem::size_of::<f32>()) as i32, (3 * std::mem::size_of::<f32>()) as *const _));
            gl::EnableVertexAttribArray(1);

            gl_call!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
            gl_call!(gl::BindVertexArray(0));
        }




		{
			"entity_type": "MooseMan",
			"position": [0.0, 0.0, 4.0],
			"scale": [0.013, 0.013, 0.013],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "-FRAC_PI_2",
			"faction": "Enemy",
			"mesh_path": "resources/models/animated/001_moose/moose_model_FINAL.txt", 
			"bone_path": "resources/models/animated/001_moose/moose_bones_FINAL.txt",
			"animation_properties" : [
				{
					"name": "Idle",
					"one_shots": { },
					"continuous_sounds": [
						"moose3D"
					]
				}
			]
		},



		{
			"entity_type": "TreeFoliage",
			"position": [-4.2, 0.0, -3.1],
			"scale": [1.0, 1.0, 1.0],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "",
			"faction": "Static",
			"mesh_path": "resources/models/static/trees/001_tree_foliage_model.txt", 
			"bone_path":"",
			"animation_properties" : []
		},
		{
			"entity_type": "TreeTrunk",
			"position": [-4.2, 0.0, -3.1],
			"scale": [1.0, 1.0, 1.0],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "",
			"faction": "Static",
			"mesh_path": "resources/models/static/trees/001_tree_trunk_model.txt", 
			"bone_path":"",
			"animation_properties" : []
		},
		{
			"entity_type": "TreeFoliage",
			"position": [3.0, 0.0, 2.8],
			"scale": [1.0, 1.0, 1.0],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "",
			"faction": "Static",
			"mesh_path": "resources/models/static/trees/001_tree_foliage_model.txt", 
			"bone_path":"",
			"animation_properties" : []
		},
		{
			"entity_type": "TreeTrunk",
			"position": [3.0, 0.0, 2.8],
			"scale": [1.0, 1.0, 1.0],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "",
			"faction": "Static",
			"mesh_path": "resources/models/static/trees/001_tree_trunk_model.txt", 
			"bone_path":"",
			"animation_properties" : []
		},


		{
			"entity_type": "TreeFoliage",
			"position": [5.0, 0.0, -2.1],
			"scale": [1.0, 1.0, 1.0],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "",
			"faction": "Static",
			"mesh_path": "resources/models/static/trees/001_tree_foliage_model.txt", 
			"bone_path":"",
			"animation_properties" : []
		},
		{
			"entity_type": "TreeTrunk",
			"position": [5.0, 0.0, -2.1],
			"scale": [1.0, 1.0, 1.0],
			"__note__": "use Quat::from_rotation_x()",
			"rotation": "",
			"faction": "Static",
			"mesh_path": "resources/models/static/trees/001_tree_trunk_model.txt", 
			"bone_path":"",
			"animation_properties" : []
		}







            // Donut revolution stuff
            if let Some(donut) = self.entity_types.iter().find(|e| e.value() == &EntityType::Donut) {
                let donut_key = donut.key();

                let player_position = self.transforms.get(player_key).map(|t| t.position);

                if let Some(donut_transform) = self.transforms.get_mut(donut_key) {
                    if let Some(player_position) = player_position {
                        revolve_around_something(
                            &mut donut_transform.position,
                            &player_position,
                            elapsed_time,
                            2.0,
                            5.0
                        );
                    }
                }
            }





        /// RNG FLAT MAP STUFF
    pub fn populate_floor_tiles(&mut self, grid: &Grid, model_path: &str) {
        for cell in grid.cells.iter() {
            let pos = cell.position;
            self.create_static_entity(EntityType::BlockGrass, Faction::World, pos, vec3(1.0, 1.0, 1.0), Quat::IDENTITY, model_path);
        }
    }

    pub fn populate_cell_rng(&mut self, grid: &Grid) {
        for cell in grid.cells.iter() {

            let (entity_data, subtile_size, entity_type) = match cell.cell_type {
                CellType::Tree => (TREES, 3.0, EntityType::Tree),
                CellType::Grass => (GRASSES, 3.0, EntityType::Grass),
                _=> continue,
            };

            let within = grid.cell_size / subtile_size;

            let cell_pos = cell.position;
            for x in -1..=1 {
                for z in -1..=1 {
                    let num = self.rng.random_range(0..entity_data.len() + 1);
                    let scale = match entity_type {
                        EntityType::Grass => self.rng.random_range(20..=45) as f32 / 100.0,
                        EntityType::Tree => self.rng.random_range(90..=110) as f32 / 100.0,
                        _=> 1.0,
                };
                    let smoff = self.rng.random_range(-0.1..=0.1);

                    let offset_x = x as f32 * within;
                    let offset_z = z as f32 * within;

                    if num < entity_data.len() {
                        self.create_static_entity(
                            entity_type.clone(),
                            Faction::World,
                            vec3(cell_pos.x + offset_x + smoff, 0.0, cell_pos.z + offset_z + smoff),
                            Vec3::splat(scale),
                            Quat::IDENTITY,
                            entity_data[num],
                        );
                    }
                }
            }
        }
    }


// fn entity_sim_state_machine(
//     em: &mut EntityManager, 
//     dt: f32, particles: &mut ParticleSystem, 
//     ps: &mut PhysicsState, 
//     input: &InputState
// ) {
//     for fac in em.factions.iter() {
//         if *fac.value() == Faction::Enemy {
//             let controller = em.simstate_controllers.get_mut(fac.key()).unwrap();
//             let player_key = em.factions.iter().find(|e| *e.value() == Faction::Player).unwrap().key();
//             let player_pos = em.transforms.get(player_key).unwrap().position;
//             let entity_pos = em.transforms.get(fac.key()).unwrap().position;
//             let animator = em.animators.get_mut(fac.key()).unwrap();
//             let destination = em.destinations.get_mut(fac.key()).unwrap();
//             let health = em.healths.get(fac.key()).unwrap();
//             let ph = em.physics_handles.get(fac.key()).unwrap();
//             let rb = ps.rigid_body_set.get_mut(ph.rigid_body).unwrap();
// 
//             let yaw = em.yaws.get(fac.key()).unwrap();
//             let dir = vec3(yaw.sin(), 1.0, yaw.cos()).normalize();
// 
//             let impulse_strength = vec3(7.0, 3.5, 7.0);
//             let m = rb.mass();
//             let impulse = vec3(dir.x * (7.0 * m), 0.0, dir.z * (7.0 * m));
// 
//             let kb = match em.knockbacks.get_mut(fac.key()) {
//                 Some(kb) => kb,
//                 None     => &mut Knockback { ttl: -1.0, flinch: false, did_particles: true },
//             };
// 
//             if em.v_effects.get(fac.key()).is_some_and(|v| v.ttl <= 0.0) {
//                 em.v_effects.remove(fac.key());
//             } else if let Some(v_effect) = em.v_effects.get_mut(fac.key()) {
//                 v_effect.ttl -= dt;
//             }
// 
//             if controller.state == SimState::Dancing { continue };
// 
//             let active_weapon_id = em
//                 .active_items
//                 .get(fac.key())
//                 .and_then(|ai| ai.right_hand);
// 
//             let weapon_length = active_weapon_id
//                 .and_then(|wid| {
//                     em.parents
//                         .iter()
//                         .find(|p| p.value().parent_id == wid && em.cuboids.get(p.key()).is_some())
//                         .and_then(|entry| em.cuboids.get(entry.key()).map(|hb| hb.h)) // child id = entry.key()
//                 })
//                 .unwrap();
// 
//             let within_weapon_length = entity_pos.distance(player_pos) <= weapon_length;
// 
//             let trans = em.transforms.get(fac.key()).unwrap();
// 
//             let next_state = (|| match controller.state {
//                 SimState::Dancing => {
//                     *destination = entity_pos;
//                     animator.set_next_animation(AnimationType::Dance);
//                     SimState::Dancing
//                 },
//                 SimState::Waiting => {
//                     if *health <= 0.0 { return SimState::Dying; }
// 
//                     animator.set_next_animation(AnimationType::Idle);
//                     *destination = entity_pos;
// 
//                     if kb.flinch && kb.ttl > 0.0 {
//                         controller.time_in_state = 0.0;
//                         animator.set_next_animation(AnimationType::Flinch);
//                         return SimState::Flinching;
//                     }
// 
//                     let to_player = (player_pos - entity_pos).with_y(0.0).normalize();
//                     // let forward = (trans.rotation * trans.original_rotation.inverse() * -Vec3::Z).with_y(0.0).normalize();
//                     let forward = (trans.rotation * Vec3::Z).with_y(0.0).normalize();
//                     let alignment = forward.dot(to_player);
//                     let fov_threshold = 0.5; // cos(30 degrees);
// 
//                     let view_distance = 12.0;
// 
//                     let player_in_range = entity_pos.distance(player_pos) <= view_distance;
// 
//                     if  alignment >= fov_threshold && player_in_range {
//                         animator.set_next_animation(AnimationType::Run);
//                         return SimState::Aggro
//                     }
// 
//                     SimState::Waiting
//                 },
//                 SimState::Aggro => {
//                     if *health <= 0.0 { return SimState::Dying; }
// 
//                     controller.time_in_state += dt;
// 
//                     if kb.flinch && kb.ttl > 0.0 {
//                         controller.time_in_state = 0.0;
//                         animator.set_next_animation(AnimationType::Flinch);
//                         return SimState::Flinching;
//                     }
// 
//                     if within_weapon_length {
//                         animator.set_next_animation(AnimationType::Slash);
//                         controller.time_in_state = 0.0;
//                         return SimState::Attacking
//                     }
// 
//                     *destination = player_pos;
// 
//                     if entity_pos.distance(player_pos) > 12.0 {
//                         return SimState::Waiting
//                     } 
// 
// 
//                     SimState::Aggro
//                 },
//                 SimState::Dying => {
// 
//                     em.v_effects.remove(fac.key());
// 
//                     animator.set_next_animation(AnimationType::Death);
//                     *destination = entity_pos;
//                     
//                     if let Some(anim) = animator.animations.get(&AnimationType::Death) {
//                         if anim.current_time >= anim.duration - ANIMATION_EPSILON {
//                             return SimState::Dead { time: 0.0, target_time: 5.0 }
//                         } 
//                     } else {
// 
//                         let model_transform = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
//                         let skellington = em.skellingtons.get_mut(fac.key()).unwrap();
// 
//                         let bone_names: Vec<String> = {
//                             let anim = animator.animations.get(&animator.current_animation).unwrap();
//                             anim.model_animation_join.iter().map(|b| b.name.clone()).collect()
//                         };
// 
//                         let anim = animator.animations.get_mut(&animator.current_animation).unwrap();
//                         for bone_name in bone_names{
// 
//                             if let Some(bone_world_model_space) = anim.get_raw_global_bone_transform_by_name(
//                                 &bone_name,
//                                 skellington,
//                                 Mat4::IDENTITY,
//                             ) {
// 
//                                 let bone_world_space = model_transform * bone_world_model_space;
//                                 let position = bone_world_space.w_axis.truncate();
// 
//                                 // You can randomize velocity or make it static for now
//                                 particles.spawn_oneshot_emitter(EmitterName::DamageBlood, position);
//                             }
//                         }
//                         //particles.spawn_oneshot_emitter(1000, entity_pos);
//                         em.entity_trashcan.push(fac.key());
//                     }
//                     
//                     SimState::Dying
//                 },
//                 SimState::Dead { time, target_time } => {
//                     animator.set_next_animation(AnimationType::Death);
// 
//                     let new_time = time + dt;
// 
//                     if new_time >= 4.0 {
//                         em.v_effects.insert(fac.key(), VisualEffect { 
//                             effect: Effect::Flashing,
//                             ttl: 5.0,
//                         });
//                     }
// 
//                     if new_time >= target_time {
//                         let model_transform = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
//                         let skellington = em.skellingtons.get_mut(fac.key()).unwrap();
// 
//                         let bone_names: Vec<String> = {
//                             let anim = animator.animations.get(&animator.current_animation).unwrap();
//                             anim.model_animation_join.iter().map(|b| b.name.clone()).collect()
//                         };
// 
//                         let anim = animator.animations.get_mut(&animator.current_animation).unwrap();
// 
//                         for bone_name in bone_names{
//                             if let Some(bone_world_model_space) = anim.get_raw_global_bone_transform_by_name(
//                                 &bone_name,
//                                 skellington,
//                                 Mat4::IDENTITY,
//                             ) {
//                                 let bone_world_space = model_transform * bone_world_model_space;
//                                 let position = bone_world_space.w_axis.truncate();
// 
//                                 // You can randomize velocity or make it static for now
//                                 particles.spawn_oneshot_emitter(EmitterName::DamageBlood, position);
//                             }
//                         }
// 
// 
//                         // let model = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
//                         // let  anim = animator.animations.get_mut(&animator.current_animation).unwrap();
//                         // let skellington = em.skellingtons.get(fac.key()).unwrap();
// 
//                         // if let Some(neck_transform_model_space) = anim.get_raw_global_bone_transform_by_name("mixamorig:Neck", skellington, Mat4::IDENTITY) {
//                         //     let world_transform = model * neck_transform_model_space;
//                         //     let neck_position = world_transform.w_axis.truncate();
//                         //     particles.spawn_particles(1000, neck_position);
//                         // }
// 
//                         // if let Some(hip_transform_model_space) = anim.get_raw_global_bone_transform_by_name("mixamorig:Hips", skellington, Mat4::IDENTITY) {
//                         //     let world_transform = model * hip_transform_model_space;
//                         //     let neck_position = world_transform.w_axis.truncate();
//                         //     particles.spawn_particles(1000, neck_position);
//                         // }
//                         em.entity_trashcan.push(fac.key());
//                     }
// 
//                     SimState::Dead { time: new_time, target_time: target_time }
//                 },
//                 SimState::Attacking => {
//                     if *health <= 0.0 { return SimState::Dying; }
// 
//                     let (slash1, slash2) = animator.animations.get_pair_mut(&AnimationType::Slash, &AnimationType::Slash2).unwrap();
// 
//                     controller.time_in_state += dt;
// 
//                     if kb.flinch && kb.ttl > 0.0 {
//                         controller.time_in_state = 0.0;
//                         animator.set_next_animation(AnimationType::Flinch);
//                         return SimState::Flinching;
//                     }
// 
//                     match controller.attack_state {
//                         AttackState::Attack1 => {
//                             if animator.current_animation != AnimationType::Slash && animator.next_animation != AnimationType::Slash {
//                                 animator.set_next_animation(AnimationType::Slash);
//                                 controller.attack_state = AttackState::Attack1;
//                                 return SimState::Attacking;
//                             }
// 
//                             if animator.current_animation != AnimationType::Slash {
//                                 return SimState::Attacking;
//                             }
// 
//                             slash2.current_time = 0.0;
// 
//                             if slash1.current_segment >= 14 {
//                                 if within_weapon_length {
// 
//                                     animator.set_next_animation(AnimationType::Slash2);
//                                     controller.attack_state = AttackState::Attack2;
//                                     return SimState::Attacking
//                                 }
//                             }
// 
//                             if slash1.current_time >= slash1.duration- ANIMATION_EPSILON {
//                                 if within_weapon_length {
//                                     return SimState::Attacking
//                                 }
//                                 
//                                 animator.set_next_animation(AnimationType::Run);
//                                 controller.attack_state = AttackState::Attack1;
//                                 return SimState::Aggro;
//                             }
//                         },
//                         AttackState::Attack2 => {
// 
//                             if animator.current_animation != AnimationType::Slash2 && animator.next_animation != AnimationType::Slash2 {
//                                 animator.set_next_animation(AnimationType::Slash2);
//                                 controller.attack_state = AttackState::Attack2;
//                                 return SimState::Attacking;
//                             }
// 
//                             if animator.current_animation != AnimationType::Slash2 {
//                                 return SimState::Attacking;
//                             }
// 
//                             slash1.current_time = 0.0;
// 
//                             if slash2.current_segment >= 18 {
//                                 if within_weapon_length {
//                                     controller.attack_state = AttackState::Attack1;
//                                     animator.set_next_animation(AnimationType::Slash);
//                                     return SimState::Attacking;
//                                 } else {
//                                     // rb.apply_impulse(impulse.into(), true);
//                                     // controller.attack_state = AttackState::Attack1;
//                                     // animator.set_next_animation(AnimationType::Slash);
//                                     // animator.set_next_animation(AnimationType::DashF);
//                                     // return SimState::Dashing;
//                                     controller.attack_state = AttackState::Attack1;
//                                     animator.set_next_animation(AnimationType::Run);
//                                     return SimState::Aggro;
//                                 }
//                             }
// 
//                             if slash2.current_segment >= 22 {
//                                 if within_weapon_length {
//                                     controller.attack_state = AttackState::Attack1;
//                                     animator.set_next_animation(AnimationType::Slash);
//                                     return SimState::Attacking;
//                                 } else {
//                                     // rb.apply_impulse(impulse.into(), true);
//                                     // controller.attack_state = AttackState::Attack1;
//                                     // animator.set_next_animation(AnimationType::Slash);
//                                     // animator.set_next_animation(AnimationType::DashF);
//                                     // return SimState::Dashing;
//                                     controller.attack_state = AttackState::Attack1;
//                                     animator.set_next_animation(AnimationType::Run);
//                                     return SimState::Aggro;
//                                 }
//                             }
//                         },
//                         _ => {},
//                     }
// 
//                     return SimState::Attacking;
//                 },
//                 SimState::Blocking => {
//                     return SimState::Blocking;
//                 },
//                 SimState::Flinching => {
//                     controller.time_in_state += dt;
// 
//                     if animator.current_animation != AnimationType::Flinch {
//                         return SimState::Flinching;
//                     }
// 
//                     if !kb.did_particles {
//                             let model_transform = Mat4::from_scale_rotation_translation(trans.scale, trans.rotation, trans.position);
//                             let skellington = em.skellingtons.get_mut(fac.key()).unwrap();
// 
//                             let bone_names: Vec<String> = {
//                                 let anim = animator.animations.get(&animator.current_animation).unwrap();
//                                 anim.model_animation_join.iter().map(|b| b.name.clone()).collect()
//                             };
// 
//                         let anim = animator.animations.get_mut(&animator.current_animation).unwrap();
// 
//                             for bone_name in bone_names{
//                                 if let Some(bone_world_model_space) = anim.get_raw_global_bone_transform_by_name(
//                                     &bone_name,
//                                     skellington,
//                                     Mat4::IDENTITY,
//                                 ) {
//                                     let bone_world_space = model_transform * bone_world_model_space;
//                                     let position = bone_world_space.w_axis.truncate();
// 
//                                     // You can randomize velocity or make it static for now
//                                     particles.spawn_oneshot_emitter(EmitterName::DamageBlood, position);
//                                 }
//                             }
//                         kb.did_particles = true;
//                     }
// 
// 
// 
//                     if kb.flinch && kb.ttl > 0.0 {
//                         animator.set_next_animation(AnimationType::Flinch);
//                         return SimState::Flinching;
//                     }
// 
//                     let anim = animator.get_current_animation().unwrap();
// 
//                     if anim.current_time >= anim.duration - ANIMATION_EPSILON {
//                         animator.set_next_animation(AnimationType::Run);
//                         return SimState::Aggro;
//                     }
// 
//                     return SimState::Flinching;
//                 },
//                 SimState::Dashing => {
//                     return SimState::Dashing;
//                 }
//             })();
// 
//             if input.is_down(Key::U) {
//                 dbg!(&next_state);
//                 dbg!(&controller.attack_state);
//                 dbg!(&animator.get_current_animation().unwrap().current_segment);
//                 dbg!(&animator.get_current_animation().unwrap().current_time);
//                 dbg!(&animator.get_current_animation().unwrap().duration);
//                 dbg!(&animator.get_current_animation().unwrap().duration - ANIMATION_EPSILON);
//                 dbg!(&animator.current_animation);
//                 dbg!(&animator.next_animation);
//             }
//             
// 
//             if input.is_down(Key::Y) {
//                 controller.state = SimState::Waiting;
// 
//                 for (_, anim) in animator.animations.iter_mut() {
//                     anim.current_time = 0.0;
//                 }
// 
//                 animator.set_next_animation(AnimationType::Idle);
//                 animator.set_current_animation(AnimationType::Idle);
//                 controller.attack_state = AttackState::Attack1;
//             } else {
//                 controller.state = next_state;
//             }
//         }
//     }
// }

