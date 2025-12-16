---
type: "always_apply"
---

# Rust OpenGL Engine - Augment Guidelines

## Workflow

1. **Always plan first** - Use sequential thinking to break down the task before making any code changes
2. **Gather context** - Use codebase-retrieval to understand existing patterns before implementing
3. **Execute incrementally** - Make changes in small, testable chunks
4. **Verify** - Build after changes to catch errors early

## Architecture Patterns

### UI Views and Managers
- `GameUiManager` orchestrates UI views (pause menu, HUD, etc.)
- Each view (e.g., `PauseMenuView`) handles its own logic internally
- Views receive a context struct with mutable refs to state they can modify
- Views directly modify state via context refs - no action flags returned to callers
- Slint UI uses software rendering to pixel buffers, then uploads to GL textures
- Each Slint component gets its own `MinimalSoftwareWindow`

### UI Event Handling Pattern
```
game.rs                    GameUiManager              PauseMenuView
   |                            |                          |
   |-- GameUiUpdateContext ---->|                          |
   |   (mutable refs to state)  |-- PauseMenuContext ----->|
   |                            |   (mutable refs)         |
   |                            |                          |-- directly modifies state
   |                            |                          |-- sends global messages
   |<-- global messages only ---|<-------------------------|
```

1. **game.rs passes mutable refs** via `GameUiUpdateContext` (paused, render_gizmos, etc.)
2. **Views directly modify state** for view-specific actions (unpause, toggle gizmos)
3. **Views send messages** only for global/cross-system actions (quit, reload world)
4. **game.rs handles only global messages** - never view-specific logic

### Message Queue
- Use `MessageQueue` for **global messages only** (e.g., `WindowShouldClose`, `ReloadWorldData`)
- Do NOT use for view-specific actions - those should modify state directly via context refs
- Messages are processed in the game loop's tick/update phase
- Keep `UiMessage` enum lean - only truly global events belong here

### Entity Manager
- ECS-like architecture with component hashmaps (transforms, animators, etc.)
- Use slotmap keys for entity references
- `serialize_entity_data` / `populate_entity_data` for persistence

### Game Structure
- `Game` struct owns all major systems (platform, renderer, world, physics, sound, UI managers)
- `game.tick()` handles update logic, `game.render()` handles drawing
- Pause state is managed via `game.paused` boolean
- Camera states: `CameraState::Third` (gameplay), `CameraState::Free` (editor mode)

## Code Style

- Use lowercase comments (e.g., `// create a new window` not `// Create a new window`)
- Prefer `&mut` references in context structs over returning action flags
- Keep game.rs lean - delegate logic to managers and systems
- Prefer self-documenting helper methods over inline comments:
  ```rust
  // bad: inline comments
  if self.close_pending.replace(false) {
      *ctx.paused = false; // unpause
  }

  // good: helper method
  self.handle_unpause(ctx.paused);
  ```

### OpenGL Calls
- **Always wrap OpenGL calls in the `gl_call!` macro** for error checking
- Example: `gl_call!(gl::bindTexture(gl::TEXTURE_2D, tex));`
- Never call raw `gl::*` functions directly without the macro
- The macro is defined in `src/macros.rs`

## Slint UI

- UI files in `resources/ui/` with `.slint` extension
- Include modules via `slint::include_modules!()`
- Callbacks use `Rc<Cell<bool>>` for interior mutability
- Process callbacks in update loop with `slint::platform::update_timers_and_animations()`

## Build & Test

- Run `cargo build` to verify changes compile
- Check for warnings - treat unused imports and variables as issues to fix
- The binary name is `learn-opengl-rs` (legacy from LearnOpenGL tutorials)
- There's an existing bug in Augment that polls the terminal over and over. when calling a build command, run it for a second, terminate it, then read from terminal to see the results.

## File Organization

- `src/ui/` - UI managers and Slint platform code
- `src/config/` - Configuration loading/saving (JSON, TOML)
- `src/state_machines/` - Player and enemy state machines
- `src/animation/` - Animation system and model loading
- `resources/` - Shaders, UI files, textures, models

