#![allow(dead_code)]

use std::{cell::Cell, collections::HashMap, ffi::CString};

use glam::Vec3;

use crate::{camera::Camera, config::game_config::GameConfig, enums_types::SoundType, sound::fmod::{FMOD_Studio_EventDescription_LoadSampleData, FMOD_INIT_3D_RIGHTHANDED}};

use super::fmod::{FMOD_Studio_EventDescription_CreateInstance, FMOD_Studio_EventInstance_Release, FMOD_Studio_EventInstance_Set3DAttributes, FMOD_Studio_EventInstance_SetParameterByName, FMOD_Studio_EventInstance_Start, FMOD_Studio_EventInstance_Stop, FMOD_Studio_System_Create, FMOD_Studio_System_GetEvent, FMOD_Studio_System_Initialize, FMOD_Studio_System_LoadBankFile, FMOD_Studio_System_SetListenerAttributes, FMOD_Studio_System_Update, FMOD_3D_ATTRIBUTES, FMOD_INIT_NORMAL, FMOD_STUDIO_BANK, FMOD_STUDIO_EVENTDESCRIPTION, FMOD_STUDIO_EVENTINSTANCE, FMOD_STUDIO_INIT_NORMAL, FMOD_STUDIO_SYSTEM, FMOD_VECTOR, FMOD_VERSION};

pub struct SoundData {
    description: FMOD_STUDIO_EVENTDESCRIPTION,
    // instance: FMOD_STUDIO_EVENTINSTANCE,
}

#[derive(Clone, Debug)]
pub struct SoundTrigger {
    pub sound_type: SoundType,
    pub frame: usize,
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

pub struct SoundManager {
    pub fmod_system: FMOD_STUDIO_SYSTEM,
    pub sounds: HashMap<SoundType, SoundData>, //The key (String) is the sound_name in the game_config.json
    pub active_sounds: HashMap<SoundType, FMOD_STUDIO_EVENTINSTANCE>,
    pub active_3d_sounds: HashMap<usize, Vec<FMOD_STUDIO_EVENTINSTANCE>>,
    pub playing_bg: bool,
    pub master_volume: f32,
} 

impl SoundManager {
    pub fn new(config: &GameConfig) -> SoundManager {
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
                panic!("FMOD System initialization failed with error code {}", result);
            }

            /************** LOAD BANK AND BANK STRINGS ******************/
            let bank_path = CString::new("resources/fmod/Desktop/Master.bank").expect("CString::new failed");
            let mut bank: FMOD_STUDIO_BANK = std::ptr::null_mut();
            let result = FMOD_Studio_System_LoadBankFile(
                fmod_system,
                bank_path.as_ptr(),
                0,
                &mut bank,
            );

            if result != 0 {
                panic!("FMOD Bank load failed with error code {}", result);
            }

            let strings_bank_path = CString::new("resources/fmod/Desktop/Master.strings.bank").expect("CString::new failed");
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

                let result = FMOD_Studio_System_GetEvent(
                    fmod_system, 
                    event_path.as_ptr(), 
                    &mut description
                );
                if result != 0 {
                    panic!("Failed to load the event for {:?} with code {}", sound_name, result);
                }

                FMOD_Studio_EventDescription_LoadSampleData(description);
                sounds.insert(sound_name.clone(), SoundData {
                    description, 
                });
            }
        }
        SoundManager {
            fmod_system,
            sounds,
            playing_bg: false,
            master_volume: 1.0,
            active_3d_sounds: HashMap::new(),
            active_sounds: HashMap::new(),
        }

    }

    pub fn update(&self, camera: &Camera) {
        unsafe {
            let result = FMOD_Studio_System_Update(self.fmod_system);
            if result != 0 {
                eprintln!("FMOD update failed with error code {}", result);
            }
        }
        self.set_listener_attributes(camera);
    }

    pub fn set_listener_attributes(&self, camera: &Camera) {
        let forward = camera.forward.normalize();
        let up = camera.up.normalize();
        let attributes = FMOD_3D_ATTRIBUTES {
            position: Self::opengl_to_fmod(camera.position),
            velocity: FMOD_VECTOR {
                x: 0.0, 
                y: 0.0,
                z: 0.0,
            },
            forward: Self::opengl_to_fmod(forward),
            up: Self::opengl_to_fmod(up),
        };
        
        unsafe {
            let result = FMOD_Studio_System_SetListenerAttributes(self.fmod_system, 0, &attributes, std::ptr::null());
            if result != 0 {
                eprintln!("Failed to set listener attributes: {}", result);
            }
        }
    }


    pub fn play_sound_3d(&mut self, sound_type: SoundType, position: &Vec3, entity_id: usize) {
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
            let create_result = FMOD_Studio_EventDescription_CreateInstance(
                sound_data.description, 
                &mut instance
            );
            if create_result != 0 {
                eprintln!("Failed to create event instance: {}", create_result);
                return;
            }

            let attributes = FMOD_3D_ATTRIBUTES {
                position: Self::opengl_to_fmod(*position),
                velocity: FMOD_VECTOR { x: 0.0, y: 0.0, z: 0.0 },
                forward: FMOD_VECTOR { x: 0.0, y: 0.0, z: 1.0 },
                up: FMOD_VECTOR { x: 0.0, y: 1.0, z: 0.0 }
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

            // FMOD_Studio_EventInstance_Release(instance);
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
            let create_result = FMOD_Studio_EventDescription_CreateInstance(
                sound_data.description, 
                &mut instance
            );
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

    pub fn stop_sound(&mut self, sound_type: &SoundType){
        if let Some(instance) = self.active_sounds.get(sound_type) {
            unsafe {
                FMOD_Studio_EventInstance_Stop(*instance, super::fmod::FMOD_STUDIO_STOP_MODE::FMOD_STUDIO_STOP_IMMEDIATE);
                FMOD_Studio_EventInstance_Release(*instance);
            }
        }
    }

    pub fn cleanup_entity_sounds(&mut self, entity_id: usize) {
        if let Some(instances) = self.active_3d_sounds.remove(&entity_id) {
            for instance in instances {
                unsafe {
                    FMOD_Studio_EventInstance_Stop(instance, super::fmod::FMOD_STUDIO_STOP_MODE::FMOD_STUDIO_STOP_IMMEDIATE);
                    FMOD_Studio_EventInstance_Release(instance);
                }
            }
        }
    }

    pub fn set_master_volume(&mut self, sound_type: &SoundType) {
        if let Some(instance) = self.active_sounds.get(sound_type) {
            let vol = CString::new("main_volume").unwrap();
            unsafe {
                let result = FMOD_Studio_EventInstance_SetParameterByName(*instance, vol.as_ptr(), self.master_volume, 0);
                if result != 0 {
                    println!("Updating volume failed with error code: {}", result);
                }
            }
        }
    }

    pub fn opengl_to_fmod(v: Vec3) -> FMOD_VECTOR {
        FMOD_VECTOR { x: v.x, y: v.y, z: -v.z }
    }
}
