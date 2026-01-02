# RON Format Reference

## Overview

Views are defined using RON (Rusty Object Notation), which deserializes directly to Rust structs via serde.

## Basic Syntax

```ron
WidgetType(
    property: value,
    style: (
        width: Px(100.0),
        height: Percent(50.0),
    ),
    children: [
        ChildWidget(...),
    ]
)
```

## Widget Types

### Row

Horizontal flex container. Children are laid out left-to-right.

```ron
Row(
    style: (...),
    justify: Center,      // Start, Center, End, SpaceBetween, SpaceAround
    align: Center,        // Start, Center, End
    children: [...]
)
```

### Column

Vertical flex container or grid cell. When inside a Row, can specify a grid span (1-12).

```ron
Column(
    style: (...),
    span: 6,              // 1-12, how many grid columns to span
    justify: Start,
    align: Start,
    children: [...]
)
```

### Text
Displays a text string.
```ron
Text(
    content: "Hello World",
    style: (
        color: Some(Rgba(1.0, 1.0, 1.0, 1.0)),
        font_size: Some(24.0),
        font_family: Some("weiholmir"),
        text_align: Some(Center),
    )
)
```

### Label
Wrapper around Text.
```ron
Label(
    content: "Label Text",
    style: (
        font_size: Some(14.0),
    )
)
```

### Box

Simple colored rectangle. A leaf widget with no children.

```ron
Box(
    style: (
        width: Px(100.0),
        height: Px(50.0),
        background: Rgba(1.0, 0.0, 0.0, 1.0),
    )
)
```

## Style Properties

The `Style` struct supports the following properties:

### Common Properties (All Widgets)
- `id` (Option<String>): Unique identifier for the widget. Used for debugging (logs on click) and event routing.
- `width` (Length): Width of the widget.
- `height` (Length): Height of the widget.
- `background` (Color): Background color.


- `Px(f32)` - Absolute pixels
- `Percent(f32)` - Percentage of parent (0.0 - 100.0)
- `Auto` - Automatic sizing based on content
- `Variable(String)` - Reference to a theme variable (e.g., `Variable("spacing-sm")`)

## Color Values

- `Rgba(r, g, b, a)` - RGBA values from 0.0 to 1.0
- `Hex("#RRGGBB")` - Hex color string
- `Hex("#RRGGBBAA")` - Hex color with alpha
- `Variable(String)` - Reference to a theme variable (e.g., `Variable("panel-background")`)

## Example View

```ron
Row(
    style: (
        width: Percent(100.0),
        height: Px(100.0),
        padding: Px(8.0),
        background: Rgba(0.1, 0.1, 0.1, 0.9),
    ),
    justify: SpaceBetween,
    children: [
        Column(
            span: 4,
            style: (background: Rgba(1.0, 0.0, 0.0, 1.0)),
        ),
        Column(
            span: 4,
            style: (background: Rgba(0.0, 1.0, 0.0, 1.0)),
        ),
        Column(
            span: 4,
            style: (background: Rgba(0.0, 0.0, 1.0, 1.0)),
        ),
    ]
)
```

## Implementation Details

### Serde Deserialization

The RON format uses **standard Rust enum deserialization** where the variant name (`Row`, `Column`, `Box`) acts as the tag. This is achieved by defining `NodeDefinition` as an enum with struct variants:

```rust
#[derive(Debug, Deserialize)]
pub enum NodeDefinition {
    Row {
        #[serde(default)]
        style: Style,
        #[serde(default)]
        justify: Alignment,
        #[serde(default)]
        align: Alignment,
        #[serde(default)]
        children: Vec<NodeDefinition>,
    },
    Column { /* ... */ },
    Box { /* ... */ },
}
```

**Important:** We do NOT use `#[serde(tag = "type")]` because that would require an explicit `type` field in the RON:

```ron
// ❌ WRONG - This would require #[serde(tag = "type")]
(
    type: "Row",
    style: (...),
    children: [...]
)

// ✅ CORRECT - Standard enum deserialization
Row(
    style: (...),
    children: [...]
)
```

### Default Values

All fields use `#[serde(default)]`, which means:
- Omitted fields will use the type's `Default` implementation
- You can write minimal RON files by only specifying what you need
- Example: `Row(children: [])` is valid and will use default style/alignment

### Column Span Behavior

The `span` field for `Column` widgets has special handling:
- If omitted, defaults to `0` (via `#[serde(default)]`)
- The `build_widget` function converts `span == 0` to `span == 12` (full width)
- Valid range: 1-12 (12-column grid system)

## Troubleshooting

### "Unexpected missing field `type`"

If you see this error, it means the Rust enum has `#[serde(tag = "type")]` but the RON uses the variant-name syntax. The fix is to remove the `#[serde(tag = "type")]` attribute from `NodeDefinition`.

### "Expected struct variant, found tuple variant"

This error occurs when the enum variants are defined as tuple variants (e.g., `Row(RowDef)`) instead of struct variants (e.g., `Row { style: Style, ... }`). Make sure `NodeDefinition` uses struct variants to match the RON syntax.

### Parser Accepts RON but Nothing Renders

If parsing succeeds but you see no UI:
1. Check that `UiTree::set_screen_size()` is called with the correct dimensions
2. Verify `UiTree::layout()` is called before `UiTree::render()`
3. Add debug logging in `RenderBatch::push_rect()` to see if geometry is being generated
4. Ensure the UI renderer's OpenGL state is correct (blending, viewport, etc.)


