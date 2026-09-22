use crate::{
    color::Color,
    font::TextHorizontalAlign,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::{FontStyle, Stroke},
    rect::Rect,
    theme::THEME,
    widgets::{Widget, WidgetCore},
};
use glam::{Vec4, vec2};

pub struct CheckBox {
    /// Widget common properties.
    core: WidgetCore,
    /// Text displayed aside the checkbox
    label: String,
    /// Checkbox state.
    pub checked: bool,
    /// Set when the user checked or unchecked the checkbox.
    pub on_change: bool,
}

impl CheckBox {
    /// Creates a new checkbox with an empty label.
    pub fn new() -> Self {
        Self {
            core: WidgetCore::new("CheckBox"),
            label: "".into(),
            checked: false,
            on_change: false,
        }
    }

    /// Build a checkbox with the given label.
    pub fn with_label(self, label: &str) -> Self {
        Self {
            label: label.into(),
            ..self
        }
    }

    /// Build a checkbox checked by default.
    pub fn with_checked(self) -> Self {
        Self {
            checked: true,
            ..self
        }
    }
}

impl Default for CheckBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for CheckBox {
    impl_widget_core!();

    fn minimum_size(&mut self, _graphics: &mut Graphics) -> glam::Vec2 {
        vec2(50.0, 25.0)
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        self.on_change = input.mouse_left_press && input.hit;
        self.checked ^= self.on_change;
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let box_size = 14.0;
        let box_rect = Rect::new(
            self.core.rect.x1,
            self.core.rect.v_center() - box_size / 2.0,
            self.core.rect.x1 + box_size,
            self.core.rect.v_center() + box_size / 2.0,
        );
        graphics.painter.round_rectangle(
            box_rect + vec2(0.0, 1.0),
            Vec4::splat(3.0),
            Vec4::splat(-1.0),
            [Color::BLACK.with_alpha(0.25); 4],
        );
        let box_color = if self.checked {
            THEME.active_color
        } else {
            Color::new_gray(0.329)
        };
        graphics.painter.round_rectangle(
            box_rect,
            Vec4::splat(3.0),
            Vec4::splat(-1.0),
            [box_color; 4],
        );
        if self.checked {
            let c = box_rect.center();
            graphics.painter.polyline(
                &[c + vec2(-3.0, 0.0), c + vec2(0.0, 4.0), c + vec2(3.0, -4.0)],
                Stroke::new(Color::WHITE, 1.5),
            );
        }
        if !self.label.is_empty() {
            let style = FontStyle::new();
            let layout = graphics.font.layout(
                &self.label,
                self.core.rect + Rect::new(box_size + 4.0, 0.0, 0.0, 0.0),
                TextHorizontalAlign::Left,
                style.size,
            );
            graphics.painter.text_layout(&graphics.font, &layout, style);
        }
    }
}
