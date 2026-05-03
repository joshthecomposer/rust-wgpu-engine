# Rendering Phase 6 Accelerated WebGL Handoff

This handoff captures the current state after the accelerated compatibility
push. The goal shifted from small renderer-only slices to getting a native
WebGL-compatible world render path working before starting real browser build
work.

## Current State

The compatibility path now attempts to render real world content in native
`webgl_compatibility_mode` instead of only clearing the screen and drawing the
widget gallery.

Key changes in this pass:

- `src/renderer.rs`
  - Added `ShaderProfile::GlslEs300` world shader loading for compatibility
  mode.
  - `Renderer::new_webgl_compatibility` now creates only the resources needed
  by the compatibility world path:
    - ES depth, skybox, static model, and animated model shaders.
    - Skybox VAO/cubemap.
    - WebGL-friendlier shadow depth map using `DEPTH_COMPONENT24` /
    `UNSIGNED_INT`.
  - Added `render_world_webgl_compatibility`, a slim path that renders to the
  default framebuffer and skips HDR, bloom, MSAA, FXAA, particles, ImGui, and
  post-processing.
  - Compatibility model drawing includes static and animated models. Animated
  models use the existing `bone_transforms` uniform upload.
  - Compatibility shadows are disabled in the ES model shaders via
  `use_shadows = false`.
  - Default material textures are rebound per model draw in the compatibility
  path so models with incomplete material sets do not inherit stale bindings.
- `src/game.rs`
  - Compatibility mode now calls `render_world_webgl_compatibility`.
  - UI is temporarily disabled behind `const UI_ENABLED: bool = false;`
  because the widget gallery is currently bugged.
  - ImGui creation, UI loading, UI layout/update, and UI rendering are all
  skipped while that flag is false.
- `src/shaders.rs`
  - `ShaderProfile::GlslEs300` maps the existing logical shader paths to new
  ES shader variants.
  - Shader reads now go through the new `assets` module.
- `src/assets.rs`
  - New minimal native filesystem-backed read boundary:
    - `read_text`
    - `path_exists`
  - This is only a first step. Most runtime asset loading still uses direct
  filesystem paths.
- `src/config/base_config.rs`
  - Config reads now use `assets::read_text`.
  - Config existence checks now use `assets::path_exists`.
  - Config writes still use direct filesystem writes.
- `src/platform.rs`
  - Added initial `PlatformBackend` scaffolding with `NativeGlutin` and
  `WebCanvas` variants.
  - Current runtime still uses the native glutin/winit platform only.
- New ES shader files:
  - `resources/shaders/depth_shader_es300.glsl`
  - `resources/shaders/skybox_es300.glsl`
  - `resources/shaders/model/static_model_es300.glsl`
  - `resources/shaders/model/animated_model_es300.glsl`

## Important Behavior Notes

- Compatibility rendering is not expected to match desktop visuals yet.
- The desktop path still uses HDR, bloom, MSAA/FXAA, particles, and full
post-processing.
- The compatibility path renders directly to the default framebuffer and skips
tonemapping/exposure, so dark textures look darker than desktop.
- Terrain appears very dark in compatibility mode because it uses
`resources/models/static/terrain/ai_slop/dark_dirt_pixelated.png`, and the
debug lighting boost that made it brighter was intentionally reverted.
- Characters and weapons were reported as visually acceptable in compatibility
mode.
- The temporary UI disable is intentional. Do not assume the UI system was
removed.

## Reverted/Not Kept

A short-lived `compatibility_lighting_fallback` uniform was added to brighten
very dark materials, then removed at the user's request. Do not reintroduce it
unless explicitly requested.

The remaining ES shader compatibility-specific visual change is:

- `use_shadows = false` for compatibility model shaders.
- Ambient floor remains at `vec3(0.28)` in the ES model shaders.

## Known Gaps

- No real browser/WebGL build exists yet.
- `src/platform.rs` is still native glutin/winit only.
- Cargo features are not split for browser yet.
- FMOD audio is still native-only and likely web-hostile.
- ImGui/editor tooling is still native-oriented, though runtime creation is
currently disabled through `UI_ENABLED`.
- Asset loading is only partially abstracted. Many paths still use direct
filesystem APIs:
  - model imports
  - texture loading
  - skybox image loading
  - RON UI loading
  - fonts
  - particle/config data
- Config writes still assume a writable local filesystem.
- Animated model ES shaders still use `uniform mat4 bone_transforms[100]`.
This may hit WebGL uniform limits and should be tested in an actual browser
context.
- The compatibility path still creates a shadow map for the depth pass even
though model shadow sampling is disabled. This is acceptable for now but can
be simplified if it blocks WebGL.

## Validation Already Run

After the accelerated compatibility work and follow-up changes:

- `cargo fmt`
- `cargo check`
- Cursor lints on edited files

The final lint checks for the edited renderer/shader/game files reported no
introduced errors. Dead-code-only warnings were explicitly deprioritized by the
user.

## Suggested Next Step

Start the real web-build preparation work. Recommended PR:

### Web Build Prep: Features, Asset Boundary, Platform Split

1. Add target/feature scaffolding for browser builds without adding new
  dependencies unless the user explicitly approves it.
2. Gate or stub native-only systems for web:
  - FMOD audio
  - ImGui/editor UI
  - native filesystem writes
3. Broaden the `assets` boundary beyond shaders/config:
  - UI RON files
  - fonts
  - model files
  - textures
  - skybox faces
  - particle/config JSON
4. Split platform implementation:
  - keep the current glutin/winit path as native
  - add a `cfg(target_arch = "wasm32")` placeholder for a future canvas/WebGL2
  path
5. Attempt a first `wasm32` or Emscripten build and use compiler errors as the
  next task list.

## Suggested Guardrails For The Next Agent

- Do not chase desktop visual parity yet.
- Do not re-enable the widget gallery until the user asks.
- Keep `webgl_compatibility_mode` useful as the native test harness.
- Prefer capability/feature gates over platform-name checks.
- Do not add, remove, or update dependencies without explicit user approval.
- Do not edit plan files.
- Do not revert unrelated dirty-tree changes.