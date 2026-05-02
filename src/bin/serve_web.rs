//! Build `dist/` (same as `scripts/build_web.{ps1,sh}`) and serve it with `python -m http.server`.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let root = env!("CARGO_MANIFEST_DIR");
    let port = env::args().nth(1).unwrap_or_else(|| "8080".into());

    eprintln!("(serve-web) building dist/…");
    let build_ok = {
        #[cfg(windows)]
        {
            Command::new("powershell.exe")
                .args([
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    "scripts/build_web.ps1",
                ])
                .current_dir(root)
                .status()
        }
        #[cfg(not(windows))]
        {
            Command::new("bash")
                .arg("scripts/build_web.sh")
                .current_dir(root)
                .status()
        }
    };
    match build_ok {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("(serve-web) build script exited with {:?}", s.code());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("(serve-web) failed to run build script: {e}");
            std::process::exit(1);
        }
    }

    let dist = Path::new(root).join("dist");
    if !dist.is_dir() {
        eprintln!("(serve-web) missing dist/ at {:?}", dist);
        std::process::exit(1);
    }

    eprintln!(
        "(serve-web) serving {:?} at http://127.0.0.1:{port}/  (Ctrl+C to stop)",
        dist
    );

    let http = |cmd: &str| {
        Command::new(cmd)
            .args(["-m", "http.server", &port, "--directory"])
            .arg(&dist)
            .current_dir(root)
            .status()
    };

    let st = http("python3")
        .or_else(|_| http("python"))
        .expect("(serve-web) failed to spawn python");

    if !st.success() {
        eprintln!("(serve-web) python http.server exited with {:?}", st.code());
        std::process::exit(1);
    }
}
