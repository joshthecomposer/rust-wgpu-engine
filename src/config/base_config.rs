use crate::assets;
use serde::{Deserialize, Serialize};
use serde_json;

/// A trait for configuration types that can be loaded from and saved to JSON files.
pub trait Config: Default + for<'de> Deserialize<'de> + Serialize + Sized {
    /// Loads and deserializes data from a JSON file.
    ///
    /// # Arguments
    /// * `file_name` - Path to the JSON file to load
    ///
    /// # Panics
    /// Panics if the file cannot be read or if deserialization fails.
    fn load_from_file(file_name: &str) -> Self {
        println!(
            "loading {} from {}",
            std::any::type_name::<Self>(),
            file_name
        );
        let config_str = assets::read_text(file_name).unwrap();

        serde_json::from_str(&config_str).expect("The file was missing or malformed")
    }

    /// Serializes and saves data to a JSON file.
    ///
    /// # Arguments
    /// * `file_name` - Path to the JSON file to save to
    ///
    /// # Panics
    /// Panics if serialization fails or if the file cannot be written.
    fn save_to_file(&self, file_name: &str) {
        println!("saving {} to {}", std::any::type_name::<Self>(), file_name);
        let json_string = serde_json::to_string_pretty(self).expect("Failed to serialize data");
        assets::write_text(file_name, &json_string).expect("Failed to write data");

        println!(
            "Completed writing {} to {}",
            std::any::type_name::<Self>(),
            file_name
        );
    }

    /// Loads configuration from a file if it exists, otherwise creates and saves a default configuration.
    ///
    /// # Arguments
    /// * `file_name` - Path to the configuration file
    ///
    /// # Returns
    /// The loaded configuration if the file exists, otherwise a newly created default configuration.
    ///
    /// # Panics
    /// Panics if the file cannot be read, deserialization fails, or if writing the default config fails.
    fn load_or_create_default(file_name: &str) -> Self {
        if assets::path_exists(file_name) {
            println!("Config file found at {}, loading data", file_name);
            Self::load_from_file(file_name)
        } else {
            #[cfg(target_arch = "wasm32")]
            {
                println!(
                    "Config file not found at {}, using in-memory default for web",
                    file_name
                );
                return Self::default();
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                println!(
                    "Config file not found at {}, creating default config",
                    file_name
                );
                let config = Self::default();
                config.save_to_file(file_name);
                config
            }
        }
    }
}
