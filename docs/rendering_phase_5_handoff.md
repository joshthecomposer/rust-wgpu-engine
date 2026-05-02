# Rendering Phase 5 Handoff

Phase 5 should turn the Phase 4 WebGL2 audit into the first narrow
GLES/WebGL-compatible renderer path. This is not a full WASM/itch HTML5 port
yet. The goal is to make compatibility decisions explicit in code and prove one
small render path can obey WebGL2-style constraints while preserving current
native behavior.

Read first:

- `docs/rendering_phase_1.md`
- `docs/rendering_phase_3_handoff.md`
- `docs/rendering_phase_4_webgl2_audit.md`
- `src/renderer.rs`
- `src/platform.rs`
- `src/shaders.rs`
- `resources/shaders/custom_ui.glsl`
- `resources/shaders/ui_overlay.glsl`

## Scope

Build the smallest compatibility slice that edges the renderer toward OpenGL ES
3.0/WebGL2:

- Add renderer/platform capability reporting.
- Add shader-profile plumbing for desktop GLSL vs GLSL ES.
- Port one low-risk UI shader path to a GLES/WebGL-compatible profile.
- Gate or isolate desktop-only GL calls needed by that slice.
- Use the existing direct CPU-slice UI texture upload path for the compatibility
  profile.
- Keep the current native renderer behavior unchanged by default.

## Non-Goals

- Do not add dependencies.
- Do not start a wgpu migration.
- Do not add a full `wasm32` target or browser canvas platform yet.
- Do not change ECS model storage.
- Do not change model loading, animation, gameplay, UI layout, or input logic.
- Do not migrate every shader in one PR.
- Do not change HDR, bloom, MSAA, shadow, or portrait framebuffer formats yet.
- Do not remove the native PBO upload path.

## Prior Art

- WebGL2 follows OpenGL ES 3.0 closely, so a native GLES-compatible slice is a
  lower-risk stepping stone before browser startup.
- WebGL2 shaders use GLSL ES 3.00 with precision qualifiers, not desktop
  `#version 460 core`.
- WebGL2 cannot expose `MapBuffer`/`UnmapBuffer`, so texture upload code needs a
  CPU-slice route behind renderer-owned helpers.
- Float render targets such as `RGBA16F` require explicit extension support in
  WebGL2, so full HDR/bloom hardening should come after capability reporting.

## Recommended PR Sequence

### PR 1: Capability Reporting

Add a small renderer/platform capability surface without changing behavior.

Suggested shape:

- Add a `RendererCapabilities` or `GlCapabilities` struct near the renderer or
  platform boundary.
- Query/log:
  - `GL_VERSION`
  - `GL_SHADING_LANGUAGE_VERSION`
  - `GL_VENDOR`
  - `GL_RENDERER`
  - supported extension string or extension count/list, depending on API
    availability.
- Record booleans for future decisions:
  - `is_gles_like`
  - `supports_float_color_buffer`
  - `supports_msaa_float_renderbuffer`
  - `supports_clamp_to_border`
  - `supports_buffer_mapping`
  - `supports_instancing`
  - `supports_mrt`
- Keep all flags informational at first unless the current context cannot
  report them.

Acceptance:

- Native startup still renders exactly as before.
- Startup logs enough GL/driver information to explain which compatibility path
  should be selected later.
- No framebuffer or shader behavior changes.

### PR 2: Shader Profile Plumbing

Introduce shader-profile awareness without porting the full shader tree.

Suggested shape:

- Add a `ShaderProfile` enum, for example:
  - `DesktopCore`
  - `GlslEs300`
- Let `Shader::new` keep its existing native behavior.
- Add a profile-aware constructor or internal path that can choose a shader
  source variant.
- Keep combined shader files and `// VERTEX_SHADER` / `// FRAGMENT_SHADER`
  splitting intact.
- Explicitly reject or skip `// GEOMETRY_SHADER` for `GlslEs300`.

Acceptance:

