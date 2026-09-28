use crate::{
    color::Color,
    font::TextHorizontalAlign,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::FontStyle,
    rect::Rect,
    theme::THEME,
    widgets::{Widget, WidgetCore},
};
use glam::{Vec4, vec2};

pub struct Button {
    /// Widget common properties.
    core: WidgetCore,
    /// Text displayed on the button.
    pub text: String,
    /// True while the button is being pressed down.
    pub pressed: bool,
    /// Set to true on the frame the button is released after being pressed.
    pub on_click: bool,
}

impl Button {
    const PADDING: f32 = 8.0;

    pub fn new(text: &str) -> Self {
        Self {
            core: WidgetCore::new(),
            text: text.into(),
            pressed: false,
            on_click: false,
        }
    }
}

impl Widget for Button {
    impl_widget_core!();

    fn minimum_size(&mut self, graphics: &mut Graphics) -> glam::Vec2 {
        vec2(
            graphics.font.text_width(&self.text, THEME.font_size) + Self::PADDING * 2.0,
            21.0,
        )
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        self.core.update(input);
        self.on_click = false;

        if self.core.hover && input.mouse_left_press && input.hit {
            self.pressed = true;
        }

        if input.mouse_left_release {
            if self.pressed && self.core.hover && input.hit {
                self.on_click = true;
            }
            self.pressed = false;
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let rect = self.core.rect;
        // The bottom pixel row of the widget is left to the shadow, so the button body is one
        // pixel shorter. Everything else is placed relatively to the body, otherwise it is off by
        // half a pixel.
        let body = rect + Rect::new(0.0, 0.0, 0.0, -1.0);

        // Shadow
        graphics.painter.round_rectangle(
            rect,
            Vec4::splat(4.0),
            Vec4::splat(-1.0),
            [THEME.shadow_color; 4],
        );

        // Background
        let bg_color = if self.pressed {
            THEME.active_color
        } else if self.core.hover {
            THEME.button_hover_color
        } else {
            THEME.button_idle_color
        };
        graphics
            .painter
            .round_rectangle(body, Vec4::splat(4.0), Vec4::splat(-1.0), [bg_color; 4]);

        // Border
        graphics.painter.round_rectangle(
            body,
            Vec4::splat(4.0),
            Vec4::splat(3.0),
            [THEME.border_color; 4],
        );

        // Text
        let style = FontStyle::new()
            .color(Color::WHITE)
            .shadow_color(Color::new_rgba(0.0, 0.0, 0.0, 0.5))
            .shadow_offset(vec2(0.0, 1.0));
        let layout =
            graphics
                .font
                .layout(&self.text, body, TextHorizontalAlign::Center, style.size);
        graphics.painter.text_layout(&graphics.font, &layout, style);
    }
}
