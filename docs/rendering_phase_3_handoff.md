# Rendering Centralization Phase 3 Handoff

Phase 3 centralized UI texture creation and updates behind renderer-owned helper
APIs while preserving current UI behavior. This is still an OpenGL renderer; it
is not a WebGL, OpenGL ES, or wgpu migration.

## What Changed

- `src/renderer.rs` now owns a small UI texture upload surface:
  - `UiTextureDescriptor`
  - `UiTextureFormat`
  - `UiTextureFilter`
  - `UiTextureWrap`
  - `Renderer::create_ui_texture`
  - `Renderer::resize_ui_texture`
  - `Renderer::update_ui_texture_from_pbo`
  - `Renderer::update_ui_texture_region`
  - `Renderer::create_ui_upload_pbo`
  - `Renderer::write_ui_upload_pbo`
- `src/ui/game_ui_manager.rs` still owns Slint overlay timing and cached draw
behavior, but texture resize/upload and PBO writes now route through `Renderer`.
- `src/ui/ability_bar_renderer.rs` still preserves its standalone Slint update
throttling and cached state behavior, but texture/PBO creation and upload now
route through `Renderer`.
- `src/ui/game_new/views/ability_bar_view.rs` keeps the existing path-based icon
cache and fallback-to-zero behavior, but icon texture creation now routes through
`Renderer`.
- `src/ui/game_new/render/renderer.rs` keeps batching, glyph rendering, scissor,
and shader behavior unchanged, but white texture and glyph atlas upload now use
renderer-owned texture helpers.

## Preserved Constraints

- Do not change UI layout, Slint overlay behavior, update/render throttling, or
cached overlay draw behavior without a separate UI task.
- Do not change ability icon cache keys, fallback texture ID behavior, or slot
data flow without a separate gameplay/UI task.
- Do not change ECS model storage. `Model` still carries the current CPU data and
GL handle fields.
- Do not change shader versions, framebuffer formats, or model rendering as part
of renderer centralization cleanup.
- Do not add dependencies for the WebGL prep phases unless explicitly requested.

## Remaining Raw GL Islands

- `src/renderer.rs`: main 3D frame, HDR/MSAA/FXAA/bloom, shadow map, skybox,
fullscreen quads, texture helpers, and model upload/draw helpers.
- `src/ui/game_ui_manager.rs`: overlay quad creation/draw and blend/depth state
for compositing still live in the UI integration layer.
- `src/ui/ability_bar_renderer.rs`: cleanup remains local because ownership of
the texture/PBO handle remains local.
- `src/ui/game_new/render/renderer.rs`: custom UI VBO/EBO uploads, draw calls,
texture binding, scissor, blend/depth/cull state, and cleanup remain a UI
renderer island.
- `src/ui/portrait_renderer.rs`: portrait framebuffer/renderbuffer setup and HUD
texture rendering are still separate feature-renderer work.
- `src/particles.rs`: particle texture upload, dynamic buffers, instancing, and
state setup are still separate feature-renderer work.
- `src/platform.rs` and `src/shaders.rs`: native GL loading and desktop GLSL
program compilation are still native-backend assumptions.

## WebGL Compatibility Distance

The codebase is closer to a web port because UI texture upload behavior is now
behind one renderer-owned API, but the renderer is not WebGL-ready yet.

Likely blockers for a WebGL2/itch build:

- PBOs and `MapBuffer` are still used inside the renderer helper. WebGL does not
support this upload path directly, so Phase 4 needs a non-PBO UI upload path
behind the same API.
- HDR and bloom use `RGBA16F`, float color targets, MRT draw buffers, and
framebuffer blits. These need a WebGL2 extension/format audit and fallback plan.
- MSAA resolve currently depends on framebuffer blits and renderbuffer behavior
that should be checked against WebGL2 constraints.
- Shadow/depth textures and portrait framebuffer attachments need WebGL2 format
checks.
- Shader source version/profile assumptions need a GLSL ES compatibility pass.
- `src/platform.rs` is native-window/native-GL oriented. A web build needs a
canvas/WASM platform path and web-safe asset loading.
- Filesystem asset paths need an itch/web packaging plan before full game startup
can work in the browser.

## Recommended Phase 4

Phase 4 should be a compatibility audit plus the smallest backend-friendly
fallbacks, not a broad renderer rewrite.

1. Create a WebGL2 compatibility checklist for every `gl::` call still used by
  the renderer and UI renderer islands.
2. Replace the UI PBO upload implementation behind `Renderer::write_ui_upload_pbo`
  and `Renderer::update_ui_texture_from_pbo` with an API shape that can support
   both native PBO uploads and direct pixel uploads.
3. Audit shaders for GLSL ES compatibility before changing shader behavior.
4. Audit framebuffer formats and attachments for HDR, bloom, FXAA, shadows, and
  portrait rendering.
5. Build a minimal web proof-of-life target with a simple clear/color quad or
  static scene before trying the full game UI and model stack.
6. Only after the minimal web target works, bring up asset loading, UI overlay,
  model rendering, particles, and post-processing one at a time.

## Suggested First PRs

- PR 1: Rename the PBO-specific UI helper API to an upload-buffer-neutral shape
while keeping the native implementation unchanged.
- PR 2: Add a direct CPU-slice texture update path for UI textures and use it in
one low-risk call site behind a feature/platform decision.
- PR 3: Add a shader compatibility inventory for every file in
`resources/shaders/`.
- PR 4: Add a framebuffer compatibility inventory for HDR, bloom, FXAA, shadow,
and portrait targets.
- PR 5: Add the first web platform skeleton only after the compatibility notes
identify the minimum supported render path.