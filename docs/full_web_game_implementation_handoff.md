# Full Web Game Hookup Implementation Handoff

## Goal

Replace the old static `src/web_game.rs` browser scene with a wasm boot path that constructs the real desktop-style game stack:

- `Game::new`
- `World::new`
- ECS population from config
- Rapier physics
- renderer WebGL compatibility path
- browser input adapters
- animation/state/movement/combat/particles/UI module graph where feasible
- static `dist/` package for itch-style hosting

The original plan file at `C:\Users\jdwis\.cursor\plans\full_web_game_57ab9c52.plan.md` was not edited.

## Current Status

The codebase now compiles the real game module graph for wasm, the web boot constructs `Game`, and the browser smoke test reaches the rendered WebGL scene.

Confirmed after the texture and terrain asset fixes:

```bash
cargo check --target wasm32-unknown-unknown --no-default-features --features web
cargo check
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/build_web.ps1
```

Both cargo checks passed with existing warning noise, and the web package rebuilt successfully.

Latest browser smoke test URL:

```text
http://127.0.0.1:8002/index.html?terrain_asset_fix=1
```

Result:

- status shows `WebGL2 web game boot ready`
- skybox/background and world geometry render
- no new `RuntimeError: unreachable` after the terrain asset fix

## Main Changes Made

### Real Module Graph On Wasm

`src/main.rs` now includes the real game modules for wasm instead of gating most of them behind `not(target_arch = "wasm32")`.

This brings in:

- `game`
- `world`
- `entity_manager`
- `physics`
- `renderer`
- `input`
- `animation`
- `particles`
- `state_machines`
- `movement_system`
- `combat_system`
- `items`
- `spawn_system`
- `terrain`
- `ui`
- `sound`
- `config`

### Dependency Target Adjustments

`Cargo.toml` was adjusted conservatively, without upgrading versions:

- `winit`, `rand`, `rand_chacha`, and `rapier3d` are now available to wasm.
- `getrandom` is enabled for wasm via:
  - `.cargo/config.toml` with `getrandom_backend="wasm_js"`
  - target wasm dependency `getrandom = { version = "0.3.4", features = ["wasm_js"] }`
- Rapier `parallel` stays native-only because it failed on `wasm32-unknown-unknown`.
- Additional `web-sys` features were enabled for browser input and WebGL objects.

### Platform Adapter

`src/platform.rs` now aliases the wasm `Platform` type to `web_canvas::WebCanvasPlatform`:

```rust
#[cfg(target_arch = "wasm32")]
pub type Platform = web_canvas::WebCanvasPlatform;
```

`WebCanvasPlatform` now has `set_cursor_mode`, `swap_buffers`, and the fields/methods needed by `Game` and `Renderer`.

### Game Native API Split

`src/game.rs` now gates native-only imports and `handle_window_event` behind `not(target_arch = "wasm32")`.

Added wasm input adapter methods on `Game`:

- `handle_web_keyboard_input`
- `handle_web_mouse_move`
- `handle_web_mouse_button`
- `handle_web_scroll`

Settings application is split:

- Native path keeps window resize, fullscreen, vsync, and config writes.
- Wasm path applies runtime render settings only and avoids disk writes.

### Web Boot

`src/web_game.rs` now constructs the real game:

```rust
let mut config = GameConfig::load_or_create_default("config/game_config.json");
config.webgl_compatibility_mode = true;
let game = Game::new(platform, config);
```

`requestAnimationFrame` calls:

```rust
self.game.tick(elapsed);
```

It also installs browser keyboard/mouse/wheel handlers and logs on the first tick:

```text
real Game::tick running with ... populated transforms
```

Important: the old static web scene helpers are still present in `src/web_game.rs`, but they are no longer used. They currently produce dead-code warnings on wasm. Per project rules, no `#[allow(dead_code)]` was added.

`src/web_game.rs` also installs a small local panic hook, without adding dependencies, so browser console output includes Rust panic file/line/payload instead of only `RuntimeError: unreachable`.

### Config Loading

`Config::load_or_create_default` in `src/config/base_config.rs` now uses an in-memory default on wasm if the file is missing, instead of trying to write a default file.

### Asset Loading

`src/entity_manager.rs` terrain loading now uses `assets::load_image` instead of `image::open`, so wasm reads `resources/textures/small_terrain.png` from the browser preload map instead of unsupported filesystem IO.

