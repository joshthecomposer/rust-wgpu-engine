# Style Properties Reference

## Sizing

| Property | Type | Description |
|----------|------|-------------|
| `width` | Length | Widget width |
| `height` | Length | Widget height |
| `min_width` | Length | Minimum width constraint |
| `min_height` | Length | Minimum height constraint |
| `max_width` | Length | Maximum width constraint |
| `max_height` | Length | Maximum height constraint |

## Spacing

| Property | Type | Description |
|----------|------|-------------|
| `margin` | Length | Margin on all sides |
| `margin_top` | Length | Top margin |
| `margin_right` | Length | Right margin |
| `margin_bottom` | Length | Bottom margin |
| `margin_left` | Length | Left margin |
| `padding` | Length | Padding on all sides |
| `padding_top` | Length | Top padding |
| `padding_right` | Length | Right padding |
| `padding_bottom` | Length | Bottom padding |
| `padding_left` | Length | Left padding |

## Visual

| Property | Type | Description |
|----------|------|-------------|
| `background` | Color | Background color |
| `border_color` | Color | Border color (future) |
| `border_width` | Length | Border width (future) |
| `border_radius` | Length | Corner radius (future) |

## Length Values

```ron
// Absolute pixels
width: Px(100.0)

// Percentage of parent
width: Percent(50.0)

// Auto-size based on content
width: Auto

// Theme variable
width: Variable("spacing-md")
```

## Color Values

```ron
// RGBA (0.0 - 1.0)
background: Rgba(1.0, 0.5, 0.0, 1.0)

// Hex string
background: Hex("#FF8000")

// Hex with alpha
background: Hex("#FF8000FF")

// Theme variable
background: Variable("accent")
```

## Default Values

If a property is not specified, these defaults are used:

- `width`: `Auto`
- `height`: `Auto`
- `margin_*`: `Px(0.0)`
- `padding_*`: `Px(0.0)`
- `background`: `Rgba(0.0, 0.0, 0.0, 0.0)` (transparent)

