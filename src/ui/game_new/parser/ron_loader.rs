use std::path::Path;

use serde::Deserialize;

use crate::assets;
use crate::ui::game_new::parser::theme::{load_theme, Theme};
use crate::ui::game_new::styles::{Alignment, Color, Length, ScrollbarStyle, Style};
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::{
    AbilitySlot, BoxWidget, Checkbox, CloseButton, Column, ComboBox, DiamondWidget, Label,
    MenuButton, ProgressBar, Row, ScrollView, Slider, TabView, Text, TextureRect, ToastContainer,
    TooltipManager, Widget,
};

/// Represents a widget definition parsed from RON.
///
/// This enum uses standard Rust enum deserialization, matching the RON syntax:
/// ```ron
/// Row(
///     style: (...),
///     justify: Center,
///     children: [...]
/// )
/// ```
///
/// Note: We do NOT use `#[serde(tag = "type")]` because that would require
/// an explicit `type` field in the RON, which is not the documented format.
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
    Column {
        #[serde(default)]
        style: Style,
        /// Grid span (1-12). Defaults to 12 if not specified or if 0.
        #[serde(default)]
        span: u8,
        #[serde(default)]
        justify: Alignment,
        #[serde(default)]
        align: Alignment,
        #[serde(default)]
        children: Vec<NodeDefinition>,
    },
    Box {
        #[serde(default)]
        style: Style,
    },
    Diamond {
        #[serde(default)]
        style: Style,
    },
    Text {
        content: String,
        #[serde(default)]
        style: Style,
    },
    Label {
        content: String,
        #[serde(default)]
        style: Style,
    },
    TextureRect {
        texture_id: u32,
        #[serde(default)]
        style: Style,
        #[serde(default)]
        flip_v: bool,
    },
    ProgressBar {
        #[serde(default)]
        current_value: f32,
        #[serde(default)]
        max_value: f32,
        #[serde(default)]
        fill_color: Color,
        #[serde(default)]
        outline_color: Color,
        #[serde(default)]
        style: Style,
    },
    ScrollView {
        content_height: f32,
        #[serde(default)]
        justify: Alignment,
        #[serde(default)]
        style: Style,
        #[serde(default)]
        scrollbar_style: ScrollbarStyle,
        #[serde(default)]
        children: Vec<NodeDefinition>,
    },
    AbilitySlot {
        #[serde(default)]
        key_label: String,
        #[serde(default)]
        slot_background: Color,
        #[serde(default)]
        ready_border_color: Color,
        #[serde(default)]
        normal_border_color: Color,
        #[serde(default)]
        style: Style,
    },
    TooltipManager {
        #[serde(default)]
        background_color: Color,
        #[serde(default)]
        border_color: Color,
        #[serde(default)]
        title_color: Color,
        #[serde(default)]
        description_color: Color,
        #[serde(default)]
        style: Style,
    },
    MenuButton {
        text: String,
        #[serde(default)]
        normal_background: Color,
        #[serde(default)]
        hover_background: Color,
        #[serde(default)]
        accent_color: Color,
        #[serde(default)]
        border_color: Color,
        #[serde(default)]
        text_color: Color,
        #[serde(default)]
        style: Style,
    },
    CloseButton {
        #[serde(default)]
        normal_color: Color,
        #[serde(default)]
        hover_color: Color,
        #[serde(default)]
        style: Style,
    },
    ComboBox {
        #[serde(default)]
        options: Vec<String>,
        #[serde(default)]
        selected_index: usize,
        #[serde(default)]
        dropdown_background: Color,
        #[serde(default)]
        item_hover_color: Color,
        #[serde(default)]
        text_color: Color,
        #[serde(default)]
        border_color: Color,
        #[serde(default)]
        style: Style,
    },
    Checkbox {
        #[serde(default)]
        checked: bool,
        #[serde(default)]
        background_color: Color,
        #[serde(default)]
        border_color: Color,
        #[serde(default)]
        hover_border_color: Color,
        #[serde(default)]
        check_color: Color,
        #[serde(default)]
        style: Style,
    },
    Slider {
        #[serde(default)]
        min_value: f32,
        #[serde(default)]
        max_value: f32,
        #[serde(default)]
        value: f32,
        #[serde(default)]
        track_color: Color,
        #[serde(default)]
        fill_color: Color,
        #[serde(default)]
        thumb_color: Color,
        #[serde(default)]
        style: Style,
    },
    TabView {
        #[serde(default, rename = "tab_labels")]
        tabs: Vec<String>,
        #[serde(default)]
        selected_index: usize,
        #[serde(default)]
        active_text_color: Color,
        #[serde(default)]
        inactive_text_color: Color,
        #[serde(default)]
        underline_color: Color,
        #[serde(default)]
        style: Style,
        #[serde(default, rename = "tab_contents")]
        children: Vec<NodeDefinition>,
    },
    ToastContainer {
        #[serde(default)]
        style: Style,
    },
}

