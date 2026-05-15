use std::{cell::Cell, collections::HashMap};

#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
use std::ffi::CString;

use glam::Vec3;

use crate::{
    camera::Camera,
    command_buffer::{CommandBuffer, SoundKind},
    config::sound_config::SoundConfig,
    entity_manager::EntityManager,
    enums_types::SoundType,
    physics::PhysicsState,
};

#[cfg(all(target_arch = "wasm32", feature = "web_audio"))]
use wasm_bindgen::JsValue;

#[cfg(all(target_arch = "wasm32", feature = "web_audio"))]
use crate::sound::web_fmod_bridge;

#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
use crate::sound::fmod::{FMOD_Studio_EventDescription_LoadSampleData, FMOD_INIT_3D_RIGHTHANDED};

#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
use super::fmod::{
    FMOD_Studio_EventDescription_CreateInstance, FMOD_Studio_EventInstance_Release,
    FMOD_Studio_EventInstance_Set3DAttributes, FMOD_Studio_EventInstance_SetParameterByName,
    FMOD_Studio_EventInstance_Start, FMOD_Studio_EventInstance_Stop, FMOD_Studio_System_Create,
    FMOD_Studio_System_GetEvent, FMOD_Studio_System_Initialize, FMOD_Studio_System_LoadBankFile,
    FMOD_Studio_System_SetListenerAttributes, FMOD_Studio_System_Update, FMOD_3D_ATTRIBUTES,
    FMOD_INIT_NORMAL, FMOD_STUDIO_BANK, FMOD_STUDIO_EVENTDESCRIPTION, FMOD_STUDIO_EVENTINSTANCE,
    FMOD_STUDIO_INIT_NORMAL, FMOD_STUDIO_SYSTEM, FMOD_VECTOR, FMOD_VERSION,
};

#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
pub struct SoundData {
    description: FMOD_STUDIO_EVENTDESCRIPTION,
    // instance: FMOD_STUDIO_EVENTINSTANCE,
}

#[derive(Clone, Debug)]
pub struct OneShot {
    pub sound_type: SoundType,
    pub segment: u32,
    pub triggered: Cell<bool>,
}

#[derive(Clone, Debug)]
pub struct ContinuousSound {
    pub sound_type: SoundType,
    pub playing: Cell<bool>,
}

#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
pub struct SoundManager {
    pub fmod_system: FMOD_STUDIO_SYSTEM,
    pub sounds: HashMap<SoundType, SoundData>, //The key (String) is the sound_name in the game_config.json
    pub active_sounds: HashMap<SoundType, FMOD_STUDIO_EVENTINSTANCE>,
    pub active_3d_sounds: HashMap<usize, Vec<FMOD_STUDIO_EVENTINSTANCE>>,
    pub master_volume: f32,

    last_listener_pos: Option<Vec3>,
    last_entity_sound_pos: HashMap<usize, Vec3>,
}

