# Rendering Phase 6 PR 2 Handoff

PR 2 should make framebuffer creation explicit, checked, and ready for the
WebGL-compatible render path. This follows the Phase 6 render-target policy
work and should keep the desktop renderer behavior unchanged by default.

Read first:

- `docs/rendering_phase_6_handoff.md`
- `docs/rendering_phase_4_webgl2_audit.md`
- `src/renderer.rs`
- `src/platform.rs`
- `src/config/game_config.rs`

## Current State

Phase 6 PR 1 added a private `RenderTargetPolicy` in `src/renderer.rs`.

The policy is selected at the start of `Renderer::new` from
`GlCapabilities` plus `GameConfig::webgl_compatibility_mode` and logs:

- compatibility mode
- color format intent
- depth format intent
- HDR enabled/disabled
- bloom enabled/disabled
- MSAA enabled/disabled
- FXAA enabled/disabled
- MRT enabled/disabled

Compatibility mode still returns through `Renderer::new_webgl_compatibility`.
That constructor only creates default textures and keeps shaders, VAOs, FBOs,
HDR, bloom, MSAA, FXAA, and depth resources empty or disabled.

The native constructor still creates the existing desktop render graph inline:

- HDR FBO with two `RGBA16F` color textures and a `DEPTH_COMPONENT24` depth
texture.
- Bloom ping-pong FBOs with `RGBA16F` textures.
- MSAA HDR FBO with multisampled `RGBA16F` color renderbuffers.
- Shadow depth map FBO.
- FXAA FBO with `RGBA8` color texture.
- Bloom mip chain through `Renderer::create_bloom_chain`.

## PR 2 Goal

Extract framebuffer setup into small renderer-owned helpers so future
compatibility paths can create WebGL-safe resources without copying or mutating
the desktop path.

The target outcome is:

- Framebuffer completeness checks are centralized enough to produce useful
diagnostics.
- Desktop startup still creates the same resources and uses the same formats.
- Compatibility mode has a clear helper path for either a simple scene target
or intentional default-framebuffer rendering.
- The new helpers are driven by `RenderTargetPolicy`, not platform-name checks.

## Non-Goals

- Do not re-enable world rendering in compatibility mode yet.
- Do not port world shaders in this PR.
- Do not change shadow sampling or `CLAMP_TO_BORDER` yet.
- Do not add HDR, bloom, MSAA, or FXAA to compatibility mode.
- Do not add dependencies.
- Do not change ECS, model loading, UI layout, input, assets, or packaging.
- Do not rewrite the full render graph.

## Prior Art

- WebGL2 is based on OpenGL ES 3.0, so framebuffer resources should be created
from ES/WebGL-safe formats when compatibility mode is selected.
- WebGL `EXT_color_buffer_float` is required before `RGBA16F` is
color-renderable, and multisampled float renderbuffers are still optional.
- MDN WebGL best practices recommend setting up hot framebuffers ahead of time
because attachment changes can invalidate completeness and trigger browser
validation overhead.
- WebGL render-to-float support cannot be assumed just because float textures
are sampleable, so completeness checks and fallback policy should be explicit.

## Suggested Implementation

### 1. Add framebuffer diagnostics helpers

Add small private helpers near the existing renderer setup code:

- `fn framebuffer_status_label(status: u32) -> &'static str`
- `fn check_framebuffer_complete(label: &str, details: &str)`

The helper should call `gl::CheckFramebufferStatus(gl::FRAMEBUFFER)` and panic
or log with enough information to identify:

- target name
- selected color/depth formats
- width and height
- GL framebuffer status hex value and readable label

Keep the current fail-fast behavior for native startup. The important change is
better diagnostics and one place to add compatibility behavior later.

### 2. Extract native HDR target creation

Move the existing HDR FBO creation block into a helper that preserves current
behavior:

- two `RGBA16F` color textures
- `COLOR_ATTACHMENT0` and `COLOR_ATTACHMENT1`
- `DrawBuffers` for both attachments
- `DEPTH_COMPONENT24` depth texture
- `fbos.insert(FboType::HDR, hdr_fbo)`
- returned `hdr_color`, `hdr_bright`, and `hdr_depth`

Suggested shape:

- `fn create_hdr_framebuffer(width: u32, height: u32, policy: RenderTargetPolicy) -> HdrFramebuffer`

For this PR, the helper may assert or debug-assert that it is using the native
policy. Do not make the compatibility path create `RGBA16F`.

### 3. Extract native bloom ping-pong target creation

Move the ping-pong FBO setup into a helper that returns:

- `pingpong_fbos: [u32; 2]`
- `pingpong_tex: [u32; 2]`

Preserve the existing `RGBA16F` texture format and completeness check.

Do not change bloom execution yet. `Renderer::create_bloom_chain` can remain as
it is unless extracting a shared completeness check is low risk.

### 4. Extract native MSAA target creation

Move the HDR MSAA FBO setup into a helper that:

- preserves current multisampled `RGBA16F` color renderbuffers
- preserves current `DEPTH_COMPONENT24` depth renderbuffer
- inserts `FboType::HdrMsaa`
- only runs when `policy.msaa_enabled` is true

If `policy.msaa_enabled` is false, keep the current behavior of disabling
`gl::MULTISAMPLE` and using no MSAA target.

### 5. Add a compatibility simple-scene target helper, but do not use it yet

Add a helper that can create a WebGL-safe simple scene target from the
compatibility policy:

- color texture: `RGBA8`, `RGBA`, `UNSIGNED_BYTE`
- depth attachment: `DEPTH_COMPONENT24` with `DEPTH_COMPONENT`, `UNSIGNED_INT`,
or a depth renderbuffer if that fits the local code better
- one color attachment only
- no MRT
- no MSAA
- no HDR/bloom attachments

It is acceptable for this helper to be unused in PR 2 if that keeps behavior
unchanged. If unused warnings are a concern, keep it private but call it only
from a narrowly named compatibility constructor path in a later PR.

## Acceptance Criteria

- Native mode still follows the existing render path.
- Compatibility mode still starts through `Renderer::new_webgl_compatibility`
and does not construct HDR, bloom, MSAA, shadow, or FXAA resources.
- All newly extracted framebuffer creation paths call the shared completeness
check.
- Failure messages include framebuffer label, formats, dimensions, status hex,
and readable status label.
- No desktop resource formats change in this PR.
- No new dependencies are added.

## Validation

Run:

- `cargo fmt`
- `cargo check`
- Cursor lints on edited Rust files

If possible, also smoke-test native startup with `webgl_compatibility_mode: false` and compatibility startup with `webgl_compatibility_mode: true`, but do
not commit local config churn.

## Risks And Notes

- `Renderer::new` is large and currently builds many resources inline. Keep the
extraction mechanical and avoid changing render behavior while moving code.
- Avoid a broad abstraction. The first helpers can be renderer-private structs
and functions.
- Do not make compatibility mode depend on `RGBA16F`, MRT, or MSAA even if the
local desktop driver supports them.
- PR 3 should handle shadow map `CLAMP_TO_BORDER` fallback separately.
- PR 4 should port the first world shaders to GLSL ES 3.00 separately.