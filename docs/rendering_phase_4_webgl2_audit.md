# Rendering Phase 4 WebGL2 Compatibility Audit

Phase 4 starts the renderer/WebGL2 compatibility work for an eventual
WASM/itch HTML5 build. This phase is intentionally conservative: preserve
current native behavior, keep ECS model storage unchanged, keep current shader
versions and framebuffer formats unchanged, avoid new dependencies, and keep
raw GL changes inside renderer/backend-owned helpers where possible.

## Prior Art

- [WebGL 2.0](https://registry.khronos.org/webgl/specs/latest/2.0/) is based
on OpenGL ES 3.0 but does not expose unsafe buffer mapping APIs such as
`MapBuffer`/`UnmapBuffer` to web code.
- [EXT_color_buffer_float](https://registry.khronos.org/webgl/extensions/EXT_color_buffer_float/)
is required before floating point color formats such as `RGBA16F` are
color-renderable in WebGL2.
- [WebGL texture parameter rules](https://developer.mozilla.org/en-US/docs/Web/API/WebGLRenderingContext/texParameter)
only include `REPEAT`, `CLAMP_TO_EDGE`, and `MIRRORED_REPEAT` as core wrap
modes, so `CLAMP_TO_BORDER` is not a portable shadow-map assumption.
- WebGL2 shaders use GLSL ES 3.00 (`#version 300 es`) with precision
qualifiers, while the current renderer shaders use desktop GLSL
`#version 460 core`.

## Current Blockers

### UI Uploads

- `src/renderer.rs` owns the UI upload helpers, but the native implementation
still uses `PIXEL_UNPACK_BUFFER`, `MapBuffer`, and `UnmapBuffer`.
- `src/ui/game_ui_manager.rs` and `src/ui/ability_bar_renderer.rs` keep upload
handles and render Slint software pixels through that mapped-buffer path.
- WebGL2 needs a direct CPU-slice upload route behind the same renderer-owned
abstraction so UI call sites do not grow web-specific raw GL branches.

### Framebuffers

- The main HDR, bright, ping-pong bloom, bloom mip, and MSAA targets in
`src/renderer.rs` use `RGBA16F`, MRT `DrawBuffers`, multisample renderbuffer
storage, and framebuffer blits.
- `RGBA16F` renderability is extension-dependent in WebGL2, and multisampled
float render targets are the riskiest part of the current graph.
- The HDR depth texture uses `DEPTH_COMPONENT24` with `DEPTH_COMPONENT` and
`UNSIGNED_INT`; the shadow map uses an unsized `DEPTH_COMPONENT` internal
format with `FLOAT`.
- `src/ui/portrait_renderer.rs` uses `RGBA8` plus a `DEPTH_COMPONENT24`
renderbuffer, which is closer to WebGL2 but should still remain a checked
capability with a fallback plan instead of a hard panic in a web build.

### Shadow Sampling

- The shadow map setup in `src/renderer.rs` uses `CLAMP_TO_BORDER` and
`TEXTURE_BORDER_COLOR`.
- WebGL2-core does not support that wrap mode, so the future web path needs
either shader-side border behavior, a shadow atlas/border texel strategy, or
a targeted extension gate.

### Shader Sources

- All active shader files under `resources/shaders/` use `#version 460 core`.
- Model and particle shaders use multiple render targets for `FragColor` and
`BrightColor`.
- `src/shaders.rs` loads shader text from the filesystem, splits combined
shader files by marker comments, and supports optional geometry shader
sections. Geometry shaders are not part of WebGL2.
- Phase 4 only inventories the GLSL ES work; it does not change shader versions
or introduce shader generation.

### Platform And Canvas

- `src/platform.rs` creates a native glutin/winit window, requests OpenGL 4.6
core, and has a native GLES 3.2 fallback. It also loads function pointers from
the native display.
- An itch HTML5 build needs a canvas-backed WebGL2 context, canvas backing-store
resize handling, web-safe pointer/keyboard behavior, and a present/swap path
that is not a glutin `WindowSurface`.
- An itch native zip is a separate target and can continue using the desktop
path if assets and working directory are packaged predictably.

### Assets

- Shaders, models, textures, RON UI files, fonts, particles, skybox faces, and
sound banks are loaded through direct `resources/...` filesystem paths.
- Representative paths include `src/shaders.rs`, `src/renderer.rs`,
`src/particles.rs`, `src/ui/game_new/font_system.rs`,
`src/ui/game_new/parser/ron_loader.rs`, and
`src/ui/game_new/views/ability_bar_view.rs`.
- A future web build needs an asset-source boundary for HTTP, embedded, or
preloaded resources. Phase 4 should not thread a new asset system through the
whole game yet.

### Feature Renderer Islands

- `src/ui/game_new/render/renderer.rs` owns custom UI VBO/EBO uploads, scissor
state, alpha-mask glyph atlas updates, and draw calls.
- `src/ui/portrait_renderer.rs` owns portrait framebuffer setup and draw state.
- `src/particles.rs` owns particle texture upload, instanced buffer uploads,
blend/depth state, and `DrawArraysInstanced`.
- These islands are acceptable for now, but new compatibility paths should be
introduced through renderer/backend helpers when the code has to change.

## Shader Inventory


| Shader                                                    | Current profile            | WebGL2 notes                                                                                                         |
| --------------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `resources/shaders/custom_ui.glsl`                        | `#version 460 core`        | Needed early for proof-of-life UI. Add `#version 300 es`, precision qualifiers, and verify `R8` alpha-mask sampling. |
| `resources/shaders/ui_overlay.glsl`                       | `#version 460 core`        | Needed for Slint overlay composition. Should be straightforward GLSL ES once version/precision are handled.          |
| `resources/shaders/hdr.glsl`                              | `#version 460 core`        | Depends on HDR/bloom target availability and depth sampling from the HDR depth texture.                              |
| `resources/shaders/fxaa.glsl`                             | `#version 460 core`        | Likely portable after GLSL ES conversion; depends on the resolved color texture path.                                |
| `resources/shaders/bloom/*.glsl`                          | `#version 460 core`        | Depends on float color-buffer capability or an `RGBA8`/disabled-bloom fallback.                                      |
| `resources/shaders/model/static_model.glsl`               | `#version 460 core`        | Uses MRT outputs and shadow sampling. Needs GLSL ES conversion plus shadow-border strategy.                          |
| `resources/shaders/model/animated_model.glsl`             | `#version 460 core`        | Same as static model, plus bone matrix uniform limits need checking on web/mobile.                                   |
| `resources/shaders/particles.glsl`                        | `#version 460 core`        | Uses instancing and MRT outputs. WebGL2 supports instancing, but bloom output depends on MRT/float target path.      |
| `resources/shaders/depth_shader.glsl`                     | `#version 460 core`        | Needs GLSL ES conversion and explicit depth attachment format decisions.                                             |
| `resources/shaders/skybox.glsl`                           | `#version 460 core`        | Needs GLSL ES conversion and web-safe cubemap asset loading.                                                         |
| `resources/shaders/debug_depth_quad.glsl`                 | `#version 460 core`        | Debug-only path; lower priority for minimal web proof-of-life.                                                       |
| `resources/shaders/gizmo.glsl`                            | `#version 460 core`        | Current gizmo path also uses `PolygonMode`, which is not WebGL2-compatible.                                          |
| `resources/shaders/text.glsl`                             | `#version 460 core`        | Legacy text path. Confirm whether it is still needed before porting.                                                 |
| `resources/shaders/game_ui.glsl`                          | `#version 460 core`        | Legacy game UI path. Confirm active usage before porting.                                                            |
| `resources/shaders/point_light.glsl`                      | `#version 460 core`        | Debug light path; lower priority.                                                                                    |
| `resources/shaders/color_for_texture.glsl`                | `#version 460 core`        | Not currently loaded by `Renderer::new`; keep out of the first web proof-of-life.                                    |
| `resources/shaders/frostbite_volumetric_fog_compute.glsl` | compute-style desktop GLSL | Not currently loaded. WebGL2 has no compute shaders, so this is out of scope for the web renderer path.              |


## Framebuffer Inventory


| Target                       | Current format/path                                                | WebGL2 risk                                                                        |
| ---------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| HDR color                    | `RGBA16F`, `RGBA`, `FLOAT`, `COLOR_ATTACHMENT0`                    | Requires `EXT_color_buffer_float`; fallback should disable HDR or use `RGBA8`.     |
| HDR bright                   | `RGBA16F`, `RGBA`, `FLOAT`, `COLOR_ATTACHMENT1`                    | Same as HDR color; also depends on MRT availability with the chosen format.        |
| HDR depth texture            | `DEPTH_COMPONENT24`, `DEPTH_COMPONENT`, `UNSIGNED_INT`             | Needs explicit WebGL2 validation and fallback format choice.                       |
| HDR MSAA color renderbuffers | `RenderbufferStorageMultisample`, `RGBA16F`, two color attachments | High risk; multisampled float targets may need to be disabled on web.              |
| HDR MSAA depth renderbuffer  | `RenderbufferStorageMultisample`, `DEPTH_COMPONENT24`              | Validate sample counts and depth blit constraints.                                 |
| MSAA resolve                 | `BlitFramebuffer` for two color attachments and depth              | WebGL2 has narrower blit constraints; plan for no-MSAA or shader resolve fallback. |
| Bloom ping-pong              | `RGBA16F` textures                                                 | Same extension risk as HDR color.                                                  |
| Bloom mip chain              | `RGBA16F` textures and single draw buffer                          | Same extension risk; should be optional on web.                                    |
| FXAA target                  | `RGBA8`, `UNSIGNED_BYTE`                                           | Generally WebGL2-friendly.                                                         |
| Shadow map                   | `DEPTH_COMPONENT`, `FLOAT`, `CLAMP_TO_BORDER`                      | Needs sized format and border behavior changes for web.                            |
| Portrait FBO                 | `RGBA8` color texture, `DEPTH_COMPONENT24` renderbuffer            | Likely portable, but should not hard panic in a web build.                         |


## Acceptance Criteria For Phase 4

- Native rendering behavior remains unchanged by default.
- UI upload API names no longer expose PBO as the renderer-facing concept.
- A direct CPU-slice UI texture upload helper exists for future WebGL use.
- The known WebGL2 blockers are documented with file paths and subsystem
ownership.
- Shader and framebuffer inventories are recorded without changing shader
versions or framebuffer formats.
- Platform/canvas and asset-loading blockers are documented without adding
dependencies or starting a web platform implementation.