fn build_widget(def: NodeDefinition, theme: &Theme) -> Box<dyn Widget> {
    match def {
        NodeDefinition::Row {
            style,
            justify,
            align,
            children,
        } => {
            let mut row = Row::new(style).with_justify(justify).with_align(align);

            for child_def in children {
                row.add_child(build_widget(child_def, theme));
            }

            Box::new(row)
        }
        NodeDefinition::Column {
            style,
            span,
            justify,
            align,
            children,
        } => {
            let mut col = Column::new(style)
                .with_span(if span == 0 { 12 } else { span })
                .with_justify(justify)
                .with_align(align);

            for child_def in children {
                col.add_child(build_widget(child_def, theme));
            }

            Box::new(col)
        }
        NodeDefinition::Box { style } => Box::new(BoxWidget::new(style)),
        NodeDefinition::Diamond { style } => Box::new(DiamondWidget::new(style)),
        NodeDefinition::Text { content, style } => Box::new(Text::new(content, style)),
        NodeDefinition::Label { content, style } => Box::new(Label::new(content, style)),
        NodeDefinition::TextureRect {
            texture_id,
            style,
            flip_v,
        } => Box::new(TextureRect::new(texture_id, style).with_flip_v(flip_v)),
        NodeDefinition::ProgressBar {
            current_value,
            max_value,
            fill_color,
            outline_color,
            style,
        } => Box::new(ProgressBar::new(
            style,
            current_value,
            max_value,
            fill_color,
            outline_color,
        )),
        NodeDefinition::ScrollView {
            content_height,
            justify,
            style,
            scrollbar_style,
            children,
        } => {
            let mut scroll = ScrollView::new(style, content_height)
                .with_justify(justify)
                .with_scrollbar_style(scrollbar_style);

            for child_def in children {
                scroll.add_child(build_widget(child_def, theme));
            }

            Box::new(scroll)
        }
        NodeDefinition::AbilitySlot {
            key_label,
            slot_background,
            ready_border_color,
            normal_border_color,
            style,
        } => {
            let mut slot = AbilitySlot::new(style).with_key_label(key_label);
            if !matches!(slot_background, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                slot = slot.with_slot_background(slot_background);
            }
            if !matches!(ready_border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                slot = slot.with_ready_border_color(ready_border_color);
            }
            if !matches!(normal_border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                slot = slot.with_normal_border_color(normal_border_color);
            }
            Box::new(slot)
        }
        NodeDefinition::TooltipManager {
            background_color,
            border_color,
            title_color,
            description_color,
            style,
        } => {
            let mut tooltip = TooltipManager::new(style);
            if !matches!(background_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                tooltip.background_color = background_color;
            }
            if !matches!(border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                tooltip.border_color = border_color;
            }
            if !matches!(title_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                tooltip.title_color = title_color;
            }
            if !matches!(description_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                tooltip.description_color = description_color;
            }
            Box::new(tooltip)
        }
        NodeDefinition::MenuButton {
            text,
            normal_background,
            hover_background,
            accent_color,
            border_color,
            text_color,
            style,
        } => {
            let mut btn = MenuButton::new(text, style);

            let mut normal_bg = if !matches!(normal_background, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                normal_background
            } else {
                btn.normal_background.clone()
            };

            let mut hover_bg = if !matches!(hover_background, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                hover_background
            } else {
                btn.hover_background.clone()
            };

            let mut accent = if !matches!(accent_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                accent_color
            } else {
                btn.accent_color.clone()
            };

            let mut border = if !matches!(border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                border_color
            } else {
                btn.border_color.clone()
            };

            let mut txt_color = if !matches!(text_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                text_color
            } else {
                btn.text_color.clone()
            };

            // resolve any variables
            resolve_color(&mut normal_bg, theme);
            resolve_color(&mut hover_bg, theme);
            resolve_color(&mut accent, theme);
            resolve_color(&mut border, theme);
            resolve_color(&mut txt_color, theme);

            btn = btn
                .with_normal_background(normal_bg)
                .with_hover_background(hover_bg)
                .with_accent_color(accent)
                .with_border_color(border)
                .with_text_color(txt_color);

            Box::new(btn)
        }
        NodeDefinition::CloseButton {
            normal_color,
            hover_color,
            style,
        } => {
            let mut close_btn = CloseButton::new(style);
            if !matches!(normal_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                close_btn = close_btn.with_normal_color(normal_color);
            }
            if !matches!(hover_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                close_btn = close_btn.with_hover_color(hover_color);
            }
            Box::new(close_btn)
        }
        NodeDefinition::ComboBox {
            options,
            selected_index,
            dropdown_background,
            item_hover_color,
            text_color,
            border_color,
            style,
        } => {
            let mut combo = ComboBox::new(options, style).with_selected_index(selected_index);

            let mut dd_bg = if !matches!(dropdown_background, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                dropdown_background
            } else {
                combo.dropdown_background.clone()
            };

            let mut hover_color = if !matches!(item_hover_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                item_hover_color
            } else {
                combo.item_hover_color.clone()
            };

            let mut txt_color = if !matches!(text_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                text_color
            } else {
                combo.text_color.clone()
            };

            let mut bdr_color = if !matches!(border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                border_color
            } else {
                combo.border_color.clone()
            };

            // resolve any variables
            resolve_color(&mut dd_bg, theme);
            resolve_color(&mut hover_color, theme);
            resolve_color(&mut txt_color, theme);
            resolve_color(&mut bdr_color, theme);

            combo = combo
                .with_dropdown_background(dd_bg)
                .with_item_hover_color(hover_color)
                .with_text_color(txt_color)
                .with_border_color(bdr_color);

            Box::new(combo)
        }
        NodeDefinition::Checkbox {
            checked,
            background_color,
            border_color,
            hover_border_color,
            check_color,
            style,
        } => {
            let mut checkbox = Checkbox::new(style).with_checked(checked);

            let mut bg = if !matches!(background_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                background_color
            } else {
                checkbox.background_color.clone()
            };

            let mut border = if !matches!(border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                border_color
            } else {
                checkbox.border_color.clone()
            };

            let mut hover_border = if !matches!(hover_border_color, Color::Rgba(0.0, 0.0, 0.0, 0.0))
            {
                hover_border_color
            } else {
                checkbox.hover_border_color.clone()
            };

            let mut check = if !matches!(check_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                check_color
            } else {
                checkbox.check_color.clone()
            };

            // resolve any variables
            resolve_color(&mut bg, theme);
            resolve_color(&mut border, theme);
            resolve_color(&mut hover_border, theme);
            resolve_color(&mut check, theme);

            checkbox = checkbox
                .with_background_color(bg)
                .with_border_color(border)
                .with_hover_border_color(hover_border)
                .with_check_color(check);

            Box::new(checkbox)
        }
        NodeDefinition::Slider {
            min_value,
            max_value,
            value,
            track_color,
            fill_color,
            thumb_color,
            style,
        } => {
            let mut slider = Slider::new(style)
                .with_range(min_value, max_value)
                .with_value(value);

            let mut track = if !matches!(track_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                track_color
            } else {
                slider.track_color.clone()
            };

            let mut fill = if !matches!(fill_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                fill_color
            } else {
                slider.fill_color.clone()
            };

            let mut thumb = if !matches!(thumb_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                thumb_color
            } else {
                slider.thumb_color.clone()
            };

            // resolve any variables
            resolve_color(&mut track, theme);
            resolve_color(&mut fill, theme);
            resolve_color(&mut thumb, theme);

            let mut track_border = slider.track_border_color.clone();
            resolve_color(&mut track_border, theme);
            slider.track_border_color = track_border;

            let mut thumb_pressed = slider.thumb_pressed_color.clone();
            resolve_color(&mut thumb_pressed, theme);
            slider.thumb_pressed_color = thumb_pressed;

            slider = slider
                .with_track_color(track)
                .with_fill_color(fill)
                .with_thumb_color(thumb);

            Box::new(slider)
        }
        NodeDefinition::TabView {
            tabs,
            selected_index,
            active_text_color,
            inactive_text_color,
            underline_color,
            style,
            children,
        } => {
            let mut tab_view = TabView::new(tabs, style).with_selected_index(selected_index);

            let mut active_text = if !matches!(active_text_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                active_text_color
            } else {
                tab_view.active_text_color.clone()
            };

            let mut inactive_text =
                if !matches!(inactive_text_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                    inactive_text_color
                } else {
                    tab_view.inactive_text_color.clone()
                };

            let mut underline = if !matches!(underline_color, Color::Rgba(0.0, 0.0, 0.0, 0.0)) {
                underline_color
            } else {
                tab_view.underline_color.clone()
            };

            // resolve any variables
            resolve_color(&mut active_text, theme);
            resolve_color(&mut inactive_text, theme);
            resolve_color(&mut underline, theme);

            tab_view = tab_view
                .with_active_text_color(active_text)
                .with_inactive_text_color(inactive_text)
                .with_underline_color(underline);

            for child_def in children {
                tab_view.add_child(build_widget(child_def, theme));
            }
            Box::new(tab_view)
        }
        NodeDefinition::ToastContainer { style } => {
            let container = ToastContainer::new(style);
            Box::new(container)
        }
    }
}

