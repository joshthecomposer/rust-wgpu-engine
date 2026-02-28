# Rust OpenGL Engine & Custom UI - Copilot Instructions

This repository features a **high-performance custom game engine** written in Rust (2021) using OpenGL.
**Crucially**, it uses a **custom GPU-rendered UI system** (replacing Slint). UI layouts are defined in **RON (Rusty Object Notation)** and deserialized into a widget tree.

## High Level Details

-   **Language:** Rust (2021 edition).
-   **Graphics:** OpenGL (via `gl` crate), raw calls wrapped in `gl_call!` macro.
-   **UI Framework:** Custom In-House GPU System.
    -   **Definition:** RON files in `resources/ui/`.
    -   **Source:** `src/ui/game_new/`.
    -   **Documentation:** `src/ui/game_new/doc/`.
-   **Architecture:** ECS-like Entity Manager, centralized Game struct.

## Build & Validation

The agent should follow these steps to build and verify changes.

### Build Commands
1.  **Check:** Always run `cargo check` first.
2.  **Test UI Parser:** Run `cargo test ron_loader` when modifying RON files or parser logic.
3.  **Build:** `cargo build`. (Binary name: `learn-opengl-rs`).
4.  **Run:** `cargo run`.
    * *Note:* If the build process polls the terminal indefinitely, terminate and read the log.

### Verification Steps
1.  **RON Syntax:** Ensure all `Option<T>` fields in RON are wrapped in `Some(...)`.
2.  **Layout:** Verify grid spans are used correctly in Rows (Columns default to `span: 12` unless specified).

## Project Layout & Documentation

**The Agent must prioritize reading the local documentation before implementation.**

### Key Documentation Locations
* `src/ui/game_new/doc/architecture.md` - Core rendering cycle (Layout -> Update -> Render).
* `src/ui/game_new/doc/ron_format.md` - Syntax reference, `Some()` wrapping rules, node definitions.
* `src/ui/game_new/doc/widgets.md` - Properties for `Row`, `Column`, `Box`, `TextureRect`, `AbilitySlot`, etc.
* `src/ui/game_new/doc/troubleshooting.md` - Common layout gotchas (margins, borders, stacking).

### Directory Structure
* `src/game.rs`: Main loop.
* `src/ui/game_new/parser/`: RON deserialization logic.
* `src/ui/game_new/widgets/`: Widget trait implementations.
* `resources/ui/`: UI layout definitions (.ron).

## Critical Guidelines for Custom UI

### 1. RON Syntax Rules
* **No Tag Field:** Do NOT add a `type: "Row"` field. The struct variant name IS the tag.
    * *Wrong:* `(type: "Row", ...)`
    * *Correct:* `Row( ... )`
* **Option Wrapping:** Most style properties are `Option<T>`. You **MUST** wrap values in `Some()`.
    * *Wrong:* `style: ( width: Px(10.0) )`
    * *Correct:* `style: ( width: Some(Px(10.0)) )`
* **Struct Variants:** Use struct syntax, not tuple syntax.
    * *Wrong:* `Row(RowDef)`
    * *Correct:* `Row( style: (...), children: [...] )`

### 2. Layout & Styling
* **Row vs Column:**
    * `Row`: Horizontal layout. Uses `justify` to space children.
    * `Column`: Vertical layout. When inside a `Row`, uses `span: 1-12` for grid sizing.
* **Grid Spans:** If a `Column` is inside a `Row`, `width` is ignored in favor of `span`.
    * `span: 6` = 50% width.
    * `span: 12` (default) = 100% width.
* **Borders:** If using borders on transparent backgrounds, the engine renders 4 outline rects. If opaque, it renders a fill + border.

### 3. Special Widgets
* **TextureRect:** Requires `flip_v: true` if rendering an FBO/Render Target (textures are upside down in OpenGL).
* **ProgressBar:** Text is auto-centered. Explicit height is preserved regardless of margins.
* **TooltipManager:** Renders specifically named styling (Shadows + Accent Bar). Do not reimplement logic; use the manager.

### 4. Code Style
* **OpenGL:** NEVER call raw `gl::*`. Always use `gl_call!(...)`.
* **Comments:** Use lowercase comments (e.g., `// update ui state`).
* **Throttling:** Do not rebuild UI meshes every frame unless necessary. Use dirty flags.

## Workflow for Agent
1.  **Consult Docs:** Read `src/ui/game_new/doc/widgets.md` to find the correct properties for the widget you are editing.
2.  **Check Syntax:** Verify `ron_format.md` to ensure `Some()` wrappers are present.
3.  **Implement:** Modify the `.ron` file or Rust widget logic.
4.  **Verify:** Run `cargo test ron_loader` to catch deserialization errors immediately.