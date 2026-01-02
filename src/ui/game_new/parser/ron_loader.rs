use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::ui::game_new::styles::{Alignment, Style};
use crate::ui::game_new::tree::UiTree;
use crate::ui::game_new::widgets::{BoxWidget, Column, Row, Widget};

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
    }
}

pub fn load_view<P: AsRef<Path>>(path: P) -> Result<UiTree, String> {
    let content =
        fs::read_to_string(path.as_ref()).map_err(|e| format!("Failed to read RON file: {}", e))?;

    let root_def: NodeDefinition =
        ron::from_str(&content).map_err(|e| format!("Failed to parse RON: {}", e))?;

    let root_widget = build_widget(root_def);

    let mut tree = UiTree::new();
    tree.set_root(root_widget);

    Ok(tree)
}

pub fn load_view_from_str(content: &str) -> Result<UiTree, String> {
    let root_def: NodeDefinition =
        ron::from_str(content).map_err(|e| format!("Failed to parse RON: {}", e))?;

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
}