/// Resolves theme variables within a [`Style`] definition.
///
/// This function iterates through all length and color properties of the style,
/// replacing any variable references with their actual values from the provided [`Theme`].
fn resolve_style(style: &mut Style, theme: &Theme) {
    resolve_length(&mut style.width, theme);
    resolve_length(&mut style.height, theme);
    resolve_length(&mut style.min_width, theme);
    resolve_length(&mut style.min_height, theme);
    resolve_length(&mut style.max_width, theme);
    resolve_length(&mut style.max_height, theme);

    resolve_length(&mut style.margin, theme);
    if let Some(l) = &mut style.margin_top {
        resolve_length(l, theme);
    }
    if let Some(l) = &mut style.margin_right {
        resolve_length(l, theme);
    }
    if let Some(l) = &mut style.margin_bottom {
        resolve_length(l, theme);
    }
    if let Some(l) = &mut style.margin_left {
        resolve_length(l, theme);
    }

    resolve_length(&mut style.padding, theme);
    if let Some(l) = &mut style.padding_top {
        resolve_length(l, theme);
    }
    if let Some(l) = &mut style.padding_right {
        resolve_length(l, theme);
    }
    if let Some(l) = &mut style.padding_bottom {
        resolve_length(l, theme);
    }
    if let Some(l) = &mut style.padding_left {
        resolve_length(l, theme);
    }

    resolve_color(&mut style.background, theme);
    resolve_color(&mut style.border_color, theme);
    if let Some(c) = &mut style.color {
        resolve_color(c, theme);
    }
}

