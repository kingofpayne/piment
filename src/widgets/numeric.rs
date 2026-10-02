use crate::{
    color::Color,
    font::TextHorizontalAlign,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::{FontStyle, Stroke},
    rect::Rect,
    theme::THEME,
    widgets::{LineBox, Share, Shared, Widget, WidgetCore},
};
use glam::{Vec2, Vec4, vec2};
use winit::{keyboard::KeyCode, window::CursorIcon};

/// Area of a [Numeric] widget.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Zone {
    /// Left arrow, decrementing the value.
    Decrement,
    /// Middle area displaying the value.
    Value,
    /// Right arrow, incrementing the value.
    Increment,
}

/// A button displaying a numeric value.
///
/// Dragging the mouse horizontally while the button is pressed changes the value. Clicking without
/// moving the mouse opens a [LineBox] to type a new value, or decrements or increments the value
/// by one when clicking on the left or right arrow.
pub struct Numeric {
    /// Widget common properties.
    core: WidgetCore,
    /// Current value.
    value: f32,
    /// Lowest allowed value, if any.
    min: Option<f32>,
    /// Highest allowed value, if any.
    max: Option<f32>,
    /// Value change of a single increment, when clicking on an arrow or dragging by
    /// [Self::speed] pixels.
    pub step: f32,
    /// Horizontal mouse displacement in pixels for a single [Self::step] increment when dragging.
    /// Dragging has no effect if this is not greater than zero.
    pub speed: f32,
    /// Text box used to edit the value, only visible while editing.
    line_box: Shared<LineBox>,
    /// Area where the button has been pressed down, while it is pressed.
    pressed: Option<Zone>,
    /// Area under the mouse pointer, if any.
    hover_zone: Option<Zone>,
    /// True if the mouse has moved horizontally since the button has been pressed.
    dragged: bool,
    /// Mouse abscissa when the button has been pressed.
    press_x: f32,
    /// Value when the button has been pressed.
    press_value: f32,
    /// True while the value is edited in [Self::line_box].
    editing: bool,
    /// Set to true on the frames the value has been changed by the user.
    pub on_change: bool,
}

impl Numeric {
    /// Widget minimum width.
    const MIN_WIDTH: f32 = 80.0;
    /// Widget height.
    const HEIGHT: f32 = 21.0;
    /// Width of the decrement and increment arrows.
    const ARROW_WIDTH: f32 = 13.0;
    /// Arrow background color.
    const ARROW_IDLE_COLOR: Color = Color::new_gray(0.396);
    /// Hovered arrow background color.
    const ARROW_HOVER_COLOR: Color = Color::new_gray(0.463);

    /// Creates a new numeric widget with a zero value.
    pub fn new() -> Self {
        let line_box = LineBox::new().shared();
        line_box.borrow_mut().hide();
        Self {
            core: WidgetCore::new()
                .with_children(vec![line_box.clone()])
                .with_cursor(CursorIcon::EwResize),
            value: 0.0,
            min: None,
            max: None,
            step: 1.0,
            speed: 10.0,
            line_box,
            pressed: None,
            hover_zone: None,
            dragged: false,
            press_x: 0.0,
            press_value: 0.0,
            editing: false,
            on_change: false,
        }
    }

    /// Builds a numeric widget holding the given `value`.
    pub fn with_value(mut self, value: f32) -> Self {
        self.set_value(value);
        self
    }

    /// Builds a numeric widget whose value cannot be lower than `min`.
    pub fn with_min(mut self, min: f32) -> Self {
        self.set_min(Some(min));
        self
    }

    /// Builds a numeric widget whose value cannot be greater than `max`.
    pub fn with_max(mut self, max: f32) -> Self {
        self.set_max(Some(max));
        self
    }

    /// Builds a numeric widget with the given increment [Self::step].
    pub fn with_step(self, step: f32) -> Self {
        Self { step, ..self }
    }

    /// Builds a numeric widget with the given dragging [Self::speed].
    pub fn with_speed(self, speed: f32) -> Self {
        Self { speed, ..self }
    }

    /// Returns the current value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Sets the current value, clamped to the allowed range.
    pub fn set_value(&mut self, value: f32) {
        self.value = self.clamp(value);
    }

    /// Returns the lowest allowed value, if any.
    pub fn min(&self) -> Option<f32> {
        self.min
    }

