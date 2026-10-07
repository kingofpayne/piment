use glam::{Vec2, vec2};

use crate::{
    graphics::Graphics,
    impl_widget_core,
    painter::FontStyle,
    widgets::{Widget, WidgetCore},
};

/// A simple text label.
pub struct Label {
    /// Widget common properties.
    core: WidgetCore,
    /// Text of the label.
    text: String,
    /// Text style.
    pub style: FontStyle,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            core: WidgetCore::new(),
            text: text.into(),
            style: FontStyle::new(),
        }
    }

    /// Sets the text to be displayed by the label.
    pub fn set_text(&mut self, text: &str) {
        // Check for a difference to avoid triggering unnecessary layout recalculation.
        if text != self.text {
            self.text = text.into();
            self.core.layout_request = true;
        }
    }
}

impl Widget for Label {
    impl_widget_core!();

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        vec2(graphics.font.text_width(&self.text, self.style.size), 20.0)
    }

    fn render(&mut self, graphics: &mut Graphics) {
        graphics.painter.text(
            &mut graphics.font,
            &self.text,
            vec2(
                self.core.rect.x1,
                self.core.rect.y1.midpoint(self.core.rect.y2),
            )
            .into(),
            self.style,
        );
    }
}
