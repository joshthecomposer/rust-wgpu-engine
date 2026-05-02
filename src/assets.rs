#![allow(dead_code)]

#[cfg(not(target_arch = "wasm32"))]
use std::{fs, path::Path};

use image::DynamicImage;

pub fn read_text(path: &str) -> std::io::Result<String> {
    let bytes = read_bytes(path)?;
    String::from_utf8(bytes).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("asset is not valid UTF-8: {path}: {error}"),
        )
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    fs::read(path)
}

#[cfg(target_arch = "wasm32")]
pub fn read_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("browser asset loading is not implemented yet: {path}"),
    ))
}

pub fn load_image(path: &str) -> Result<DynamicImage, String> {
    let bytes = read_bytes(path).map_err(|error| format!("Failed to read image {path}: {error}"))?;
    image::load_from_memory(&bytes).map_err(|error| format!("Failed to decode image {path}: {error}"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[cfg(target_arch = "wasm32")]
pub fn path_exists(_path: &str) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_text(path: &str, contents: &str) -> std::io::Result<()> {
    fs::write(path, contents)
}

#[cfg(target_arch = "wasm32")]
pub fn write_text(path: &str, _contents: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        format!("browser config writes are not implemented yet: {path}"),
    ))
}
