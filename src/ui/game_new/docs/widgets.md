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
- `style`: Standard style properties (width, height, margin, etc.).

**Example:**
```ron
TextureRect(
    texture_id: 123,
    style: (
        width: Px(64.0),
        height: Px(64.0),
    )
)
```

### ProgressBar
A bar that visualizes a value within a range.

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
        height: Px(20.0),
        background: Rgba(0.1, 0.1, 0.1, 1.0), // Empty background color
    )
)
```
