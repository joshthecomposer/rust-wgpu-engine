use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{GridSpan, Rect};

pub trait Widget {
    /// layout phase: given a bounding rect, compute positions for self and children
    fn layout(&mut self, available: Rect);

    /// update phase: process input events, return true if event was consumed
    fn update(&mut self, ctx: &mut UiContext) -> bool;

    /// render phase: draw self and children to the renderer
    fn render(&self, renderer: &mut UiRenderer);

    /// get the computed rect after layout
    fn rect(&self) -> Rect;

    /// get the grid span if this widget is a Column, None otherwise
    fn grid_span(&self) -> Option<GridSpan> {
        None
    }
}

