use crate::{
    color::Color,
    font::TextHorizontalAlign,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::{FontStyle, Stroke},
    rect::Rect,
    widgets::{SharedWidget, Widget, WidgetCore},
};
use glam::{Vec2, Vec4, vec2};

/// A panel with a title header, holding one child widget which can be shown or hidden by clicking
/// on the header.
pub struct CollapsiblePanel {
    /// Widget common properties.
    core: WidgetCore,
    /// Text displayed in the header.
    title: String,
    /// Child widget, only visible while the panel is expanded.
    child: Option<SharedWidget>,
    /// True when the child is visible.
    expanded: bool,
    /// Set to true on the frame the user expanded or collapsed the panel.
    pub on_toggle: bool,
}

impl CollapsiblePanel {
    /// Height of the header, which is also the height of the collapsed panel.
    const HEADER_HEIGHT: f32 = 25.0;
    /// Space between the child and the left, right and bottom panel edges.
    const PADDING: f32 = 4.0;
    /// Panel corner radius.
    const CORNER_RADIUS: f32 = 4.0;
    /// Panel background color.
    const BACKGROUND_COLOR: Color = Color::new_gray(0.239);
    /// Panel border color.
    const BORDER_COLOR: Color = Color::new_gray(0.290);
    /// Horizontal position of the arrow center, from the panel left edge.
    const ARROW_CENTER_X: f32 = 10.0;
    /// Horizontal position of the title, from the panel left edge.
    const TEXT_OFFSET: f32 = 18.0;

    /// Creates a new expanded panel with the given header `title` and no child.
    pub fn new(title: &str) -> Self {
        Self {
            core: WidgetCore::new(),
            title: title.into(),
            child: None,
            expanded: true,
            on_toggle: false,
        }
    }

    /// Builds a panel holding the given `child`.
    pub fn with_child(mut self, child: SharedWidget) -> Self {
        self.set_child(child);
        self
    }

    /// Builds a collapsed panel.
    pub fn with_collapsed(mut self) -> Self {
        self.set_expanded(false);
        self
    }

    /// Replaces the child widget.
    pub fn set_child(&mut self, child: SharedWidget) {
        child.borrow_mut().core_mut().visible = self.expanded;
        self.child = Some(child.clone());
        self.core.children = vec![child];
        self.core.request_layout();
    }

    /// Returns true when the child is visible.
    pub fn expanded(&self) -> bool {
        self.expanded
    }

    /// Shows the child when `expanded` is true, hides it otherwise.
    pub fn set_expanded(&mut self, expanded: bool) {
        if expanded == self.expanded {
            return;
        }
        self.expanded = expanded;
        if let Some(child) = &self.child {
            child.borrow_mut().core_mut().visible = expanded;
        }
        self.core.request_layout();
    }

    /// Returns the header area, in window coordinates.
    fn header_rect(&self) -> Rect {
        let rect = self.core.rect;
        Rect::new(rect.x1, rect.y1, rect.x2, rect.y1 + Self::HEADER_HEIGHT)
    }
}

impl Widget for CollapsiblePanel {
    impl_widget_core!();

    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        self.core.layout(rect);
        if let Some(child) = &self.child {
            let child_rect = if self.expanded {
                Rect::new(
                    rect.x1 + Self::PADDING,
                    rect.y1 + Self::HEADER_HEIGHT,
                    rect.x2 - Self::PADDING,
                    rect.y2 - Self::PADDING,
                )
            } else {
                Rect::ZERO
            };
            child.borrow_mut().layout(graphics, child_rect);
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        let title_width = Self::TEXT_OFFSET
            + graphics.painter.text_width(&self.title, &FontStyle::new())
            + Self::PADDING;
        let mut size = vec2(title_width, Self::HEADER_HEIGHT);
        if self.expanded
            && let Some(child) = &self.child
        {
            let child_size = child.borrow_mut().minimum_size(graphics);
            size.x = size.x.max(child_size.x + Self::PADDING * 2.0);
            size.y += child_size.y + Self::PADDING;
        }
        size
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        self.core.update(input);
        self.on_toggle = false;

        if input.hit && input.mouse_left_press && self.header_rect().contains(input.mouse_pos) {
            self.set_expanded(!self.expanded);
            self.on_toggle = true;
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let rect = self.core.rect;
        let header = self.header_rect();

        // Background
        graphics.painter.round_rectangle(
            rect,
            Vec4::splat(Self::CORNER_RADIUS),
            Vec4::splat(-1.0),
            [Self::BACKGROUND_COLOR; 4],
        );

        // Border
        graphics.painter.round_rectangle(
            rect,
            Vec4::splat(Self::CORNER_RADIUS),
            Vec4::splat(Self::CORNER_RADIUS - 1.0),
            [Self::BORDER_COLOR; 4],
        );

        // Arrow
        let c = vec2(rect.x1 + Self::ARROW_CENTER_X, header.v_center());
        let points = if self.expanded {
            [
                c + vec2(-4.0, -2.0),
                c + vec2(0.0, 2.0),
                c + vec2(4.0, -2.0),
            ]
        } else {
            [
                c + vec2(-2.0, -4.0),
                c + vec2(2.0, 0.0),
                c + vec2(-2.0, 4.0),
            ]
        };
        graphics
            .painter
            .polyline(&points, Stroke::new(Color::WHITE, 1.0));

        // Title
        let style = FontStyle::new()
            .color(Color::WHITE)
            .shadow_color(Color::new_rgba(0.0, 0.0, 0.0, 0.5))
            .shadow_offset(vec2(0.0, 1.0));
        graphics.painter.text(
            &self.title,
            header + Rect::new(Self::TEXT_OFFSET, 0.0, 0.0, 0.0),
            TextHorizontalAlign::Left,
            &style,
        );
    }
}