/// Resolves a [`Length`] variable within the provided [`Theme`].
///
/// Currently, length variables are not fully supported in the theme definition.
/// This function will log a warning if a variable reference is encountered.
fn resolve_length(length: &mut Length, _theme: &Theme) {
    if let Length::Variable(name) = length {
        // TODO: Support Length variables in theme if needed.
        // For now, theme only has 'color' and 'string' properly typed in our parser,
        // but we can assume some convention or just support what's needed.
        // Given the theme file, it's mostly colors.
        // If we need lengths, we'd need to update theme parser.
        // For now, let's leave it no-op or maybe log?
        eprintln!("Warning: Length variable '{}' not supported yet", name);
    }
}

/// Resolves a [`Color`] variable within the provided [`Theme`].
///
/// This function looks up the color in the theme and replaces the variable reference
/// with the actual color value. If the color is not found in the theme, a warning
/// is logged.
fn resolve_color(color: &mut Color, theme: &Theme) {
    if let Color::Variable(name) = color {
        if let Some(resolved) = theme.get_color(name) {
            *color = resolved;
        } else {
            eprintln!("Warning: Theme color '{}' not found", name);
        }
    }
}

/// Recursively resolves variables in a [`NodeDefinition`].
///
/// This function traverses the widget tree and resolves any variables
/// in the style properties of each widget. It also recursively resolves
/// variables in child widgets.
fn resolve_variables(def: &mut NodeDefinition, theme: &Theme) {
    match def {
        NodeDefinition::Row {
            style, children, ..
        } => {
            resolve_style(style, theme);
            for child in children {
                resolve_variables(child, theme);
            }
        }
        NodeDefinition::Column {
            style, children, ..
        } => {
            resolve_style(style, theme);
            for child in children {
                resolve_variables(child, theme);
            }
        }
        NodeDefinition::Box { style } => {
            resolve_style(style, theme);
        }
        NodeDefinition::Diamond { style } => {
            resolve_style(style, theme);
        }
        NodeDefinition::Text { style, .. } => {
            resolve_style(style, theme);
        }
        NodeDefinition::Label { style, .. } => {
            resolve_style(style, theme);
        }
        NodeDefinition::TextureRect { style, .. } => {
            resolve_style(style, theme);
        }
        NodeDefinition::ProgressBar {
            fill_color,
            outline_color,
            style,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(fill_color, theme);
            resolve_color(outline_color, theme);
        }
        NodeDefinition::ScrollView {
            style,
            scrollbar_style,
            children,
            ..
        } => {
            resolve_style(style, theme);
            // resolve scrollbar colors if using theme variables
            if let Some(c) = &mut scrollbar_style.track_color {
                resolve_color(c, theme);
            }
            if let Some(c) = &mut scrollbar_style.thumb_color {
                resolve_color(c, theme);
            }
            if let Some(c) = &mut scrollbar_style.thumb_hover_color {
                resolve_color(c, theme);
            }
            if let Some(c) = &mut scrollbar_style.thumb_active_color {
                resolve_color(c, theme);
            }
            for child in children {
                resolve_variables(child, theme);
            }
        }
        NodeDefinition::AbilitySlot {
            slot_background,
            ready_border_color,
            normal_border_color,
            style,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(slot_background, theme);
            resolve_color(ready_border_color, theme);
            resolve_color(normal_border_color, theme);
        }
        NodeDefinition::TooltipManager {
            background_color,
            border_color,
            title_color,
            description_color,
            style,
        } => {
            resolve_style(style, theme);
            resolve_color(background_color, theme);
            resolve_color(border_color, theme);
            resolve_color(title_color, theme);
            resolve_color(description_color, theme);
        }
        NodeDefinition::MenuButton {
            normal_background,
            hover_background,
            accent_color,
            border_color,
            text_color,
            style,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(normal_background, theme);
            resolve_color(hover_background, theme);
            resolve_color(accent_color, theme);
            resolve_color(border_color, theme);
            resolve_color(text_color, theme);
        }
        NodeDefinition::CloseButton {
            normal_color,
            hover_color,
            style,
        } => {
            resolve_style(style, theme);
            resolve_color(normal_color, theme);
            resolve_color(hover_color, theme);
        }
        NodeDefinition::ComboBox {
            dropdown_background,
            item_hover_color,
            text_color,
            border_color,
            style,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(dropdown_background, theme);
            resolve_color(item_hover_color, theme);
            resolve_color(text_color, theme);
            resolve_color(border_color, theme);
        }
        NodeDefinition::Checkbox {
            background_color,
            border_color,
            hover_border_color,
            check_color,
            style,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(background_color, theme);
            resolve_color(border_color, theme);
            resolve_color(hover_border_color, theme);
            resolve_color(check_color, theme);
        }
        NodeDefinition::Slider {
            track_color,
            fill_color,
            thumb_color,
            style,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(track_color, theme);
            resolve_color(fill_color, theme);
            resolve_color(thumb_color, theme);
        }
        NodeDefinition::TabView {
            active_text_color,
            inactive_text_color,
            underline_color,
            style,
            children,
            ..
        } => {
            resolve_style(style, theme);
            resolve_color(active_text_color, theme);
            resolve_color(inactive_text_color, theme);
            resolve_color(underline_color, theme);
            for child in children {
                resolve_variables(child, theme);
            }
        }
        NodeDefinition::ToastContainer { style } => {
            resolve_style(style, theme);
        }
    }
}

