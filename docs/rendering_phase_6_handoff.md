# Rendering Phase 6 Handoff

Phase 6 should harden the real renderer for WebGL2/OpenGL ES 3.0 constraints
after the Phase 5 minimal compatibility proof. This is still not the full
browser/itch.io build. The goal is to make the native render graph able to
initialize and degrade predictably in a WebGL-compatible profile while keeping
the desktop OpenGL path unchanged.

Read first:

- `docs/rendering_phase_5_handoff.md`
- `docs/rendering_phase_4_webgl2_audit.md`
- `src/platform.rs`
- `src/shaders.rs`
- `src/renderer.rs`
- `src/game.rs`
- `src/config/game_config.rs`
- `resources/shaders/custom_ui.glsl`
- `resources/shaders/custom_ui_es300.glsl`

## Current State

Phase 5 groundwork is now present in the codebase:

- `GlCapabilities` exists in `src/platform.rs` and records GLES/WebGL-relevant
  support flags.
- `ShaderProfile` exists in `src/shaders.rs` with `DesktopCore` and
  `GlslEs300`.
- `Shader::new_with_profile` can select profile-specific shader sources.
- `resources/shaders/custom_ui_es300.glsl` provides a GLSL ES 3.00 version of
  the custom RON UI shader.
- `UiRenderer::new_with_profile` can choose the UI shader profile.
- `GameConfig` has `webgl_compatibility_mode` with `#[serde(default)]`.
- Compatibility mode currently clears the default framebuffer and renders
  `resources/ui/gallery_view.ron`, while skipping the main world render graph.
- `Renderer::new` has a minimal compatibility constructor that avoids
  constructing desktop-only HDR/bloom/MSAA/world resources.
- `gl::PolygonMode` is already gated by `supports_gizmo_wireframe`, which
  returns false for GLES-like contexts.

Validation from the Phase 5 change:

- `cargo fmt`
- `cargo check`
- Cursor lints on edited Rust files

## Phase 6 Goal

Bring the real renderer back into compatibility mode one safe slice at a time.

The target outcome is:

- Compatibility mode can create WebGL-safe renderer resources without panics.
- Framebuffer setup uses explicit fallbacks instead of desktop GL assumptions.
- Shadow texture behavior no longer depends on `CLAMP_TO_BORDER`.
- A small world smoke path can render with GLSL ES 3.00 shaders, then overlay
  the existing RON UI.

## Non-Goals

- Do not add dependencies.
- Do not start an Emscripten, WASM, or itch.io packaging PR yet.
- Do not port the whole shader tree in one pass.
- Do not rework ECS, model loading, animation, gameplay, input, or asset
  formats.
- Do not add HDR or bloom compatibility until the basic no-post-processing path
  is stable.
- Do not make native OpenGL use the compatibility fallbacks unless explicitly
  requested by config or required by an actual GLES-like context.

## Prior Art

- WebGL2 is based on OpenGL ES 3.0, so the compatibility target should be
  GLSL ES 3.00 and ES-safe GL state rather than desktop `#version 460 core`.
- Emscripten's OpenGL support is strongest when the app stays within the
  WebGL-friendly ES subset instead of depending on desktop GL emulation.
- MDN WebGL best practices call out framebuffer completeness, stable
  framebuffer attachments, and avoiding WebGL errors as important because
  browser validation overhead is real.

## Recommended PR Sequence

### PR 1: Render Target Policy

Add a small policy object or helper near `Renderer::new` that chooses render
target formats and feature toggles from `GlCapabilities` plus
`GameConfig::webgl_compatibility_mode`.

Suggested shape:

- Keep desktop defaults unchanged.
- In compatibility mode, choose:
  - color: `RGBA8`
  - depth: `DEPTH_COMPONENT24` or an existing supported depth fallback
  - HDR: disabled
  - bloom: disabled
  - MSAA: disabled
  - FXAA: disabled unless proven safe later
  - MRT: disabled at first, even if available
- Log the selected policy once at renderer startup.

Acceptance:

- Native startup still takes the existing path.
- Compatibility startup does not create `RGBA16F` HDR/bloom targets.
- Compatibility startup does not create float MSAA renderbuffers.
- The selected policy is visible in logs.

### PR 2: WebGL-Safe Framebuffer Helpers