#[cfg(all(
    feature = "native_audio",
    any(target_os = "macos", target_os = "windows"),
    not(target_arch = "wasm32")
))]
impl SoundManager {
    pub fn new(config: &SoundConfig) -> SoundManager {
        let sound_props = &config.sounds;

        let mut fmod_system: FMOD_STUDIO_SYSTEM = std::ptr::null_mut();
        let mut sounds = HashMap::new();

        unsafe {
            /************** INITIALIZE FMOD SYSTEM ******************/
            println!("FMOD VERSION IS {}", FMOD_VERSION);
            let result = FMOD_Studio_System_Create(&mut fmod_system, FMOD_VERSION);
            if result != 0 {
                panic!("FMOD System creation failed with error code {}", result);
            }

            let result = FMOD_Studio_System_Initialize(
                fmod_system,
                512,
                FMOD_STUDIO_INIT_NORMAL,
                FMOD_INIT_NORMAL | FMOD_INIT_3D_RIGHTHANDED,
                std::ptr::null_mut(),
            );
            if result != 0 {
                panic!(
                    "FMOD System initialization failed with error code {}",
                    result
                );
            }

            /************** LOAD BANK AND BANK STRINGS ******************/
            let bank_path =
                CString::new("resources/fmod/Desktop/Master.bank").expect("CString::new failed");
            let mut bank: FMOD_STUDIO_BANK = std::ptr::null_mut();
            let result =
                FMOD_Studio_System_LoadBankFile(fmod_system, bank_path.as_ptr(), 0, &mut bank);

            if result != 0 {
                panic!("FMOD Bank load failed with error code {}", result);
            }

            let strings_bank_path = CString::new("resources/fmod/Desktop/Master.strings.bank")
                .expect("CString::new failed");
            let mut strings_bank: FMOD_STUDIO_BANK = std::ptr::null_mut();
            let result = FMOD_Studio_System_LoadBankFile(
                fmod_system,
                strings_bank_path.as_ptr(),
                0,
                &mut strings_bank,
            );

            if result != 0 {
                panic!("FMOD Strings Bank load failed with error code {}", result);
            }

            /***************** CREATE EVENT DESC AND INSTANCES ****************/

            for (sound_name, path) in sound_props {
                let event_path = CString::new(path.as_str()).expect("CString::new failed");
                let mut description: FMOD_STUDIO_EVENTDESCRIPTION = std::ptr::null_mut();

                let result =
                    FMOD_Studio_System_GetEvent(fmod_system, event_path.as_ptr(), &mut description);
                if result != 0 {
                    panic!(
                        "Failed to load the event for {:?} with code {}",
                        sound_name, result
                    );
                }

                FMOD_Studio_EventDescription_LoadSampleData(description);
                sounds.insert(sound_name.clone(), SoundData { description });
            }
        }
        SoundManager {
            fmod_system,
            sounds,
            master_volume: 1.0,
            active_3d_sounds: HashMap::new(),
            active_sounds: HashMap::new(),

            last_listener_pos: None,
            last_entity_sound_pos: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        camera: &Camera,
        cmds: &mut CommandBuffer,
        em: &mut EntityManager,
        ps: &PhysicsState,
        dt: f32,
    ) {
        // Evaluate commands
        let soundcmds = std::mem::take(&mut cmds.sound);

        for c in soundcmds {
            match c.kind {
                SoundKind::Sound3d(t, p) => {
                    self.play_sound_3d(t, &p);
                }
                SoundKind::Sound2d(t) => {
                    self.play_sound_2d(t);
                }
                SoundKind::Sound3dContinuous(t, eid) => {
                    self.start_continuous_entity_sound_3d(t, eid, em);
                }
            }
        }

        self.set_listener_attributes(camera, dt);
        self.update_entity_continuous_sounds(em, ps, dt);

        unsafe {
            let result = FMOD_Studio_System_Update(self.fmod_system);
            if result != 0 {
                eprintln!("FMOD update failed with error code {}", result);
            }
        }

        //let count = self.get_instance_count(SoundType::Footstep);
        //dbg!(count);
    }

    // pub fn get_instance_count(&self, sound_type: SoundType) -> Option<i32> {
    //     let desc = self.sounds.get(&sound_type)?.description;
    //     let mut count: libc::c_int = 0;

    //     let r = unsafe { FMOD_Studio_EventDescription_GetInstanceCount(desc, &mut count) };
    //     if r != 0 {
    //         eprintln!("GetInstanceCount failed for {:?}: {}", sound_type, r);
    //         return None;
    //     }
    //     Some(count as i32)
    // }

    pub fn set_listener_attributes(&mut self, camera: &Camera, dt: f32) {
        let forward = camera.forward.normalize();
        let up = camera.up.normalize();

        let velocity = if dt > 0.0 {
            match self.last_listener_pos {
                Some(prev) => (camera.position - prev) / dt,
                None => Vec3::ZERO,
            }
        } else {
            Vec3::ZERO
        };

        self.last_listener_pos = Some(camera.position);

        let attributes = FMOD_3D_ATTRIBUTES {
            position: Self::opengl_to_fmod(camera.position),
            velocity: Self::opengl_to_fmod(velocity),
            forward: Self::opengl_to_fmod(forward),
            up: Self::opengl_to_fmod(up),
        };

        unsafe {
            let result = FMOD_Studio_System_SetListenerAttributes(
                self.fmod_system,
                0,
                &attributes,
                std::ptr::null(),
            );
            if result != 0 {
                eprintln!("Failed to set listener attributes: {}", result);
            }
        }
    }

    pub fn play_sound_3d(&mut self, sound_type: SoundType, position: &Vec3) {
        let sound_data = match self.sounds.get(&sound_type) {
            Some(data) => data,
            None => {
                eprintln!("Sound {} not found", sound_type);
                return;
            }
        };

        let mut instance: FMOD_STUDIO_EVENTINSTANCE = std::ptr::null_mut();
        unsafe {
            // Create a new instance each time
            let create_result =
                FMOD_Studio_EventDescription_CreateInstance(sound_data.description, &mut instance);
            if create_result != 0 {
                eprintln!("Failed to create event instance: {}", create_result);
                return;
            }

            let attributes = FMOD_3D_ATTRIBUTES {
                position: Self::opengl_to_fmod(*position),
                velocity: FMOD_VECTOR {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                forward: FMOD_VECTOR {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                up: FMOD_VECTOR {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            };

            let set_result = FMOD_Studio_EventInstance_Set3DAttributes(instance, &attributes);
            if set_result != 0 {
                eprintln!("Failed to set 3D attributes: {}", set_result);
            }

            let play_result = FMOD_Studio_EventInstance_Start(instance);
            if play_result != 0 {
                eprintln!("FMOD sound failed to start: {}", play_result);
            }

            //self.active_3d_sounds
            //    .entry(entity_id)
            //    .or_insert(Vec::new())
            //    .push(instance);

            // We don't wanna release on continuous sounds, so there should be a separate path for
            // continuous
            FMOD_Studio_EventInstance_Release(instance);
        }
    }

    pub fn start_continuous_entity_sound_3d(
        &mut self,
        sound_type: SoundType,
        entity_id: usize,
        em: &mut EntityManager,
    ) {
        if self.active_3d_sounds.contains_key(&entity_id) {
            println!("Already have this entity in the sounds");
            return;
        }
        let sound_data = match self.sounds.get(&sound_type) {
            Some(data) => data,
            None => {
                eprintln!("Sound {} not found", sound_type);
                return;
            }
        };

        let mut instance: FMOD_STUDIO_EVENTINSTANCE = std::ptr::null_mut();
        unsafe {
            // Create a new instance each time
            let create_result =
                FMOD_Studio_EventDescription_CreateInstance(sound_data.description, &mut instance);
            if create_result != 0 {
                eprintln!("Failed to create event instance: {}", create_result);
                return;
            }

            let trans = em.transforms.get(entity_id).unwrap();

            self.last_entity_sound_pos.insert(entity_id, trans.position);

            let attributes = FMOD_3D_ATTRIBUTES {
                position: Self::opengl_to_fmod(trans.position),
                velocity: FMOD_VECTOR {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                forward: FMOD_VECTOR {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                up: FMOD_VECTOR {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            };

            let set_result = FMOD_Studio_EventInstance_Set3DAttributes(instance, &attributes);
            if set_result != 0 {
                eprintln!("Failed to set 3D attributes: {}", set_result);
            }

            let play_result = FMOD_Studio_EventInstance_Start(instance);
            if play_result != 0 {
                eprintln!("FMOD sound failed to start: {}", play_result);
            }

            self.active_3d_sounds
                .entry(entity_id)
                .or_insert(Vec::new())
                .push(instance);
        }
    }

    pub fn update_entity_continuous_sounds(
        &mut self,
        em: &mut EntityManager,
        ps: &PhysicsState,
        dt: f32,
    ) {
        for (eid, sounds) in self.active_3d_sounds.iter() {
            let trans = em.transforms.get(*eid).unwrap();

            let velocity = match em
                .physics_handles
                .get(*eid)
                .and_then(|ph| ps.rigid_body_set.get(ph.rigid_body))
            {
                Some(rb) => {
                    let v = rb.linvel();
                    Vec3::new(v.x, v.y, v.z)
                }
                None => {
                    if dt > 0.0 {
                        match self.last_entity_sound_pos.get(eid) {
                            Some(prev) => (trans.position - *prev) / dt,
                            None => Vec3::ZERO,
                        }
                    } else {
                        Vec3::ZERO
                    }
                }
            };

            self.last_entity_sound_pos.insert(*eid, trans.position);

            for sound in sounds {
                let attributes = FMOD_3D_ATTRIBUTES {
                    position: Self::opengl_to_fmod(trans.position),
                    velocity: Self::opengl_to_fmod(velocity),
                    forward: FMOD_VECTOR {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    },
                    up: FMOD_VECTOR {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                };

                unsafe {
                    let set_result = FMOD_Studio_EventInstance_Set3DAttributes(*sound, &attributes);
                    if set_result != 0 {
                        eprintln!("Failed to set 3D attributes: {}", set_result);
                    }
                }
            }
        }
    }

    pub fn play_sound_2d(&mut self, sound_type: SoundType) {
        let sound_data = match self.sounds.get(&sound_type) {
            Some(data) => data,
            None => {
                eprintln!("Sound {} not found", sound_type);
                return;
            }
        };
        let mut instance: FMOD_STUDIO_EVENTINSTANCE = std::ptr::null_mut();
        unsafe {
            let create_result =
                FMOD_Studio_EventDescription_CreateInstance(sound_data.description, &mut instance);
            if create_result != 0 {
                eprintln!("Failed to create event instance: {}", create_result)
            }
            if self.active_sounds.contains_key(&sound_type) {
                return;
            }
            let play_result = FMOD_Studio_EventInstance_Start(instance);
            if play_result != 0 {
                eprintln!("FMOD sound failed to start: {}", play_result);
            }
            self.active_sounds.insert(sound_type, instance);
        }
    }

    pub fn stop_sound(&mut self, sound_type: &SoundType) {
        if let Some(instance) = self.active_sounds.get(sound_type) {
            unsafe {
                FMOD_Studio_EventInstance_Stop(
                    *instance,
                    super::fmod::FMOD_STUDIO_STOP_MODE::FMOD_STUDIO_STOP_IMMEDIATE,
                );
                FMOD_Studio_EventInstance_Release(*instance);
            }
        }
    }

    pub fn cleanup_entity_sounds(&mut self, entity_id: usize) {
        if let Some(instances) = self.active_3d_sounds.remove(&entity_id) {
            for instance in instances {
                unsafe {
                    FMOD_Studio_EventInstance_Stop(
                        instance,
                        super::fmod::FMOD_STUDIO_STOP_MODE::FMOD_STUDIO_STOP_IMMEDIATE,
                    );
                    FMOD_Studio_EventInstance_Release(instance);
                }
            }
        }
        self.last_entity_sound_pos.remove(&entity_id);
    }

    pub fn set_master_volume(&mut self, sound_type: &SoundType) {
        if let Some(instance) = self.active_sounds.get(sound_type) {
            let vol = CString::new("main_volume").unwrap();
            unsafe {
                let result = FMOD_Studio_EventInstance_SetParameterByName(
                    *instance,
                    vol.as_ptr(),
                    self.master_volume,
                    0,
                );
                if result != 0 {
                    println!("Updating volume failed with error code: {}", result);
                }
            }
        }
    }

    pub fn opengl_to_fmod(v: Vec3) -> FMOD_VECTOR {
        FMOD_VECTOR {
            x: v.x,
            y: v.y,
            z: -v.z,
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web_audio"))]
pub struct SoundManager {
    pub sounds: HashMap<SoundType, String>,
    pub active_sounds: HashMap<SoundType, ()>,
    pub active_3d_sounds: HashMap<usize, Vec<()>>,
    pub master_volume: f32,

    last_listener_pos: Option<Vec3>,
    last_entity_sound_pos: HashMap<usize, Vec3>,
}

#[cfg(all(target_arch = "wasm32", feature = "web_audio"))]
impl SoundManager {
    pub fn new(config: &SoundConfig) -> SoundManager {
        let sounds: HashMap<SoundType, String> = config
            .sounds
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut sound_paths = serde_json::Map::new();
        for (k, v) in &config.sounds {
            sound_paths.insert(k.to_string(), serde_json::Value::String(v.clone()));
        }
        let payload = serde_json::json!({
            "bankBase": "resources/fmod/Web",
            "sounds": sound_paths,
        })
        .to_string();
        web_fmod_bridge::call_bridge("init", &[JsValue::from_str(&payload)]);
        SoundManager {
            sounds,
            active_sounds: HashMap::new(),
            active_3d_sounds: HashMap::new(),
            master_volume: config.master_volume,
            last_listener_pos: None,
            last_entity_sound_pos: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        camera: &Camera,
        cmds: &mut CommandBuffer,
        em: &mut EntityManager,
        ps: &PhysicsState,
        dt: f32,
    ) {
        let soundcmds = std::mem::take(&mut cmds.sound);
        for c in soundcmds {
            match c.kind {
                SoundKind::Sound3d(t, p) => {
                    self.play_sound_3d(t, &p);
                }
                SoundKind::Sound2d(t) => {
                    self.play_sound_2d(t);
                }
                SoundKind::Sound3dContinuous(t, eid) => {
                    self.start_continuous_entity_sound_3d(t, eid, em);
                }
            }
        }
        if web_fmod_bridge::bridge_is_ready() {
            web_fmod_bridge::call_bridge("update", &[]);
            self.set_listener_attributes(camera, dt);
            self.update_entity_continuous_sounds(em, ps, dt);
        }
    }

    pub fn set_listener_attributes(&mut self, camera: &Camera, dt: f32) {
        if !web_fmod_bridge::bridge_is_ready() {
            return;
        }
        let forward = camera.forward.normalize();
        let up = camera.up.normalize();

        let velocity = if dt > 0.0 {
            match self.last_listener_pos {
                Some(prev) => (camera.position - prev) / dt,
                None => Vec3::ZERO,
            }
        } else {
            Vec3::ZERO
        };
        self.last_listener_pos = Some(camera.position);

        let p = Self::opengl_to_fmod_vec(camera.position);
        let v = Self::opengl_to_fmod_vec(velocity);
        let f = Self::opengl_to_fmod_vec(forward);
        let u = Self::opengl_to_fmod_vec(up);
        web_fmod_bridge::call_bridge(
            "setListener",
            &[
                JsValue::from_f64(f64::from(p.x)),
                JsValue::from_f64(f64::from(p.y)),
                JsValue::from_f64(f64::from(p.z)),
                JsValue::from_f64(f64::from(v.x)),
                JsValue::from_f64(f64::from(v.y)),
                JsValue::from_f64(f64::from(v.z)),
                JsValue::from_f64(f64::from(f.x)),
                JsValue::from_f64(f64::from(f.y)),
                JsValue::from_f64(f64::from(f.z)),
                JsValue::from_f64(f64::from(u.x)),
                JsValue::from_f64(f64::from(u.y)),
                JsValue::from_f64(f64::from(u.z)),
            ],
        );
    }

    fn opengl_to_fmod_vec(v: Vec3) -> Vec3 {
        Vec3::new(v.x, v.y, -v.z)
    }

    pub fn start_continuous_entity_sound_3d(
        &mut self,
        sound_type: SoundType,
        entity_id: usize,
        em: &mut EntityManager,
    ) {
        if self.active_3d_sounds.contains_key(&entity_id) {
            println!("Already have this entity in the sounds");
            return;
        }
        if !self.sounds.contains_key(&sound_type) {
            eprintln!("Sound {} not found", sound_type);
            return;
        }
        if !web_fmod_bridge::bridge_is_ready() {
            return;
        }

        let trans = em.transforms.get(entity_id).unwrap();
        self.last_entity_sound_pos.insert(entity_id, trans.position);

        let p = Self::opengl_to_fmod_vec(trans.position);
        let key = sound_type.to_string();
        web_fmod_bridge::call_bridge(
            "start3dContinuous",
            &[
                JsValue::from_str(&key),
                JsValue::from_f64(entity_id as f64),
                JsValue::from_f64(f64::from(p.x)),
                JsValue::from_f64(f64::from(p.y)),
                JsValue::from_f64(f64::from(p.z)),
                JsValue::from_f64(0.0),
                JsValue::from_f64(0.0),
                JsValue::from_f64(0.0),
            ],
        );

        self.active_3d_sounds
            .entry(entity_id)
            .or_insert_with(Vec::new)
            .push(());
    }

    pub fn update_entity_continuous_sounds(
        &mut self,
        em: &mut EntityManager,
        ps: &PhysicsState,
        dt: f32,
    ) {
        if !web_fmod_bridge::bridge_is_ready() {
            return;
        }
        for (eid, _instances) in self.active_3d_sounds.iter() {
            let trans = em.transforms.get(*eid).unwrap();

            // Prefer the rigid body's authoritative linear velocity. Sound updates run at render
            // rate while transforms only advance on physics ticks, so a position finite-difference
            // would alternate between zero (stationary frames) and inflated spikes (frames where
            // the physics tick happened) and break Doppler.
            let velocity = match em
                .physics_handles
                .get(*eid)
                .and_then(|ph| ps.rigid_body_set.get(ph.rigid_body))
            {
                Some(rb) => {
                    let v = rb.linvel();
                    Vec3::new(v.x, v.y, v.z)
                }
                None => {
                    if dt > 0.0 {
                        match self.last_entity_sound_pos.get(eid) {
                            Some(prev) => (trans.position - *prev) / dt,
                            None => Vec3::ZERO,
                        }
                    } else {
                        Vec3::ZERO
                    }
                }
            };

            self.last_entity_sound_pos.insert(*eid, trans.position);

            let p = Self::opengl_to_fmod_vec(trans.position);
            let v = Self::opengl_to_fmod_vec(velocity);
            web_fmod_bridge::call_bridge(
                "update3dContinuous",
                &[
                    JsValue::from_f64(*eid as f64),
                    JsValue::from_f64(f64::from(p.x)),
                    JsValue::from_f64(f64::from(p.y)),
                    JsValue::from_f64(f64::from(p.z)),
                    JsValue::from_f64(f64::from(v.x)),
                    JsValue::from_f64(f64::from(v.y)),
                    JsValue::from_f64(f64::from(v.z)),
                ],
            );
        }
    }

    pub fn play_sound_3d(&mut self, sound_type: SoundType, position: &Vec3) {
        if !self.sounds.contains_key(&sound_type) {
            eprintln!("Sound {} not found", sound_type);
            return;
        }
        if !web_fmod_bridge::bridge_is_ready() {
            return;
        }
        let p = Self::opengl_to_fmod_vec(*position);
        let key = sound_type.to_string();
        web_fmod_bridge::call_bridge(
            "play3d",
            &[
                JsValue::from_str(&key),
                JsValue::from_f64(f64::from(p.x)),
                JsValue::from_f64(f64::from(p.y)),
                JsValue::from_f64(f64::from(p.z)),
            ],
        );
    }

    pub fn play_sound_2d(&mut self, sound_type: SoundType) {
        if !self.sounds.contains_key(&sound_type) {
            eprintln!("Sound {} not found", sound_type);
            return;
        }
        if self.active_sounds.contains_key(&sound_type) {
            return;
        }
        if web_fmod_bridge::bridge_is_ready() {
            web_fmod_bridge::call_bridge("play2d", &[JsValue::from_str(&sound_type.to_string())]);
            self.active_sounds.insert(sound_type, ());
        }
    }

    pub fn stop_sound(&mut self, sound_type: &SoundType) {
        if self.active_sounds.contains_key(sound_type) {
            web_fmod_bridge::call_bridge("stop2d", &[JsValue::from_str(&sound_type.to_string())]);
            self.active_sounds.remove(sound_type);
        }
    }

    pub fn cleanup_entity_sounds(&mut self, entity_id: usize) {
        web_fmod_bridge::call_bridge("cleanupEntity3d", &[JsValue::from_f64(entity_id as f64)]);
        self.active_3d_sounds.remove(&entity_id);
        self.last_entity_sound_pos.remove(&entity_id);
    }

    pub fn set_master_volume(&mut self, sound_type: &SoundType) {
        if !web_fmod_bridge::bridge_is_ready() {
            return;
        }
        web_fmod_bridge::call_bridge(
            "setMasterVolumeFor2d",
            &[
                JsValue::from_str(&sound_type.to_string()),
                JsValue::from_f64(f64::from(self.master_volume)),
            ],
        );
    }
}

#[cfg(not(any(
    all(
        feature = "native_audio",
        any(target_os = "macos", target_os = "windows"),
        not(target_arch = "wasm32")
    ),
    all(target_arch = "wasm32", feature = "web_audio")
)))]
pub struct SoundManager {
    pub active_sounds: HashMap<SoundType, ()>,
    pub active_3d_sounds: HashMap<usize, Vec<()>>,
    pub master_volume: f32,
}

#[cfg(not(any(
    all(
        feature = "native_audio",
        any(target_os = "macos", target_os = "windows"),
        not(target_arch = "wasm32")
    ),
    all(target_arch = "wasm32", feature = "web_audio")
)))]
impl SoundManager {
    pub fn new(_config: &SoundConfig) -> SoundManager {
        SoundManager {
            active_3d_sounds: HashMap::new(),
            active_sounds: HashMap::new(),
            master_volume: 1.0,
        }
    }

    pub fn update(
        &mut self,
        _camera: &Camera,
        cmds: &mut CommandBuffer,
        _em: &mut EntityManager,
        _ps: &PhysicsState,
        _dt: f32,
    ) {
        let _ = std::mem::take(&mut cmds.sound);
    }

    pub fn set_listener_attributes(&self, _camera: &Camera, _dt: f32) {}

    pub fn play_sound_3d(&mut self, _sound_type: SoundType, _position: &Vec3) {}

    pub fn play_sound_2d(&mut self, sound_type: SoundType) {
        self.active_sounds.insert(sound_type, ());
    }

    pub fn stop_sound(&mut self, sound_type: &SoundType) {
        self.active_sounds.remove(sound_type);
    }

    pub fn cleanup_entity_sounds(&mut self, entity_id: usize) {
        self.active_3d_sounds.remove(&entity_id);
    }

    pub fn set_master_volume(&mut self, _sound_type: &SoundType) {}
}
