# Troubleshooting & Common Gotchas

This document captures debugging learnings and common pitfalls when working with the custom UI system.

## Layout Issues

### Row Layout: Grid Spans Control Width Distribution

**Problem:** Columns inside a Row don't respect their `width` style properties.

**Cause:** When all children of a Row are Columns, the Row uses a **grid span system** for width distribution instead of individual width properties. Each Column defaults to `span: 12`, so two Columns get 50% each regardless of their `width` settings.

**Solution:** Set explicit `span` values:
```ron
Row(
    children: [
        Column(
            span: 3,  // Gets 25% of width (3/12)
            style: (width: Px(56.0)),  // This is ignored when span is set
        ),
        Column(
            span: 9,  // Gets 75% of width (9/12)
        ),
    ]
)
```

---

### Margins Affect Available Height (Fixed in ProgressBar)

**Problem:** Widgets with `margin_top` or `margin_bottom` appear shorter than expected.

**Cause:** The old calculation was:
```rust
let max_height = available.height - mt - mb;
let height = self.style.height.resolve_or(max_height, max_height);
self.rect.height = height.min(max_height);  // Clamps by margin-reduced height!
```

For a 22px height bar with 6px margin_top in a 22px available space, max_height would be 16px, and the bar would be clamped to 16px.

**Solution (implemented in progress_bar.rs):** For explicit Px values, only clamp by total available height, not margin-reduced height. Margins should affect positioning, not final size.

---

### Column with height: Auto Ignores justify Property

**Problem:** Setting `justify: Center` on a Column doesn't vertically center children.

**Cause:** When a Column has `height: Auto`, its height is calculated from the sum of children heights plus spacing. There's no "extra space" to distribute, so `justify` has no effect.

**Solution:** Either:
1. Give the Column a fixed height (`height: Px(100.0)`) so there's space to distribute
2. Use margins on children to control spacing instead

---

### Text Widget with Excessive Height

**Problem:** Text widgets take up way more vertical space than the text content.

**Cause:** Bug in `Text::layout()` where `resolve_or` arguments were swapped:
```rust
// WRONG - defaults to max_available when Auto
let height = self.style.height.resolve_or(measured_height, max_available_height);

// CORRECT - defaults to measured when Auto  
let height = self.style.height.resolve_or(max_available_height, measured_height);
```

**Solution:** Fixed in `text.rs`. The correct signature is `resolve_or(parent_size, default)` where `parent_size` is used for percentage calculation and `default` is used when value is `Auto`.

---

## Border Rendering

### Portrait/Image Overlaps Border

**Problem:** Content inside a bordered Column draws on top of the border instead of inside it.

**Cause:** The Column has a border but no padding, so children start at the Column's edge (where the border is).

**Solution:** Add padding equal to or greater than the border width:
```ron
Column(
    style: (
        border_width: 2.0,
        border_color: Variable("gold"),
        padding: Px(2.0),  // Inset content by border width
    ),
    children: [
        TextureRect(...)  // Now renders inside the border
    ]
)
```

---

### Transparent Background with Border Shows Filled Rectangle

**Problem:** A Column with a transparent background and border renders as a filled rectangle instead of just the border outline.

**Cause:** The render logic was drawing boundary -> background even for transparent backgrounds, filling the entire area with the border color.

**Solution (implemented in column.rs and row.rs):** When background is transparent, render border as 4 separate rectangles (outline) instead of a filled rectangle:
```rust
if has_border && has_opaque_bg {
    // Opaque bg: draw border fill, then bg on top
    renderer.draw_rect(self.rect, border_color, radius);
    renderer.draw_rect(bg_rect.shrink(border_width), bg_color, inner_radius);
} else if has_border {
    // Transparent bg: draw 4 rectangles for outline only
    renderer.draw_rect(top_edge, border_color, 0.0);
    renderer.draw_rect(bottom_edge, border_color, 0.0);
    renderer.draw_rect(left_edge, border_color, 0.0);
    renderer.draw_rect(right_edge, border_color, 0.0);
}
```

---

## ProgressBar

### Text Not Centered in Bar

**Problem:** The value text ("100 / 100") isn't horizontally or vertically centered.

**Cause:** Original code used approximate character width calculation which was inaccurate.

**Solution (implemented):** Measure text during layout using `font_system.measure_text()` and cache the dimensions for use during render:
```rust
// In layout()
let text = format!("{} / {}", current, max);
self.text_size = font_system.measure_text(&text, 11.0, None);

// In render()
let text_x = self.rect.x + (self.rect.width - self.text_size.0) / 2.0;
let text_y = self.rect.y + (self.rect.height - self.text_size.1) / 2.0;
```