- Existing callers still compile without needing to pass a profile.
- A test or debug path can request `GlslEs300` for a single shader.
- No global shader version rewrite.

### PR 3: Port One UI Shader

Prove the shader-profile path with one low-risk shader.

Preferred first target:

- `resources/shaders/custom_ui.glsl`

Alternative:

- `resources/shaders/ui_overlay.glsl`

Keep the desktop shader source unchanged. Add either:

- a parallel ES shader file, or
- a small profile-specific prelude/source transform.

For GLSL ES 3.00, account for:

- `#version 300 es`
- fragment precision qualifiers
- explicit fragment outputs
- sampler precision if needed
- `R8`/`RED` alpha-mask texture sampling in the custom UI path

Acceptance:

- Desktop native path still uses the current shader by default.
- The selected UI shader can be compiled through the ES-profile path.
- No model, particle, HDR, bloom, or shadow shaders are changed.

### PR 4: Gate Desktop-Only GL Calls For The Compatibility Profile

Start with calls known to be illegal or risky on GLES/WebGL2.

First target:

- `gl::PolygonMode` in `Renderer::gizmo_pass`.

Preferred behavior:

- Keep native desktop gizmo wireframe behavior unchanged.
- On a GLES/WebGL-like profile, disable that wireframe path or route it through
  a clearly named placeholder until a real line-list or shader-wireframe gizmo
  renderer exists.

Acceptance:

- Desktop gizmos still render as before.
- Compatibility profile never calls `gl::PolygonMode`.
- No gameplay or physics debug data structures are refactored.

### PR 5: Compatibility Upload Path Selection

Use the Phase 4 upload abstraction to select a WebGL-safe texture upload route.

Existing helpers:

- `Renderer::create_ui_upload_buffer`
- `Renderer::write_ui_upload_buffer`
- `Renderer::update_ui_texture_from_upload_buffer`
- `Renderer::update_ui_texture_from_pixels`

Suggested shape:

- Keep the native upload-buffer path as the default.
- Add a compatibility path that renders Slint/UI pixels into a CPU vector and
  uploads through `update_ui_texture_from_pixels`.
- Keep this decision inside renderer/backend-owned helpers or a small UI upload
  adapter. Do not put raw web-specific GL branches into gameplay or UI layout
  code.

Acceptance:

- Native PBO-backed upload behavior remains the default.
- A compatibility mode can avoid `MapBuffer` entirely.
- UI call sites stay thin and renderer-owned.

### PR 6: Minimal Compatibility Mode

Add a small startup/render mode for validating the compatibility slice.

Suggested shape:

- A config/debug flag or temporary development path that renders:
  - clear color
  - one custom UI quad or overlay quad
- Exclude for now:
  - HDR
  - bloom
  - MSAA
  - shadows
  - models
  - particles
  - full Slint/game UI

Acceptance:

- The compatibility path can run without touching high-risk framebuffers.
- The path proves shader-profile selection and direct texture upload can work
  together.
- The normal game path remains unchanged unless the compatibility mode is
  explicitly selected.

## Risks To Leave For Later Phases

- HDR/bloom fallback from `RGBA16F` to `RGBA8` or disabled post-processing.
- Shadow format and `CLAMP_TO_BORDER` replacement.
- Full model and animated shader GLSL ES conversion.
- Asset source abstraction for web fetch/embed/preload.
- Actual `wasm32` build target and canvas setup.
- Context loss and GPU resource lifetime centralization.

## Suggested Phase 6

After Phase 5 proves the compatibility slice, Phase 6 should harden the real
render graph behind capability decisions:

- Disable or downgrade HDR/bloom when float color buffers are unavailable.
- Disable MSAA or use a WebGL-safe resolve path when multisampled float targets
  are unavailable.
- Replace shadow `CLAMP_TO_BORDER` assumptions.
- Port static model, animated model, depth, skybox, and particle shaders in
  small groups.
- Start an asset-source boundary with shaders first, then UI RON/fonts, then
  textures/models.