### Static Web Package

`scripts/build_web.ps1` now:

- builds wasm
- runs `wasm-bindgen`
- copies `web/index.html`
- copies static startup asset roots:
  - `resources/shaders`
  - `resources/models/static`
  - `resources/models/animated`
  - `resources/textures`
  - `resources/fonts`
  - `resources/ui`
- copies top-level JSON files from `config`
- generates `dist/asset-manifest.json`

`web/index.html` now loads `asset-manifest.json` and preloads every listed asset into `window.__learn_opengl_rs_assets`.

## WebGL Shim Work Done

`src/platform.rs` WebGL shim was expanded for the real renderer path:

- framebuffers:
  - `glGenFramebuffers`
  - `glBindFramebuffer`
  - `glFramebufferTexture2D`
  - `glCheckFramebufferStatus`
- draw/read buffers:
  - `glDrawBuffers`
  - `glReadBuffer`
- render state:
  - `glDepthMask`
  - `glCullFace`
  - `glFrontFace`
- uniforms:
  - `glUniform2f`
  - `glUniform3f`
- vertex integer attrs:
  - `glVertexAttribIPointer`
- texture parameter float shim:
  - `glTexParameterfv` is currently a no-op because WebGL does not support the native border-color path; this is acceptable with `supports_clamp_to_border = false`.

## Browser Smoke Test Results

Server command used:

```bash
python -u -m http.server 8002 --bind 127.0.0.1 --directory "E:/Software_Dev/rust/rust-opengl-engine/dist"
```

URL:

```text
http://127.0.0.1:8002/index.html
```

First smoke test after initial hookup:

- assets loaded
- failed with `RuntimeError: null function or function signature mismatch`
- no `Game::tick` log
- no render

This was treated as missing WebGL shim functions. The framebuffer/state/uniform/integer-attribute shims above were added.

Second smoke test after shim additions:

- failed with `RuntimeError: unreachable`
- no `Game::tick` log
- no render
- repeated WebGL texture errors:
  - `GL_INVALID_OPERATION: glTexImage2DRobustANGLE: Invalid combination of format, type and internalFormat`
  - `GL_INVALID_OPERATION: glGenerateMipmap: Texture format does not support mipmap generation`

Likely cause found:

- `Renderer::upload_model_texture` uploaded `RGBA` pixel data with internal format `gl::SRGB8`.
- This was changed to `gl::SRGB8_ALPHA8`.

Follow-up smoke test with the panic hook showed:

```text
Rust panic at src\entity_manager.rs:1797:10: Failed to load terrain image: IoError(Error { kind: Unsupported, message: "operation not supported on this platform" })
```

Cause found:

- `load_terrain` still used `image::open`, which tries filesystem IO on wasm.
- It now uses `assets::load_image`.

Latest smoke test after rebuilding:

- status shows `WebGL2 web game boot ready`
- rendered scene is visible in the browser
- no new startup panic was reported

## Next Steps

1. Serve `dist/` if the existing server is not already running:

   ```bash
   python -u -m http.server 8002 --bind 127.0.0.1 --directory "E:/Software_Dev/rust/rust-opengl-engine/dist"
   ```

2. Reload `http://127.0.0.1:8002/index.html` and check console for:

   ```text
   real Game::tick running with ... populated transforms
   ```

3. Visually confirm:

   - skybox/background renders
   - populated world entities render, not only the old two weapon models
   - physics/ECS/animation state updates do not panic
   - browser input moves camera or toggles expected states

4. If it fails again with `RuntimeError: unreachable`, use the panic hook console line first. Likely remaining causes:

   - another asset/config parse panic
   - missing shader uniform/function shim
   - a `panic!`/`unwrap` in startup entity, terrain, animation, or UI loading
   - another WebGL-invalid texture format path

## Notes And Cautions

- Do not add `#[allow(dead_code)]`; the old web-scene helpers can be deleted later if desired, but they were intentionally left alone during this pass.
- Do not clean up unrelated warnings unless explicitly asked.
- The Python server is running outside the sandbox if it was left active.
- `scripts/build_web.ps1` needed `-NoProfile` and outside-sandbox permissions in Cursor on Windows to run correctly.
- The initial sandboxed PowerShell command returned immediately and did not create `dist/`.
- The generated `asset-manifest.json` had 400+ entries, so asset packaging is broad enough for startup, though later slimming is possible.
