# UI Widgets

This document describes the available widgets in the custom UI system.

## Basic Widgets

### Box
A generic container that can have a background color and hold content.
```ron
Box(
    style: (
        width: Px(100.0),
        height: Px(100.0),
        background: Rgba(1.0, 0.0, 0.0, 1.0),
    )
)
```

### Row / Column
Layout containers for arranging children horizontally or vertically.
```ron
Row(
    justify: Center,
    children: [ ... ]
)
```

### Label
Displays text.
```ron
Label(
    content: "Hello",
    style: ( font_size: Some(24.0) )
)
```

## Graphic Widgets

### TextureRect
Renders an OpenGL texture (by ID) within a rectangle. Useful for displaying FBO results (like 3D portraits) or loaded textures.

**Properties:**
- `texture_id`: `u32` - The OpenGL texture ID to render.
- `flip_v`: `bool` - Whether to flip the texture vertically. **Required for FBO textures** as they render upside-down by default.
- `style`: Standard style properties (width, height, margin, etc.).

**Example:**
```ron
TextureRect(
    texture_id: 123,
    flip_v: true,  // Required for FBO/render target textures
    style: (
        width: Px(64.0),
        height: Px(64.0),
    )
)
```

> **Note:** When placing TextureRect inside a bordered Column, add `padding` to the Column equal to or greater than `border_width` to prevent the texture from overlapping the border.

### ProgressBar
A bar that visualizes a value within a range. Displays centered text showing "current / max".

**Properties:**
- `current_value`: `f32` - The current progress value.
- `max_value`: `f32` - The maximum value (100% full).
- `fill_color`: `Color` - Color of the filled portion.
- `outline_color`: `Color` - Color of the border/outline.
- `style`: Standard style properties.

**Example:**
```ron
ProgressBar(
    current_value: 75.0,
    max_value: 100.0,
    fill_color: Rgba(0.8, 0.2, 0.2, 1.0),
    outline_color: Rgba(1.0, 1.0, 1.0, 1.0),
    style: (
        width: Px(200.0),
        height: Px(22.0),  // Recommended minimum for text visibility
        background: Rgba(0.1, 0.1, 0.1, 1.0),
    )
)
```

> **Note:** When adding margins to ProgressBars, explicit Px heights are correctly preserved. The text is automatically measured and centered both horizontally and vertically.

## Container Widgets

### ScrollView
A scrollable container that clips children to a viewport using GPU scissor testing. Includes a draggable scrollbar.

**Properties:**
- `content_height`: `f32` - The total height of the scrollable content area.
- `justify`: `Alignment` - Vertical alignment of children (Start, Center, End, SpaceBetween, SpaceAround).
- `style`: Standard style properties. `width` and `height` define the viewport (visible) size.
- `children`: Child widgets to render inside the scrollable area.

**Behavior:**
- Children outside the viewport are clipped (not visible).
- A scrollbar appears on the right edge when `content_height > viewport_height`.
- Click and drag the scrollbar thumb to scroll.
- Click on the scrollbar track to jump to that position.

**Example:**
```ron
ScrollView(
    content_height: 500.0,
    style: (
        id: Some("my_scroll"),
        width: Px(300.0),
        height: Px(200.0),
        background: Rgba(0.1, 0.1, 0.1, 0.9),
    ),
    children: [
        Label(content: "Item 1", style: ( color: Some(Rgba(1.0, 1.0, 1.0, 1.0)) )),
        Label(content: "Item 2", style: ( color: Some(Rgba(1.0, 1.0, 1.0, 1.0)) )),
        Label(content: "Item 3", style: ( color: Some(Rgba(1.0, 1.0, 1.0, 1.0)) )),
        // ... more items
    ]
)
```

## Game Widgets

### AbilitySlot
A single ability slot widget for ability bars. Displays icon, cooldown overlay, key label, and ready glow effect.

**Properties:**
- `key_label`: `String` - Key binding display (e.g., "M1", "Q", "Shift").
- `slot_background`: `Color` - Background color of the slot.
- `ready_border_color`: `Color` - Border color when ability is ready.
- `normal_border_color`: `Color` - Border color when ability is not ready.
- `style`: Standard style properties.

**Dynamic Properties (set via Rust):**
- `texture_id`: `u32` - Icon texture ID.
- `cooldown_progress`: `f32` - 0.0 = ready, 1.0 = full cooldown.
- `is_ready`: `bool` - Triggers pulsing gold glow when true.
- `ability_id`: `String` - Fallback display when no icon.
- `ability_name`, `ability_description`: Tooltip content.

**Example:**
```ron
AbilitySlot(
    key_label: "Q",
    slot_background: Variable("deep-void-alpha"),
    ready_border_color: Variable("runic-gold"),
    normal_border_color: Variable("stone-light"),
    style: (
        id: Some("slot_q"),
        width: Px(48.0),
        height: Px(48.0),
        border_radius: 4.0,
    ),
)
```

### TooltipManager
A tooltip overlay widget for displaying contextual information on hover.

**Visual Design:**
- Drop shadow: 4px offset (right and down), 50% opacity black
- Left accent bar: 4px wide, runic-gold color
- Sharp corners (no border-radius)
- Positioned above the hovered widget

**Properties:**
- `background_color`: `Color` - Tooltip background.
- `border_color`: `Color` - Tooltip border.
- `title_color`: `Color` - Title text color.
- `description_color`: `Color` - Description text color.
- `style`: Standard style properties.

**Usage (in Rust):**
```rust
// During update phase
tooltip.begin_frame();
if hovered {
    tooltip.show("Ability Name", "Description", widget_rect);
}
// During render phase
tooltip.render(renderer);
```

**Example:**
```ron
TooltipManager(
    background_color: Variable("deep-void"),
    border_color: Variable("stone-light"),
    title_color: Variable("runic-gold"),
    description_color: Variable("old-text"),
    style: (id: Some("tooltip_overlay")),
)
```

> **Note:** TooltipManager should be rendered last to appear above all other UI elements.

---

## Style Properties

### margin_left
Creates space before a widget. Useful for spacing items within a Row.

```ron
AbilitySlot(
    key_label: "M2",
    style: (
        width: Px(48.0),
        height: Px(48.0),
        margin_left: Some(Px(6.0)),  // 6px gap before this slot
    ),
)
```

> **Note:** The Row widget respects `margin_left` by calculating each child's "footprint" (margin + content width) and positioning children based on where the previous child's rect ends.