    /// Sets the lowest allowed value, or removes the bound with `None`. The current value is
    /// clamped to the new range.
    pub fn set_min(&mut self, min: Option<f32>) {
        self.min = min;
        self.set_value(self.value);
    }

    /// Returns the highest allowed value, if any.
    pub fn max(&self) -> Option<f32> {
        self.max
    }

    /// Sets the highest allowed value, or removes the bound with `None`. The current value is
    /// clamped to the new range.
    pub fn set_max(&mut self, max: Option<f32>) {
        self.max = max;
        self.set_value(self.value);
    }

    /// Clamps `value` to the allowed range. If the minimum is greater than the maximum, the
    /// maximum wins.
    fn clamp(&self, value: f32) -> f32 {
        let value = self.min.map_or(value, |min| value.max(min));
        self.max.map_or(value, |max| value.min(max))
    }

    /// Shows the text box filled with the current value, fully selected, and gives it the focus.
    fn start_editing(&mut self) {
        let mut line_box = self.line_box.borrow_mut();
        line_box.set_text(&self.value.to_string());
        line_box.select_all();
        line_box.show();
        self.core.give_focus(line_box.uid());
        self.editing = true;
    }

    /// Hides the text box without changing the value.
    fn stop_editing(&mut self) {
        self.line_box.borrow_mut().hide();
        self.editing = false;
    }

    /// Replaces the value by the one typed in the text box, clamped to the allowed range, unless
    /// it is not a valid number, and hides the text box.
    fn commit(&mut self) {
        let parsed = self.line_box.borrow().text().trim().parse::<f32>();
        if let Ok(value) = parsed
            && value.is_finite()
        {
            self.change_value(value);
        }
        self.stop_editing();
    }

    /// Sets the value from a user action, clamped to the allowed range, and raises
    /// [Self::on_change] if it differs from the current one.
    fn change_value(&mut self, value: f32) {
        let value = self.clamp(value);
        if value != self.value {
            self.value = value;
            self.on_change = true;
        }
    }

    /// Handles the input while the value is edited in the text box.
    fn update_editing(&mut self, input: &Input) {
        let (submitted, line_box_focused) = {
            let line_box = self.line_box.borrow();
            (line_box.on_submit, line_box.core().focused())
        };
        if submitted {
            self.commit();
            self.core.request_focus();
        } else if line_box_focused && input.key_press(KeyCode::Escape) {
            self.stop_editing();
            self.core.request_focus();
        } else if input.mouse_left_press && !self.core.rect.contains(input.mouse_pos) {
            // The focus is left untouched, as the clicked widget may have requested it.
            self.commit();
        }
    }

    /// Returns the background color of the arrow at `zone`. The whole widget is dark while
    /// pressed, whatever the zone the press started in. While the middle area is hovered, the
    /// arrows keep the idle button color so that only the middle area is highlighted.
    fn arrow_color(&self, zone: Zone) -> Color {
        if self.pressed.is_some() {
            THEME.text_background_color
        } else if self.hover_zone == Some(zone) {
            Self::ARROW_HOVER_COLOR
        } else if self.hover_zone == Some(Zone::Value) {
            THEME.button_idle_color
        } else {
            Self::ARROW_IDLE_COLOR
        }
    }

    /// Returns the area of the widget at the window abscissa `x`.
    fn zone_at(&self, x: f32) -> Zone {
        if x < self.core.rect.x1 + Self::ARROW_WIDTH {
            Zone::Decrement
        } else if x >= self.core.rect.x2 - Self::ARROW_WIDTH {
            Zone::Increment
        } else {
            Zone::Value
        }
    }

    /// Handles the input while the value is displayed as a button.
    fn update_idle(&mut self, input: &Input) {
        if input.mouse_moved() {
            self.hover_zone = self.core.hover.then(|| self.zone_at(input.mouse_pos.x));
            self.core.cursor = match self.hover_zone {
                Some(Zone::Decrement | Zone::Increment) => CursorIcon::Default,
                Some(Zone::Value) | None => CursorIcon::EwResize,
            };
        }

        if self.core.hover && input.mouse_left_press && input.hit {
            self.pressed = Some(self.zone_at(input.mouse_pos.x));
            self.dragged = false;
            self.press_x = input.mouse_pos.x;
            self.press_value = self.value;
        }

        if self.pressed.is_some() && input.mouse_moved() {
            let dx = input.mouse_pos.x - self.press_x;
            if dx != 0.0 {
                self.dragged = true;
            }
            if self.speed > 0.0 {
                let increments = (dx / self.speed).trunc();
                self.change_value(self.press_value + increments * self.step);
            }
        }

        if input.mouse_left_release {
            if !self.dragged {
                match self.pressed {
                    Some(Zone::Decrement) => self.change_value(self.value - self.step),
                    Some(Zone::Value) => self.start_editing(),
                    Some(Zone::Increment) => self.change_value(self.value + self.step),
                    None => {}
                }
            }
            self.pressed = None;
        }
    }
}

