use std::{env, fs::{self, OpenOptions}, path::Path, process::Command};

fn main() {
    // Link against the GLFW static library
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-search=native=libs");
        println!("cargo:rustc-link-lib=static=libclang");
    }
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-search=native=libs");
        println!("cargo:rustc-link-lib=dylib=clang");
        println!("cargo:rustc-link-lib=static=glfw3");
        println!("cargo:rustc-link-lib=dylib=assimp");
    }

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

}

