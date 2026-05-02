# WASM / WebGL2: HDR, Shadows, Bloom, MSAA handoff

## Context

- The desktop renderer uses HDR framebuffers (`RGBA16F`), MRT (`COLOR_ATTACHMENT0/1`), bloom chain, FXAA/MSAA where supported—see `Renderer::create_hdr_framebuffer` and `RenderTargetPolicy` in `src/renderer.rs`.
- For WebGL compatibility, `RenderTargetPolicy::for_capabilities` currently **hard-disables** HDR, bloom, MSAA, FXAA, and MRT whenever `config.webgl_compatibility_mode || capabilities.is_gles_like` (approx. lines 111–125).
- Shadows **already run** on the wasm compatibility path: `new_webgl_compatibility` → `render_world_webgl_compatibility` → `shadow_begin` / `shadow_end` (`src/renderer.rs` ~1382–1468).

The gap is restoring **HDR (and dependents: bloom → tonemap/exposure)** on wasm behind **correct capability probing**, not a permanent “compat = LDR” policy.

Recent fix (do not regress): **`WebGameRuntime` timing** uses the **first `requestAnimationFrame` timestamp as `start_ms`** so `Game::tick` receives sane `dt` (`src/web_game.rs`). Epoch `Date.now()` mixed with RAF time zeroed deltas and broke movement.

---

## Prior art (brief)

| Source | Takeaway |
|--------|----------|
| [Khronos `EXT_color_buffer_half_float`](https://registry.khronos.org/webgl/extensions/EXT_color_buffer_half_float/) | Defines when half-float attachments are color-renderable; WebGL2 still needs explicit probing on some GPUs. |
| [MDN `EXT_color_buffer_half_float`](https://developer.mozilla.org/en-US/docs/Web/API/EXT_color_buffer_half_float) | Half-float color buffers as alternative when full float buffer support is absent. |
| [KhronosGroup/WebGL#3093](https://github.com/KhronosGroup/WebGL/issues/3093) | Real divergence: devices may expose half-float paths without full `EXT_color_buffer_float`; do not assume one extension story. |

---

## Recommendation

Replace the single **boolean** “compat mode strips HDR” behavior with **graded tiers** driven by **framebuffer completeness tests**, while keeping `webgl_compatibility_mode` as a user **force-low** override if desired.

Shadows stay as-is until HDR path is merged; then re-audit shadow map format + compare against depth Sampler rules on WebGL2.

---

## Phased implementation plan

### Phase 1 — Inventory (half day)

- Diff `Renderer::new` vs `Renderer::new_webgl_compatibility` and list: HDR FBO, MRT, bloom ping-pong, FXAA, MSAA, post passes, shader variants (desktop vs `_es300.glsl`).
- Output a small matrix: feature × entry point × shaders involved.

### Phase 2 — Policy tiers (~1–2 days)

Introduce something like `WebRenderTier` (name flexible):

- Tier A: RGBA8, direct-to-default-FBO (current compat behavior).
- Tier B: HDR `RGBA16F` single attachment (tone map only).
- Tier C: MRT HDR + bloom (matches desktop subset).
- Tier D+: MSAA/Fxaa — **last**, after Tier C stable.

Wire `RenderTargetPolicy::for_capabilities` to use tier from probes + config override, instead of unconditionally disabling HDR in compat mode.

### Phase 3 — Capability probing (**core**) in `WebCanvasPlatform::load_gl`

In `src/platform.rs` wasm `web_canvas`:

- Call `gl.getSupportedExtensions()` and request relevant extensions (`EXT_color_buffer_half_float`, `EXT_color_buffer_float` as applicable).
- **Probe FBO completeness** at startup:
  - `RGBA16F` color texture + correct `texImage2D` type pairing for WebGL2.
  - Optional second color attachment (`COLOR_ATTACHMENT1`) + `glDrawBuffers` through existing shim.
- Store results on `GlCapabilities` (today wasm defaults set `supports_float_color_buffer: false` statically — replace with probe results).

Treat **successful `CHECK_FRAMEBUFFER_STATUS == COMPLETE`** as the gate, not extension string presence alone.

### Phase 4 — Unify HDR FBO allocation

Refactor `create_hdr_framebuffer` (or add `create_hdr_framebuffer_webgl2`) so one code path allocates HDR targets with desktop vs WebGL-legal `internalformat`/`format`/`type` combinations.

Re-enable bloom only when Tier C probes pass.

### Phase 5 — Performance switches

Add config knobs for wasm defaults:

- Bloom at half resolution, fewer mips, optional “HDR without bloom”.
- Prefer FXAA-only on web before multisampled float RTs.

### Phase 6 — QA matrix

Test Chrome / Firefox / Safari (Safari historically weirdest for FP render targets). Add a dev-only console report: tier chosen, probes, extension list summary.

---

## Definition of done

- With capable GPU + browser: wasm uses HDR framebuffer path (measurable: highlights not hard-clamped as in pure RGBA8), bloom optional.
- Without support: deterministic fallback to current RGBA8 path; **no** silent incomplete FBO/black screen.
- No regression: wasm boot, `Game::tick` timing, input, shadows still work.

---

## Files likely touched

- `src/platform.rs` — extension list, FBO probes, extend `GlCapabilities`.
- `src/renderer.rs` — `RenderTargetPolicy`, `create_hdr_framebuffer`, compat vs full paths, bloom enablement.
- `src/web_game.rs` — only if HUD/debug for tier reporting.
- Shader copies under `resources/shaders/` if post passes lack ES300 variants.

---

## Copy-paste prompt for the next agent

Use this as the opening message:

```
You’re continuing rust-opengl-engine: restore HDR (and dependents: bloom, tonemap/exposure) on WASM/WebGL2 while keeping stable fallbacks.

Read docs/wasm_hdr_shadows_handoff.md fully first.

Constraints:
- Do not add crates or bump dependency versions unless the user explicitly asks.
- Do not add #[allow(dead_code)] suppressions.

Current facts from the codebase:
- RenderTargetPolicy::for_capabilities in src/renderer.rs disables HDR/MRT/bloom/MSAA/FXAA whenever webgl_compat or is_gles_like.
- Shadows already work on the webgl compat path via render_world_webgl_compatibility.
- Wasm GlCapabilities defaults are static in platform.rs webgl2_defaults(); need real probing (extensions + framebuffer completeness), not booleans guessed from “WebGL 2”.
- RAF timing fix lives in WebGameRuntime in src/web_game.rs — preserve it.

Goals:
1) Replace blanket compat HDR-off with graded tiers backed by probes.
2) Implement WebGL capability probing at context init (attach test textures/FBO + checkFramebufferStatus; test MRT/drawBuffers if needed).
3) Refactor HDR FBO allocation to work on WebGL2 with legal format/type combos; add fallbacks.

Deliverables:
- Passing cargo check --target wasm32-unknown-unknown --no-default-features --features web and cargo check after changes.
- Short note in docs/wasm_hdr_shadows_handoff.md “Progress” section (optional) describing what tiers exist and probe results semantics — only if useful.

Start by inspecting RenderTargetPolicy, create_hdr_framebuffer, new_webgl_compatibility, and web_canvas Platform init.
```
