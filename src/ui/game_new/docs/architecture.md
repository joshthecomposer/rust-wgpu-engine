# Custom GPU UI System - Architecture

## Overview

This is a GPU-rendered UI system designed to replace Slint's software rendering approach. It uses OpenGL for all rendering, with RON markup files defining the UI structure.

## Core Concepts

### Three-Phase Update Cycle

Every frame, the UI goes through three phases:

1. **Layout Phase** - Calculate positions and sizes of all widgets
2. **Update Phase** - Process input events and update widget state  
3. **Render Phase** - Batch geometry and issue GPU draw calls

### Widget Tree

The UI is organized as a tree of widgets. The root widget contains children, which may contain their own children, forming a hierarchy.

```
UiTree
└── Root Widget (e.g., Row)
    ├── Child Widget (e.g., Column)
    │   └── Leaf Widget (e.g., Box)
    └── Child Widget (e.g., Column)
```

### RON Markup

Views are defined in `.ron` files that get deserialized into widget trees at runtime. This allows for declarative UI definition similar to HTML/CSS.

## Module Structure

- `widgets/` - Widget trait and implementations (Box, Row, Column)
- `styles/` - CSS-like style properties (Length, Color, Style)
- `layout/` - Layout computation engine (CSS box model, flexbox)
- `render/` - OpenGL rendering (batching, shaders)
- `parser/` - RON file loading and deserialization
- `views/` - RON view definition files

## Data Flow

```
RON File → Parser → Widget Tree → Layout → Render Batch → GPU
```

## Integration

The custom UI system runs alongside the existing Slint UI during migration. It hooks into the same winit event loop and renders after the main 3D scene.

## Testing

Unit tests for the RON parser are located in `src/ui/game_new/parser/ron_loader.rs`. These tests verify:
- Simple widget parsing (Row, Column, Box)
- Nested widget structures
- Default value handling
- The actual `test_view.ron` file parsing

Run tests with: `cargo test ron_loader`

## Current Status

✅ **Working:**
- RON parsing with `Row(...)` syntax (standard enum deserialization)
- Widget tree construction (Row, Column, Box)
- Layout engine (basic box model)
- GPU rendering (batched quads with OpenGL)
- Test view rendering (see `views/test_view.ron`)

🚧 **In Progress:**
- Additional widget types (Text, Button, Image, etc.)
- Event handling (mouse clicks, hover states)
- Animation system
- Advanced layout features (flexbox, grid)

