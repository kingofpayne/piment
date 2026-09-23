use crate::{
    graphics::Graphics,
    impl_widget_core,
    rect::Rect,
    theme::THEME,
    widgets::{SharedWidget, Widget, WidgetCore},
};
use glam::{Vec2, Vec4, vec2};

/// A panel with one child widget.
pub struct Panel {
    /// Widget common properties.
    core: WidgetCore,
    /// Child widget.
    child: Option<SharedWidget>,
    /// Padding for child widget placement.
    padding: f32,
    /// Rectangle corner radius.
    pub corner_radius: Vec4,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            core: WidgetCore::new("Panel"),
            child: None,
            padding: 4.0,
            corner_radius: Vec4::splat(0.0),
        }
    }

    pub fn set_child(&mut self, child: SharedWidget) {
        self.child = Some(child.clone());
        self.core.children = vec![child];
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Panel {
    impl_widget_core!();

    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        self.core.layout(rect);
        if let Some(child) = &mut self.child {
            child
                .borrow_mut()
                .layout(graphics, rect.expand(-self.padding));
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        let mut size = vec2(self.padding * 2.0, self.padding * 2.0);
        if let Some(child) = &self.child {
            size += child.borrow_mut().minimum_size(graphics);
        }
        size
    }

    fn render(&mut self, graphics: &mut Graphics) {
        graphics.painter.round_rectangle(
            self.core.rect,
            self.corner_radius,
            Vec4::splat(-1.0),
            [THEME.panel_color; 4],
        );
    }
}
