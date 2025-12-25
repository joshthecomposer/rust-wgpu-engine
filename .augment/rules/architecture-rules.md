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

### UI Performance Guidelines

**CRITICAL: The game runs at ~700 FPS. UI updates at this rate cause massive performance drops.**

#### Throttling Rules
1. **NEVER query entity manager every frame** - Always throttle expensive lookups
2. **NEVER render 3D content every frame** - Throttle portrait/model rendering to 30-60 Hz
3. **NEVER call Slint setters every frame** - Use change detection to only update when values change
4. **NEVER iterate entities every frame** - Cache results and throttle checks to 10-30 Hz

#### Throttling Pattern
```rust
// Add to struct
last_update_time: f64,
cached_result: T,

// In constructor
last_update_time: -999.0, // force first update

// In update method
const UPDATE_INTERVAL: f64 = 0.1; // 10 Hz
if elapsed_time - self.last_update_time >= UPDATE_INTERVAL {
    self.cached_result = expensive_operation();
    self.last_update_time = elapsed_time;
}
// Use self.cached_result between updates
```

#### Change Detection Pattern
```rust
// Add to struct
cached_data: Cell<Option<T>>,

// In update method
let new_data = compute_data();
let needs_update = match self.cached_data.take() {
    Some(cached) => cached != new_data,
    None => true, // first update
};

if needs_update {
    slint_component.set_property(new_data.value);
}
self.cached_data.set(Some(new_data));
```

#### Recommended Update Rates
- **Ability cooldowns**: 10 Hz (100ms is imperceptible for cooldown animations)
- **3D portraits**: 30 Hz (smooth enough for small animated portraits)
- **Pickup/proximity checks**: 10 Hz (100ms delay is acceptable for prompts)
- **Health/mana bars**: Change detection only (update when values change)
- **Text labels**: Change detection only (update when text changes)

#### Anti-Patterns to Avoid
❌ **BAD**: Calling `entity_manager.has_nearby_weapon()` every frame (700 Hz)
✅ **GOOD**: Throttle to 10 Hz and cache result

❌ **BAD**: Rendering 3D portrait every frame (700 Hz)
✅ **GOOD**: Throttle to 30 Hz

❌ **BAD**: Calling `set_health()` every frame even when health unchanged
✅ **GOOD**: Use change detection to only call when health changes

❌ **BAD**: Iterating all entities to find player every frame
✅ **GOOD**: Cache player ID or throttle lookup

#### When Adding New UI Features
1. **Ask**: Does this need to update every frame?
2. **Measure**: What's the performance cost of the operation?
3. **Throttle**: If expensive, add throttling (10-30 Hz is usually fine)
4. **Cache**: If the result rarely changes, use change detection
5. **Test**: Verify FPS impact is minimal (<5 FPS drop)

#### Existing Optimizations (DO NOT REMOVE)
The following systems are already optimized and should serve as examples:

1. **AbilityBarRenderer** (`src/ui/ability_bar_renderer.rs`)
   - Throttled to 10 Hz via `UPDATE_INTERVAL = 0.1`
   - Uses `should_update()` method to check throttle
   - Caches ability slot data and show state
   - Only queries entity manager when throttle interval passes

2. **PortraitRenderer** (`src/ui/portrait_renderer.rs`)
   - Throttled to 30 Hz via `UPDATE_INTERVAL = 1.0 / 30.0`
   - Uses `should_update()` method to check throttle
   - Only renders 3D model when throttle interval passes

3. **PlayerHudView** (`src/ui/game/views/player_hud.rs`)
   - Uses change detection via `cached_data: Cell<Option<PlayerHudData>>`
   - Only calls Slint setters when health/mana/etc. actually change
   - Compares new data with cached data before updating

4. **GameRootView pickup indicator** (`src/ui/game/views/game_root.rs`)
   - Throttled to 10 Hz via `PICKUP_CHECK_INTERVAL = 0.1`
   - Caches `show_pickup_prompt` result between checks
   - Only calls `has_nearby_weapon()` when throttle interval passes

5. **ToastView** (`src/ui/game/views/toast.rs`)
   - Uses change detection via `needs_sync` flag
   - Only syncs to Slint when toast states change

**If you modify any of these systems, maintain the throttling/caching patterns!**

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