/// Loads a view from a RON file.
///
/// This function reads the RON file, parses it into a [`NodeDefinition`],
/// resolves any variables in the style properties, and returns a [`UiTree`].
pub fn load_view<P: AsRef<Path>>(path: P) -> Result<UiTree, String> {
    let path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| "RON path contains invalid UTF-8".to_string())?;
    let content =
        assets::read_text(path).map_err(|e| format!("Failed to read RON file: {}", e))?;

    let mut root_def: NodeDefinition =
        ron::from_str(&content).map_err(|e| format!("Failed to parse RON: {}", e))?;

    // TODO: Pass this in or cache it
    let theme_path = "resources/ui/theme.ron";
    let theme = load_theme(theme_path).unwrap_or_else(|e| {
        eprintln!("Failed to load theme: {}", e);
        Theme::new()
    });

    resolve_variables(&mut root_def, &theme);

    let root_widget = build_widget(root_def, &theme);

    let mut tree = UiTree::new();
    tree.set_root(root_widget);

    Ok(tree)
}

/// Loads a view from a RON string.
///
/// This function parses the RON string into a [`NodeDefinition`],
/// resolves any variables in the style properties, and returns a [`UiTree`].
pub fn load_view_from_str(content: &str) -> Result<UiTree, String> {
    let mut root_def: NodeDefinition =
        ron::from_str(content).map_err(|e| format!("Failed to parse RON: {}", e))?;

    // for raw string loading (tests), we default to empty theme or could allow passing one.
    // assuming empty theme for unit tests to avoid FS dependency.
    let theme = Theme::new();
    resolve_variables(&mut root_def, &theme);

    let root_widget = build_widget(root_def, &theme);

    let mut tree = UiTree::new();
    tree.set_root(root_widget);

    Ok(tree)
}