Extract framebuffer creation into helpers that can create either native or
compatibility resources.

Suggested shape:

- Add helpers for:
  - default/simple scene framebuffer
  - depth texture attachment
  - framebuffer completeness check with useful diagnostics
- Do not mutate hot framebuffer attachments every frame.
- Prefer creating complete framebuffers once during startup or resize.
- Keep the current desktop HDR/bloom path available behind the native policy.

Acceptance:

- Compatibility mode can create a simple scene target or intentionally render
  to the default framebuffer without touching HDR/bloom setup.
- Every new framebuffer path checks `gl::CheckFramebufferStatus`.
- Failure logs include the selected internal formats and dimensions.

### PR 3: Shadow Clamp Fallback

Remove the compatibility dependency on `CLAMP_TO_BORDER`.

Current risk:

- WebGL2/GLES does not reliably support desktop-style shadow border color
  behavior.
- `GlCapabilities::supports_clamp_to_border` already exists, but the renderer
  still needs a fallback behavior where shadow setup and shader sampling do not
  assume border color.

Suggested shape:

- Use `CLAMP_TO_EDGE` for shadow maps in compatibility mode.
- Add shader-side edge handling for shadow lookup coordinates outside the
  shadow map range.
- Keep desktop `CLAMP_TO_BORDER` if available and used by the current native
  path.

Acceptance:

- Compatibility mode never calls unsupported border clamp state.
- Native shadow behavior remains unchanged.
- Shadow shader changes are profile-gated or split into desktop and ES shader
  variants.

### PR 4: First World Shader ES Ports

Port only the minimum shaders needed for a simple world smoke test.

Preferred first targets:

- `resources/shaders/depth_shader.glsl`
- `resources/shaders/skybox.glsl`
- `resources/shaders/model/static_model.glsl`

Use the same pattern as `custom_ui.glsl`:

- Keep the desktop shader source unchanged.
- Add parallel `*_es300.glsl` files or a narrow profile mapping in
  `src/shaders.rs`.
- Reject geometry shaders under `ShaderProfile::GlslEs300`.
- Add precision qualifiers and explicit fragment outputs.
- Use `texture(...)` and GLSL ES 3.00-compatible syntax.

Acceptance:

- Desktop shader loading remains unchanged.
- Compatibility mode can compile the selected ES shader variants.
- Animated model, particles, HDR, bloom, and post-processing shaders remain out
  of scope.

### PR 5: Compatibility World Smoke Path

Replace the current compatibility render path's UI-only proof with a tiny world
path.

Suggested shape:

- Keep compatibility mode simple:
  - clear framebuffer
  - optional depth pass if needed by the selected scene path
  - skybox or one static model bucket
  - RON UI overlay
- Skip:
  - HDR
  - bloom
  - MSAA
  - particles
  - animated models
  - ImGui
  - portrait UI
- Keep `Game::render` as the high-level switch until the compatibility path is
  large enough to deserve its own renderer entry point.

Acceptance:

- `webgl_compatibility_mode: true` renders more than just UI without entering
  the full native render graph.
- Native rendering is unchanged when the flag is false.
- The path avoids known WebGL-hostile state and formats.

## Suggested Phase 6 Definition Of Done

Phase 6 is done when compatibility mode can:

- Start without constructing HDR, bloom, float MSAA, or unsupported shadow state.
- Compile the first ES world shader set.
- Render a simple static scene or skybox-style smoke path.
- Overlay the existing RON UI.
- Run `cargo check` cleanly.

It does not need to look like the final game in a browser yet.

## Leave For Phase 7

- Emscripten target setup.
- Browser canvas/window integration.
- Asset packaging for itch.io.
- Animated model ES shaders.
- Particle ES shaders.
- HDR/bloom fallback design.
- Browser input/audio polish.
- Save/config persistence strategy for web.

## Notes For The Next Agent

- The repo has a dirty working tree with broad Slint-removal changes. Do not
  revert unrelated deletions or modifications.
- Keep PRs small and compatibility-flagged.
- Prefer capability-driven branches over platform-name checks.
- Avoid adding dependencies unless explicitly requested.
- If you need to test the Phase 5 compatibility path, set
  `webgl_compatibility_mode` to `true` in the local game config, but do not
  commit generated local config churn unless requested.
