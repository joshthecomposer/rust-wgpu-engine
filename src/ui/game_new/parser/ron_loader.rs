use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::ui::game_new::parser::theme::{load_theme, Theme};
use crate::ui::game_new::styles::{Alignment, Color, Length, Style};
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::{BoxWidget, Column, Row, Text, Widget};

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
    Text {
        content: String,
        #[serde(default)]
        style: Style,
    },
}

fn build_widget(def: NodeDefinition) -> Box<dyn Widget> {
    match def {
        NodeDefinition::Row {
            style,
            justify,
            align,
            children,
        } => {
            let mut row = Row::new(style).with_justify(justify).with_align(align);

            for child_def in children {
                row.add_child(build_widget(child_def));
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
                col.add_child(build_widget(child_def));
            }

            Box::new(col)
        }
        NodeDefinition::Box { style } => Box::new(BoxWidget::new(style)),
        NodeDefinition::Text { content, style } => Box::new(Text::new(content, style)),
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
}

/// Resolves a [`Length`] variable within the provided [`Theme`].
///
/// Currently, length variables are not fully supported in the theme definition.
/// This function will log a warning if a variable reference is encountered.
fn resolve_length(length: &mut Length, theme: &Theme) {
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
        NodeDefinition::Text { style, .. } => {
            resolve_style(style, theme);
        }
    }
}

/// Loads a view from a RON file.
///
/// This function reads the RON file, parses it into a [`NodeDefinition`],
/// resolves any variables in the style properties, and returns a [`UiTree`].
pub fn load_view<P: AsRef<Path>>(path: P) -> Result<UiTree, String> {
    let content =
        fs::read_to_string(path.as_ref()).map_err(|e| format!("Failed to read RON file: {}", e))?;

    let mut root_def: NodeDefinition =
        ron::from_str(&content).map_err(|e| format!("Failed to parse RON: {}", e))?;

    // TODO: Pass this in or cache it
    let theme_path = "resources/ui/theme.ron";
    let theme = load_theme(theme_path).unwrap_or_else(|e| {
        eprintln!("Failed to load theme: {}", e);
        Theme::new()
    });

    resolve_variables(&mut root_def, &theme);

    let root_widget = build_widget(root_def);

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

    let root_widget = build_widget(root_def);

    let mut tree = UiTree::new();
    tree.set_root(root_widget);

    Ok(tree)
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
    fn test_parse_test_view_file() {
        // This test reads the actual test_view.ron file to ensure it parses correctly
        let result = load_view("src/ui/game_new/views/test_view.ron");
        if let Err(e) = result {
            panic!("Failed to parse test_view.ron: {}", e);
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
        // This test reads the newly created game_hud.ron file
        let result = load_view("src/ui/game_new/views/game_hud.ron");
        if let Err(e) = result {
            panic!("Failed to parse game_hud.ron: {}", e);
        }
    }
}