/// Loads a view from a RON file, or returns a fallback error view if loading fails.
pub fn load_view_or_fallback<P: AsRef<Path>>(path: P) -> UiTree {
    match load_view(path) {
        Ok(tree) => tree,
        Err(e) => {
            eprintln!("[UiParser] Error loading view: {}", e);
            create_error_view(&e)
        }
    }
}

fn create_error_view(error: &str) -> UiTree {
    let mut tree = UiTree::new();

    let mut msg = error.to_string();
    if msg.contains("Expected option") {
        msg.push_str("\nHint: Did you forget to wrap an optional field in Some(...)?");
    }

    // create a root column to hold the error message
    let style = Style {
        width: crate::ui::game_new::styles::Length::Percent(100.0),
        height: crate::ui::game_new::styles::Length::Percent(100.0),
        background: crate::ui::game_new::styles::Color::Variable("error-background".to_string()),
        padding: crate::ui::game_new::styles::Length::Px(20.0),
        ..Default::default()
    };

    let mut col = Column::new(style)
        .with_justify(crate::ui::game_new::styles::Alignment::Center)
        .with_align(crate::ui::game_new::styles::Alignment::Center);

    let text_style = Style {
        color: Some(crate::ui::game_new::styles::Color::Variable(
            "error-text".to_string(),
        )),
        font_size: Some(20.0),
        text_align: Some(crate::ui::game_new::styles::Alignment::Center),
        ..Default::default()
    };

    // add title
    col.add_child(Box::new(Label::new(
        "UI Load Error".to_string(),
        text_style.clone(),
    )));

    // add error message
    col.add_child(Box::new(Label::new(msg, text_style)));

    tree.set_root(Box::new(col));
    tree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_row() {
        let ron = r#"
            Row(
                style: (
                    width: Px(100.0),
                    height: Px(50.0),
                ),
                justify: Center,
                align: Start,
                children: []
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse simple Row: {}", e);
        }
    }

    #[test]
    fn test_parse_simple_column() {
        let ron = r#"
            Column(
                style: (
                    background: Rgba(1.0, 0.0, 0.0, 1.0),
                ),
                span: 6,
                children: []
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse simple Column: {}", e);
        }
    }

    #[test]
    fn test_parse_simple_box() {
        let ron = r#"
            Box(
                style: (
                    width: Px(50.0),
                    height: Px(50.0),
                    background: Rgba(0.0, 1.0, 0.0, 1.0),
                )
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse simple Box: {}", e);
        }
    }

    #[test]
    fn test_parse_simple_label() {
        let ron = r#"
            Label(
                content: "Label Text",
                style: (
                    font_size: Some(12.0),
                )
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse simple Label: {}", e);
        }
    }

    #[test]
    fn test_parse_nested_structure() {
        let ron = r#"
            Row(
                style: (
                    width: Px(400.0),
                    height: Px(100.0),
                ),
                justify: SpaceBetween,
                children: [
                    Column(
                        span: 4,
                        style: (
                            background: Rgba(0.8, 0.2, 0.2, 1.0),
                        ),
                    ),
                    Column(
                        span: 4,
                        style: (
                            background: Rgba(0.2, 0.8, 0.2, 1.0),
                        ),
                    ),
                    Column(
                        span: 4,
                        style: (
                            background: Rgba(0.2, 0.2, 0.8, 1.0),
                        ),
                    ),
                ]
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse nested structure: {}", e);
        }
    }

    #[test]
    fn test_defaults_applied() {
        // Test that fields with #[serde(default)] work when omitted
        let ron = r#"
            Row(
                children: []
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse Row with defaults: {}", e);
        }
    }

    #[test]
    fn test_column_span_defaults_to_12_when_zero() {
        // The build_widget function should convert span=0 to span=12
        let ron = r#"
            Column(
                span: 0,
                children: []
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse Column with span=0: {}", e);
        }
    }

    #[test]
    fn test_resolve_theme_variable() {
        // Create a theme with a specific color
        let mut theme = Theme::new();
        theme
            .colors
            .insert("test-color".to_string(), Color::Rgba(0.5, 0.5, 0.5, 1.0));

        // Define a node using a variable
        let mut root_def = NodeDefinition::Box {
            style: Style {
                background: Color::Variable("test-color".to_string()),
                ..Default::default()
            },
        };

        // Resolve variables
        resolve_variables(&mut root_def, &theme);

        // Check if resolved
        if let NodeDefinition::Box { style } = root_def {
            if let Color::Rgba(r, g, b, a) = style.background {
                assert_eq!(r, 0.5);
                assert_eq!(g, 0.5);
                assert_eq!(b, 0.5);
                assert_eq!(a, 1.0);
            } else {
                panic!(
                    "Color should have been resolved to Rgba, found {:?}",
                    style.background
                );
            }
        } else {
            panic!("Root should be a Box");
        }
    }
    #[test]
    fn test_parse_game_hud_file() {
        let result = load_view("resources/ui/game_hud.ron");
        if let Err(e) = result {
            panic!("Failed to parse game_hud.ron: {}", e);
        }
    }
    #[test]
    fn test_load_view_fallback() {
        // Invalid RON that should trigger a fallback
        let ron = "Invalid(RON)";

        let result = load_view_from_str(ron);
        match result {
            Ok(_) => panic!("Should have failed to parse"),
            Err(_) => {
                // Manually call create_error_view logic since load_view catches it
                let _fallback = create_error_view("Test Error");
                // Verify fallback structure (root is Column, has Label children)
                // This is a basic sanity check
                // We can't easily inspect the Box<dyn Widget>, but ensuring it builds is good.
            }
        }
    }

    #[test]
    fn test_parse_menu_button() {
        let ron = r#"
            MenuButton(
                text: "Click Me",
                style: (
                    id: Some("my_button"),
                    width: Px(200.0),
                    height: Px(40.0),
                ),
                normal_background: Rgba(0.1, 0.1, 0.1, 1.0),
                hover_background: Rgba(0.2, 0.2, 0.2, 1.0),
                accent_color: Rgba(0.8, 0.5, 0.0, 1.0),
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse MenuButton: {}", e);
        }
    }

    #[test]
    fn test_parse_combo_box() {
        let ron = r#"
            ComboBox(
                options: ["Option 1", "Option 2", "Option 3"],
                selected_index: 1,
                style: (
                    id: Some("my_combo"),
                    width: Px(150.0),
                    height: Px(30.0),
                ),
                dropdown_background: Rgba(0.1, 0.1, 0.1, 1.0),
                item_hover_color: Rgba(0.2, 0.2, 0.2, 1.0),
            )
        "#;

        let result = load_view_from_str(ron);
        if let Err(e) = result {
            panic!("Failed to parse ComboBox: {}", e);
        }
    }
}
