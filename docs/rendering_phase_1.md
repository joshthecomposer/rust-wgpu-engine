# Rendering Centralization Phase 1

This document tracks the raw OpenGL call sites left after Phase 1. The first
boundary is now at `Renderer`: game code should call renderer-owned entry points
instead of selecting shaders, default textures, framebuffer handles, or texture
ids directly.

## Current Boundary

- `src/game.rs` orchestrates high-level render order only: world, game portrait,
game overlay, custom UI, debug UI, then swap buffers.
- `src/renderer.rs` owns shader selection for the main world, portrait, and game
overlay paths.
- Feature renderers still contain raw `gl::` calls internally. These are known
renderer/backend islands for later phases, not app-level call sites.

## Phase 2 Update

- Model mesh upload, model texture upload, material texture binding, and indexed
model draws now route through `Renderer` resource helpers.
- ECS storage is unchanged: `Model` still carries the current CPU data and GL
handle fields so entity/model cloning behavior stays stable for this phase.
- `src/animation/model.rs` is now data-only for renderer centralization purposes;
remaining raw GL for model resources is owned by `src/renderer.rs`.
- Model producers now call `Renderer::upload_model_mesh` and
`Renderer::upload_model_texture`; model draw paths use `Renderer::draw_model`
or `Renderer::draw_model_geometry`.

## Phase 3 Update

Phase 3 centralized UI texture upload ownership behind renderer-owned helper
APIs while preserving current UI behavior. See
`docs/rendering_phase_3_handoff.md` for the detailed handoff and next-phase
WebGL compatibility guidance.

Completed targets:

- `src/ui/game_ui_manager.rs`: Slint overlay texture resize/upload and PBO writes
now route through `Renderer`.
- `src/ui/ability_bar_renderer.rs`: standalone ability bar texture/PBO creation
and upload now route through `Renderer`.
- `src/ui/game_new/views/ability_bar_view.rs`: ability icon texture creation now
routes through `Renderer` while keeping the existing path cache behavior.
- `src/ui/game_new/render/renderer.rs`: custom UI white texture and glyph atlas
texture updates now use renderer-owned texture helpers.

Kept out of scope for Phase 3:

- ECS storage changes.
- Shader language/version migration.
- Framebuffer format compatibility work.
- Replacing OpenGL with WebGL, OpenGL ES, or wgpu.

## Remaining Raw GL Inventory


| File                                        | Responsibility                                                                                     | Phase 1 status                                | Later WebGL risk                                                                                                                         |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `src/renderer.rs`                           | Main 3D frame, HDR/MSAA/FXAA/bloom, shadow map, skybox, fullscreen quads, shared fallback textures | Central renderer owner                        | High: `RGBA16F`, float color targets, MRT draw buffers, framebuffer blits, depth texture formats, cubemap setup, polygon mode for gizmos |
| `src/shaders.rs`                            | Shader program compile/link, uniform lookup, uniform uploads                                       | Backend utility                               | Medium: GLSL version/profile and shader source format will matter for WebGL/OpenGL ES                                                    |
| `src/macros.rs`                             | Debug GL error wrapper                                                                             | Backend utility                               | Low: error constants exist, but behavior may differ by backend                                                                           |
| `src/platform.rs`                           | GL function loading and minimal startup state                                                      | Platform/backend boundary                     | Medium: native GL context setup will need a web-specific path                                                                            |
| `src/animation/model.rs`                    | Model CPU data and current GL handle fields for existing ECS storage                               | Data-only model container after Phase 2       | Medium: handle fields still exist until a later resource-cache phase                                                                     |
| `src/animation/data_loader.rs`              | Parses model and animation data, then asks `Renderer` to upload model resources                    | Uses renderer-owned upload APIs after Phase 2 | Medium: source pixel formats, mip generation, wrapping, and compressed/unsupported image formats still need backend review               |
| `src/particles.rs`                          | Particle buffer uploads, instancing, blending/depth state, particle texture upload                 | Feature renderer                              | Medium: instancing requires WebGL2; streaming buffer patterns may need tuning                                                            |
| `src/ui/game_new/render/renderer.rs`        | Batched GPU UI, dynamic buffers, glyph atlas upload, scissor, blend/depth/cull state               | UI renderer island; texture upload helpers now routed through `Renderer` | Medium: scissor/state use is fine, but dynamic buffer upload and atlas formats need verification                                         |
| `src/ui/game_ui_manager.rs`                 | Slint software buffer upload through PBO and overlay composite                                     | UI texture resize/upload now routed through `Renderer`; overlay draw remains local | High: `PIXEL_UNPACK_BUFFER`, `MapBuffer`, and PBO upload flow are not a good WebGL target                                                |
| `src/ui/ability_bar_renderer.rs`            | Ability bar texture streaming and cleanup                                                          | Texture/PBO creation and upload now routed through `Renderer`; cleanup remains local | High: PBO-style streaming still needs a WebGL-compatible implementation behind the renderer API                                          |
| `src/ui/game_new/views/ability_bar_view.rs` | Ability icon texture loading and cache for custom UI widgets                                       | Icon texture creation now routed through `Renderer`; cache behavior unchanged | Medium: texture format and cache ownership need backend review for web packaging                                                         |
| `src/ui/portrait_renderer.rs`               | Offscreen portrait framebuffer, renderbuffer/depth setup, model draw into HUD texture              | Feature renderer                              | Medium: framebuffer formats and renderbuffer attachments need WebGL-compatible choices                                                   |


## Non-Render False Positives

- `src/config/emitter_data.rs` contains commented example GL snippets only.
- `src/sound/sound_manager.rs` currently has a GL import but no meaningful render
path ownership.
- `src/movement_system.rs` currently has a GL import but no meaningful render path
ownership.

## Suggested Next Phases

- Phase 2: completed. Model GPU upload/draw helpers now route through
renderer-owned resource APIs without changing ECS storage.
- Phase 3: completed. UI texture upload paths now route through renderer-owned
texture helpers without changing UI behavior.
- Phase 4: audit shader versions, framebuffer formats, upload paths, and state
calls against a WebGL2/OpenGL ES compatibility checklist.