---

## RON Syntax Reminders

### Optional Values Need Some() Wrapper

Many style properties are `Option<T>` and require `Some()` in RON:
```ron
// WRONG
style: (
    font_size: 14.0,
    color: Variable("highlight"),
)

// CORRECT
style: (
    font_size: Some(14.0),
    color: Some(Variable("highlight")),
)
```

Check `style_properties.md` for which properties require `Some()`.

---

## Debugging Tips

1. **Add debug prints in layout()** to see what dimensions are being calculated
2. **Check the grid span** when Columns in a Row don't size correctly
3. **Verify border_width < padding** to ensure content doesn't overlap borders
4. **Use fixed heights** on Columns when you want `justify` to work
5. **Check resolve_or argument order** - it's `(parent_size, default)`

---

## Row Layout Issues

### Children with Margins Not Spacing Correctly

**Problem:** AbilitySlots with `margin_left: Some(Px(6.0))` all appear squished together, or later children get progressively smaller.

**Cause:** The Row's first-pass width calculation was only using `child.rect().width` (content width), ignoring the margin offset. When the child applies its margin during the second pass, it shifts right but the Row didn't allocate enough space, causing overlaps.

**Solution (implemented in row.rs):** Calculate child "footprint" including margin:
```rust
// Calculate child's total "footprint" including any margin offset
let margin_offset = child.rect().x - temp_rect.x;
let footprint = margin_offset + child.rect().width;
```

Then position children based on where the previous child's rect actually ends:
```rust
let child_end = child.rect().x + child.rect().width - inner_rect.x;
x_offset = child_end + gap;
```

---

### Separator Box Elements Not Creating Gaps

**Problem:** `Box` widgets used as separators between ability slots don't create visible gaps.

**Cause:** Row's `justify: Center` distributes space between children, which counteracts the separator's purpose. With centered justification, children are spread evenly regardless of separator widths.

**Solution:** Use `justify: Start` so children pack from the left, allowing separator Box widths to create actual gaps:
```ron
Row(
    justify: Start,  // NOT Center
    children: [
        AbilitySlot(...),
        AbilitySlot(...),
        Box(style: (width: Px(12.0), height: Px(48.0))),  // Creates 12px gap
        AbilitySlot(...),
    ]
)
```

---

## Tooltip Issues

### Double Tooltips Appearing

**Problem:** Two tooltips appear when hovering over ability bar slots - one from the old Slint UI and one from the new GPU UI.

**Cause:** Both tooltip systems were active:
1. The Slint `GameRoot` component has `AbilityTooltip` components that show on hover
2. The new `TooltipManager` also renders tooltips based on `AbilitySlot.get_tooltip_info()`

**Solution:** Stop syncing ability data to the old Slint system. In `game_root.rs`, comment out or remove the `set_ability_slot_*` calls:
```rust
// NOTE: Slint ability bar tooltips are DISABLED - we now use the new GPU-based tooltips
// if needs_ability_sync {
//     self.game_root.set_ability_slot_m1(ability_data.m1.to_slint(ctx.image_cache));
//     ... etc
// }
```

---

### Tooltip Missing Styling (Shadow, Accent Bar)

**Problem:** Tooltip looks plain compared to the original Slint design - missing drop shadow and left accent bar.

**Solution (implemented in tooltip_manager.rs):** Add visual elements in render():
```rust
// Draw drop shadow first (offset 4px right and down)
let shadow_rect = Rect::new(
    self.tooltip_rect.x + 4.0,
    self.tooltip_rect.y + 4.0,
    self.tooltip_rect.width,
    self.tooltip_rect.height,
);
renderer.draw_rect(shadow_rect, [0.0, 0.0, 0.0, 0.5], 0.0);

// Draw left accent bar (4px wide runic-gold)
let accent_rect = Rect::new(
    self.tooltip_rect.x + border_width,
    self.tooltip_rect.y + border_width,
    4.0,
    self.tooltip_rect.height - border_width * 2.0,
);
renderer.draw_rect(accent_rect, [0.85, 0.68, 0.42, 1.0], 0.0);
```

Remember to add the accent width to text positioning:
```rust
let text_offset_x = accent_width + 4.0;
renderer.draw_text(&self.title, self.title_pos.0 + text_offset_x, ...);
```

