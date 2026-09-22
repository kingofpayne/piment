use crate::{
    axis::Axis,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    rect::Rect,
    theme::THEME,
    widgets::{Widget, WidgetCore},
};
use glam::{Vec2, Vec4, vec2};

/// Vertical or horizontal scrollbar widget that can be integrated in other widgets, such as
/// [crate::ux::widgets::list_box::ListBox].
pub struct ScrollBar {
    /// Widget common properties.
    core: WidgetCore,
    /// Scrollbar orientation.
    axis: Axis,
    /// Minimum range value.
    min: f32,
    /// Maximum range value.
    /// Always greater or equal to min.
    max: f32,
    /// Visible portion of the range.
    visible_size: f32,
    /// Current scroll position within the range.
    position: f32,
    /// True while the user is dragging the thumb.
    dragging: bool,
    /// Offset from the thumb start to the mouse grab point, in pixels.
    drag_offset: f32,
    /// Set to true when the user drags the thumb, causing the position to change.
    pub on_scroll: bool,
}

impl ScrollBar {
    /// Creates a new scrollbar, which is horizontal when `axis` is [Axis::X], or vertical when
    /// `axis` is [Axis::Y].
    ///
    /// For simpler code, [Self::horizontal] or [Self::vertical] can be used instead.
    pub fn new(axis: Axis) -> Self {
        Self {
            core: WidgetCore::new("ScrollBar"),
            axis,
            min: 0.0,
            max: 1.0,
            visible_size: 1.0,
            position: 0.0,
            dragging: false,
            drag_offset: 0.0,
            on_scroll: false,
        }
    }

    /// Creates a new horizontal scrollbar.
    pub fn horizontal() -> Self {
        Self::new(Axis::X)
    }

    /// Creates a new vertical scrollbar.
    pub fn vertical() -> Self {
        Self::new(Axis::Y)
    }

    /// Returns the current scroll position.
    pub fn position(&self) -> f32 {
        self.position
    }

    /// Sets the current scroll position, clamped to valid bounds.
    pub fn set_position(&mut self, position: f32) {
        self.position = position.clamp(self.min, (self.max - self.visible_size).max(self.min));
    }

    /// Sets the range.
    /// If `max` is lower than `min`, `min` is used instead.
    pub fn set_range(&mut self, min: f32, max: f32) {
        self.min = min;
        self.max = max.max(min);
        self.set_position(self.position);
    }

    /// Sets the visible portion of the range.
    pub fn set_visible_size(&mut self, visible_size: f32) {
        self.visible_size = visible_size;
        self.set_position(self.position);
    }

    /// Returns the scrollable range (max position minus min).
    fn scrollable_range(&self) -> f32 {
        (self.max - self.visible_size - self.min).max(0.0)
    }

    /// Returns the track length in pixels along the scrollbar axis.
    fn track_length(&self) -> f32 {
        self.core.rect.size_axis(self.axis)
    }

    /// Returns the thumb length in pixels.
    fn thumb_length(&self) -> f32 {
        self.track_length() * {
            let range = self.max - self.min;
            if range <= 0.0 {
                return 1.0;
            }
            (self.visible_size / range).clamp(0.0, 1.0)
        }
    }

    /// Returns the pixel offset of the thumb from the track start.
    fn thumb_offset(&self) -> f32 {
        let scrollable = self.scrollable_range();
        if scrollable <= 0.0 {
            return 0.0;
        }
        let travel = self.track_length() - self.thumb_length();
        ((self.position - self.min) / scrollable) * travel
    }

    /// Returns the thumb rectangle.
    fn thumb_rect(&self) -> Rect {
        let rect = self.core.rect;
        let offset = self.thumb_offset();
        let length = self.thumb_length();
        match self.axis {
            Axis::X => Rect::new(
                rect.x1 + offset,
                rect.y1,
                rect.x1 + offset + length,
                rect.y2,
            ),
            Axis::Y => Rect::new(
                rect.x1,
                rect.y1 + offset,
                rect.x2,
                rect.y1 + offset + length,
            ),
        }
    }

    /// Converts a pixel offset along the track to a scroll position.
    fn pixel_offset_to_position(&self, pixel_offset: f32) -> f32 {
        let travel = self.track_length() - self.thumb_length();
        if travel <= 0.0 {
            return self.min;
        }
        let ratio = (pixel_offset / travel).clamp(0.0, 1.0);
        self.min + ratio * self.scrollable_range()
    }

    /// Returns the mouse coordinate along the scrollbar axis.
    fn mouse_axis(&self, input: &Input) -> f32 {
        match self.axis {
            Axis::X => input.mouse_pos.x,
            Axis::Y => input.mouse_pos.y,
        }
    }

    /// Returns the track start coordinate.
    fn track_start(&self) -> f32 {
        match self.axis {
            Axis::X => self.core.rect.x1,
            Axis::Y => self.core.rect.y1,
        }
    }
}

impl Widget for ScrollBar {
    impl_widget_core!();

    fn minimum_size(&mut self, _graphics: &mut Graphics) -> Vec2 {
        match self.axis {
            Axis::X => vec2(0.0, THEME.scroll_bar_width),
            Axis::Y => vec2(THEME.scroll_bar_width, 0.0),
        }
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        self.on_scroll = false;

        if input.mouse_left_press && input.hit {
            let thumb = self.thumb_rect();
            if thumb.contains(input.mouse_pos) {
                self.dragging = true;
                self.drag_offset =
                    self.mouse_axis(input) - self.track_start() - self.thumb_offset();
            }
        }

        if self.dragging {
            if input.mouse_moved() {
                let new_pixel_offset =
                    self.mouse_axis(input) - self.track_start() - self.drag_offset;
                let new_position = self.pixel_offset_to_position(new_pixel_offset);
                if new_position != self.position {
                    self.position = new_position;
                    self.on_scroll = true;
                }
            }

            if input.mouse_left_release {
                self.dragging = false;
            }
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        if self.visible_size < (self.max - self.min) {
            let thumb_rect = self.thumb_rect();
            let radius = THEME.scroll_bar_width / 2.0;
            graphics.painter.round_rectangle(
                thumb_rect,
                Vec4::splat(radius),
                Vec4::splat(-1.0),
                [THEME.scroll_bar_color; 4],
            );
        }
    }
}