impl Default for Numeric {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Numeric {
    impl_widget_core!();

    fn minimum_size(&mut self, _graphics: &mut Graphics) -> Vec2 {
        vec2(Self::MIN_WIDTH, Self::HEIGHT)
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        self.core.update(input);
        self.on_change = false;

        if self.editing {
            self.update_editing(input);
        } else {
            self.update_idle(input);
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        // The text box covers the whole widget while editing.
        if self.editing {
            return;
        }

        let rect = self.core.rect;
        // The bottom pixel row of the widget is left to the shadow, so the button body is one
        // pixel shorter.
        let body = rect + Rect::new(0.0, 0.0, 0.0, -1.0);

        // Shadow
        graphics.painter.round_rectangle(
            rect,
            Vec4::splat(4.0),
            Vec4::splat(-1.0),
            [THEME.shadow_color; 4],
        );

        // Background
        let bg_color = if self.pressed.is_some() {
            THEME.text_background_color
        } else if self.hover_zone == Some(Zone::Value) {
            THEME.button_hover_color
        } else {
            THEME.button_idle_color
        };
        graphics
            .painter
            .round_rectangle(body, Vec4::splat(4.0), Vec4::splat(-1.0), [bg_color; 4]);

        // Arrows, only shown while hovered, hidden while pressed in the middle area or dragging,
        // and only rounded on the widget outer corners.
        let decrement_rect = Rect::new(body.x1, body.y1, body.x1 + Self::ARROW_WIDTH, body.y2);
        let increment_rect = Rect::new(body.x2 - Self::ARROW_WIDTH, body.y1, body.x2, body.y2);
        let show_arrows = self.core.hover
            && match self.pressed {
                None => true,
                Some(Zone::Value) => false,
                Some(Zone::Decrement | Zone::Increment) => !self.dragged,
            };
        if show_arrows {
            graphics.painter.round_rectangle(
                decrement_rect,
                Vec4::new(4.0, 4.0, 0.0, 0.0),
                Vec4::splat(-1.0),
                [self.arrow_color(Zone::Decrement); 4],
            );
            graphics.painter.round_rectangle(
                increment_rect,
                Vec4::new(0.0, 0.0, 4.0, 4.0),
                Vec4::splat(-1.0),
                [self.arrow_color(Zone::Increment); 4],
            );
        }

        // Border
        graphics.painter.round_rectangle(
            body,
            Vec4::splat(4.0),
            Vec4::splat(3.0),
            [THEME.border_color; 4],
        );

        // Arrow chevrons
        if show_arrows {
            let stroke = Stroke::new(Color::WHITE, 1.5);
            let c = decrement_rect.center() + vec2(1.0, 0.0);
            graphics.painter.polyline(
                &[c + vec2(1.5, -3.5), c + vec2(-1.5, 0.0), c + vec2(1.5, 3.5)],
                stroke,
            );
            let c = increment_rect.center() - vec2(1.0, 0.0);
            graphics.painter.polyline(
                &[
                    c + vec2(-1.5, -3.5),
                    c + vec2(1.5, 0.0),
                    c + vec2(-1.5, 3.5),
                ],
                stroke,
            );
        }

        // Text
        // Like for buttons, the text is vertically centered with inclusion of the shadow.
        let style = FontStyle::new()
            .color(Color::WHITE)
            .shadow_color(Color::new_rgba(0.0, 0.0, 0.0, 0.5))
            .shadow_offset(vec2(0.0, 1.0));
        let layout = graphics.font.layout(
            &self.value.to_string(),
            rect + Rect::new(Self::ARROW_WIDTH, 0.0, -Self::ARROW_WIDTH, 0.0),
            TextHorizontalAlign::Center,
            style.size,
        );
        graphics.painter.text_layout(&graphics.font, &layout, style);
    }
}
