#![allow(non_camel_case_types)]
extern crate libc;
use libc::{c_char, c_int, c_uint, c_void};

pub type FMOD_STUDIO_SYSTEM = *mut c_void;
pub type FMOD_STUDIO_BANK = *mut c_void;
pub type FMOD_STUDIO_EVENTDESCRIPTION = *mut c_void;
pub type FMOD_RESULT = c_int;
pub const FMOD_STUDIO_INIT_NORMAL: u32 = 0;
pub const FMOD_INIT_3D_RIGHTHANDED: u32 = 0x00000010;
pub const FMOD_INIT_NORMAL: u32 = 0;
pub const FMOD_VERSION: u32 = 0x00020214;
pub type FMOD_STUDIO_EVENTINSTANCE = *mut libc::c_void;
pub const FMOD_DEBUG_LEVEL_LOG: u32 = 0x00000001;

#[repr(C)]
pub enum FMOD_STUDIO_STOP_MODE {
    FMOD_STUDIO_STOP_IMMEDIATE = 1,
    FMOD_STUDIO_STOP_ALLOWFADEOUT = 2,
}

#[repr(C)]
#[derive(Debug)]
pub struct FMOD_VECTOR {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(Debug)]
pub struct FMOD_3D_ATTRIBUTES {
    pub position: FMOD_VECTOR,
    pub velocity: FMOD_VECTOR,
    pub forward: FMOD_VECTOR,
    pub up: FMOD_VECTOR,
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[link(name = "fmodstudio")]
extern "C" {

    pub fn FMOD_Debug_Initialize(flags: u32, mode: u32, file: *const c_char) -> FMOD_RESULT;

    pub fn FMOD_Studio_System_Update(system: FMOD_STUDIO_SYSTEM) -> FMOD_RESULT;

    pub fn FMOD_Studio_System_Create(
        system: *mut FMOD_STUDIO_SYSTEM,
        headerversion: c_uint,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_System_Initialize(
        system: FMOD_STUDIO_SYSTEM,
        maxchannels: libc::c_int,
        studioflags: libc::c_uint,
        studioextraflags: libc::c_uint,
        extradriverdata: *mut c_void,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_System_LoadBankFile(
        system: FMOD_STUDIO_SYSTEM,
        filename: *const c_char,
        flags: c_int,
        bank: *mut FMOD_STUDIO_BANK,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_System_GetEvent(
        system: FMOD_STUDIO_SYSTEM,
        path: *const c_char,
        event: *mut FMOD_STUDIO_EVENTDESCRIPTION,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_EventDescription_CreateInstance(
        eventDescription: FMOD_STUDIO_EVENTDESCRIPTION,
        eventInstance: *mut FMOD_STUDIO_EVENTINSTANCE,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_EventInstance_Start(eventInstance: FMOD_STUDIO_EVENTINSTANCE)
        -> FMOD_RESULT;

    pub fn FMOD_Studio_EventInstance_Release(
        eventInstance: FMOD_STUDIO_EVENTINSTANCE,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_EventInstance_Stop(
        eventInstance: FMOD_STUDIO_EVENTINSTANCE,
        mode: FMOD_STUDIO_STOP_MODE,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_EventInstance_SetParameterByName(
        event: FMOD_STUDIO_EVENTINSTANCE,
        name: *const c_char,
        value: f32,
        ignoreseekspeed: u8,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_EventDescription_LoadSampleData(
        description: FMOD_STUDIO_EVENTDESCRIPTION,
    ) -> FMOD_RESULT;

    //
    // 3D stuff
    //
    pub fn FMOD_Studio_EventInstance_Set3DAttributes(
        event: FMOD_STUDIO_EVENTINSTANCE,
        attributes: *const FMOD_3D_ATTRIBUTES,
    ) -> FMOD_RESULT;

    pub fn FMOD_Studio_System_SetListenerAttributes(
        system: FMOD_STUDIO_SYSTEM,
        listener: c_int,
        attributes: *const FMOD_3D_ATTRIBUTES,
        attenuationposition: *const FMOD_VECTOR,
    ) -> FMOD_RESULT;
}
