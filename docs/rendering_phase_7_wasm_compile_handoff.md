# Rendering Phase 7 WASM Compile Handoff

This handoff captures the recommended next step after the Web Build Prep:
Features, Asset Boundary, Platform Split work.

## Goal

Do the first real `wasm32` compile pass and use compiler errors to drive the
next small, reviewable PR.

This is still compile-focused prep work. Do not try to implement browser
runtime rendering, asset fetching, or desktop visual parity in this pass.

## Starting Commands

```powershell
rustup target add wasm32-unknown-unknown
cargo check --target wasm32-unknown-unknown --no-default-features --features web
```

## Expected Blockers

- `Cargo.toml` still has unconditional desktop dependencies:
  - `glutin`
  - `glutin-winit`
  - `imgui`
  - `imgui-winit-support`
  - `imgui-opengl-renderer`
  - possibly `libc` / FMOD-related pieces
- `src/main.rs` is still the native `winit::ApplicationHandler` entry point.
It likely needs a native-only cfg plus a minimal wasm stub entry point.
- Core modules still import `winit` input types directly, especially `KeyCode`
and mouse event types. A small engine input abstraction may be needed before
wasm can compile cleanly.
- `src/platform.rs` now has a `wasm32` WebCanvas placeholder, but it does not
create a real browser canvas or WebGL2 context yet.
- `assets::read_bytes` has a wasm-compatible compile shape, but intentionally
returns `Unsupported` on wasm. Runtime browser asset loading still needs a
later embedded-assets or fetch strategy.

## Recommended Next PR

### First WASM Check: Target-Scoped Desktop Dependencies And Native Entrypoint

Suggested scope:

1. Run the `wasm32-unknown-unknown` check command above.
2. Move desktop-only dependencies behind target-specific or feature-specific
  gates without changing dependency versions.
3. Gate the native `src/main.rs` event loop behind native cfgs.
4. Add only the smallest wasm entry stub needed to continue compilation.
5. If `winit` input types block shared modules, introduce a minimal internal
  input type boundary rather than threading browser-specific types through the
   engine.

## Guardrails

- Keep native desktop behavior as the default.
- Do not add, remove, or update dependencies without explicit approval.
- Do not re-enable ImGui/editor UI or the widget gallery.
- Do not chase renderer visual parity.
- Prefer cfg/feature/capability-driven gates over broad platform-name checks,
except at true target boundaries such as `wasm32` entry points and dependency
tables.
- Keep the change compile-focused and reviewable.

## Leave For Later

- Real browser canvas/WebGL2 context creation.
- HTTP/fetch asset loading.
- Embedded asset packaging strategy.
- Web audio.
- Browser input polish.
- Save/config persistence for web.
- Runtime smoke test in a browser.

## Validation

Run:

```powershell
cargo fmt
cargo check
cargo check --no-default-features
cargo check --target wasm32-unknown-unknown --no-default-features --features web
```

The wasm check does not need to be clean at the start of the task. Its compiler
errors are the task list. The PR is done when the selected blocker slice is
resolved and native checks still pass.