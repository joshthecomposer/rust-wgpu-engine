use crate::ui::game_new::context::UiContext;
use crate::ui::game_new::font_system::FontSystem;
use crate::ui::game_new::render::UiRenderer;
use crate::ui::game_new::styles::{GridSpan, Rect};

pub trait Widget {
    /// layout phase: given a bounding rect, compute positions for self and children
    fn layout(&mut self, font_system: &mut FontSystem, available: Rect);

    /// update phase: process input events, return true if event was consumed
    fn update(&mut self, ctx: &mut UiContext) -> bool;

    /// render phase: draw self and children to the renderer
    fn render(&self, renderer: &mut UiRenderer);

    /// get the computed rect after layout
    fn rect(&self) -> Rect;

    fn grid_span(&self) -> Option<GridSpan> {
        None
    }

    fn id(&self) -> Option<&str> {
        None
    }

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// iterate children (if any)
    fn for_each_child_mut(&mut self, _f: &mut dyn FnMut(&mut dyn Widget)) {}

    /// recursively find a widget by ID
    fn find_widget_mut(&mut self, id: &str) -> Option<&mut dyn Widget> {
        // TODO: implement
        if self.id() == Some(id) {
            None
        } else {
            None
        }
    }
}
