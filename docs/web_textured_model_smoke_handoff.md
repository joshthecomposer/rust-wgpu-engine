# Web Textured Model Smoke Handoff

## Goal

Advance the current web smoke scene from rendering a real vertex-colored mesh to rendering a real textured model from `dist/resources/...`, without booting the full desktop `Game` yet.

## Current Working State

- `dist/index.html` loads the Rust/WASM bundle.
- `web/index.html` preloads `resources/models/static/weapons/swords/001_orc_sword_bc.txt`.
- `src/assets.rs` can read preloaded browser assets on wasm.
- `src/main.rs` parses that WiseModel mesh, uploads VAO/VBO/EBO, and renders the Orc Sword with vertex colors.
- `scripts/build_web.ps1` copies the model file into `dist/resources/...`.
- Chrome smoke test rendered the Orc Sword with status `WebGL2 model smoke ready`.

## Prior Art

- Bevy web assets load from static relative URLs in web builds: https://bevy.org/examples/assets/web-asset/
- Bevy wasm folder-loading issues show why explicit asset manifests are safer than directory scans: https://github.com/bevyengine/bevy/issues/2916
- Fyrox resource management reinforces explicit model/texture dependencies and loading state: https://fyrox-book.github.io/resources/resources.html

## Implementation Plan

1. Package a textured model and texture asset.
   - Inspect `resources/models/static/weapons/swords/001_orc_sword.txt` and its referenced `low_poly_tex.png`.
   - Prefer this over `001_orc_sword_bc.txt` because the current `_bc` file is color-first and references `meme.dream`.
   - Update `scripts/build_web.ps1` to copy the mesh and texture into `dist/resources/...`.

2. Extend the web asset manifest.
   - Add the mesh and texture paths to `assetManifest` in `web/index.html`.
   - Keep explicit paths; do not rely on web directory listing.
   - Continue preloading into `window.__learn_opengl_rs_assets`.

3. Add minimal wasm texture upload support.
   - Reuse `assets::load_image()` if image decoding works cleanly for wasm.
   - Add only the needed WebGL shims in `src/platform.rs`, likely:
     - `glGenTextures`
     - `glBindTexture`
     - `glTexImage2D`
     - `glTexParameteri`
     - `glActiveTexture`
     - `glUniform1i`
     - maybe `glGenerateMipmap`
   - Keep all web shims under `#[cfg(target_arch = "wasm32")]`.

4. Add or adapt a small textured web shader.
   - Inputs: position, normal, uv, base color.
   - Uniforms: `projection`, `view`, `model`, `elapsed`, `sampler2D diffuse_texture`, `bool use_texture`.
   - Fragment behavior: sample diffuse texture when available; otherwise fall back to base color.

5. Render the textured model.
   - Parse `TEXTURE_DIFFUSE`.
   - Resolve relative texture paths from the model file directory.
   - Upload the texture once in `WebRenderSmokeScene::new`.
   - Bind the texture before drawing.
   - Preserve native desktop behavior by default.

## Validation

After each meaningful slice:

```bash
cargo check --target wasm32-unknown-unknown --no-default-features --features web
cargo check
```

Final package build:

```bash
powershell.exe -ExecutionPolicy Bypass -File scripts/build_web.ps1
```

Serve locally:

```bash
python -u -m http.server 8002 --bind 127.0.0.1 --directory "E:/Software_Dev/rust/rust-opengl-engine/dist"
```

Open:

```text
http://127.0.0.1:8002/index.html
```

## Success Criteria

- `dist/` contains `index.html`, wasm/js files, the model file, and the texture file.
- Chrome shows the Orc Sword textured, not only vertex-colored.
- Status text reaches something like `WebGL2 textured model smoke ready`.
- Native `cargo check` still passes with only pre-existing warnings.
- Do not add `#[allow(dead_code)]`; ignore dead code diagnostics unless explicitly asked to clean them up.
