use std::{path::Path, process::Command};

fn main() {
    println!("cargo:rustc-link-search=native=libs");

    #[cfg(target_os = "windows")]
    {
        let script_path = "copy_files.bat";
        if !Path::new(script_path).exists() {
            panic!("Script not found: {}", script_path);
        }
        let status = Command::new("cmd")
            .args(["/C", script_path])
            .status()
            .expect("Failed to execute batch script");
        if !status.success() {
            panic!("Batch script failed with status: {:?}", status);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let script_path = "./copy_files.sh";
        if !Path::new(script_path).exists() {
            panic!("Script not found: {}", script_path);
        }
        let status = Command::new("bash")
            .arg(script_path)
            .status()
            .expect("Failed to execute bash script");
        if !status.success() {
            panic!("Script failed with status: {:?}", status);
        }
    }

    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    // engine_ui.slint now re-exports GameUI from game_ui.slint, so both are available
    slint_build::compile_with_config("resources/ui/engine_ui.slint", config).unwrap();
}
