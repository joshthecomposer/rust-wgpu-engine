//! Cache for Slint images to avoid redundant disk I/O.
//! Images are indexed by their file path.

use slint::Image;
use std::collections::HashMap;

/// Simple cache for Slint images loaded from disk.
#[derive(Default)]
pub struct UiImageCache {
    cache: HashMap<String, Image>,
}

impl UiImageCache {
    /// Create a new empty image cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get an image from the cache, or load it from disk if not present.
    /// Returns a default (empty) image if loading fails.
    pub fn get(&mut self, path: &str) -> Image {
        if path.is_empty() {
            return Image::default();
        }

        if let Some(img) = self.cache.get(path) {
            return img.clone();
        }

        // load the image from disk
        let img = match Image::load_from_path(std::path::Path::new(path)) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Failed to load UI image from {}: {}", path, e);
                Image::default()
            }
        };

        self.cache.insert(path.to_string(), img.clone());
        img
    }
}
