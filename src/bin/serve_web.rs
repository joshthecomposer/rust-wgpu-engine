use std::env;
use std::path::Path;
use std::process::Command;

const DIST_ZIP_PY: &str = r#"
import os, zipfile
root = os.environ["LEARN_OPENGL_WEB_ROOT"]
dist = os.path.join(root, "dist")
out = os.path.join(root, "dist.zip")
if not os.path.isdir(dist):
    raise SystemExit("missing dist/")
with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as z:
    for dirpath, _, filenames in os.walk(dist):
        for name in filenames:
            path = os.path.join(dirpath, name)
            arc = os.path.relpath(path, dist).replace(os.sep, "/")
            z.write(path, arc)
print("Wrote", os.path.abspath(out))
"#;

fn run_python_snippet(python: &str, code: &str, root: &str) -> bool {
    Command::new(python)
        .env("LEARN_OPENGL_WEB_ROOT", root)
        .args(["-c", code])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn write_dist_zip(root: &str) {
    eprintln!("(serve-web) writing dist.zip for itch…");
    let ok = run_python_snippet("python3", DIST_ZIP_PY, root)
        || run_python_snippet("python", DIST_ZIP_PY, root);
    if !ok {
        eprintln!("(serve-web) failed to write dist.zip (need python3/python with zipfile)");
        std::process::exit(1);
    }
}

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

    // write_dist_zip(root);

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
