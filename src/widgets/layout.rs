use crate::{
    axis::{Axis, AxisVec2},
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    rect::Rect,
    widgets::{SharedWidget, Widget, WidgetCore},
};
use glam::{Vec2, Vec4, vec2};

pub struct Layout {
    /// Widget common properties.
    core: WidgetCore,
    /// Layout direction.
    axis: Axis,
    /// Children widgets.
    children: Vec<Item>,
    /// Margin, left, top, right, bottom order.
    margin: Vec4,
}

struct Item {
    /// Widget to be placed, or None for an empty space.
    widget: Option<SharedWidget>,
    /// Minimum size. If Widget minimum size is higher, this value is ignored.
    min: f32,
    /// Priority attributed to this widget for space sharing.
    /// If 0, widget won't extend.
    weight: f32,
    /// Previous visibility flag of the widget.
    /// Used to detect visibility changes and recalculate the layout when necessary.
    previous_visible: bool,
}

impl Layout {
    /// Creates a new layout, which is horizontal when `axis` is [Axis::X], or vertical when `axis`
    /// is [Axis::Y].
    ///
    /// For simpler code, [Self::horizontal] or [Self::vertical] can be used instead.
    pub fn new(axis: Axis) -> Self {
        Self {
            core: WidgetCore::new(),
            axis,
            children: Vec::new(),
            margin: Vec4::ZERO,
        }
    }

    /// Creates a new horizontal layout.
    pub fn horizontal() -> Self {
        Self::new(Axis::X)
    }

    /// Creates a new vertical layout.
    pub fn vertical() -> Self {
        Self::new(Axis::Y)
    }

    /// Defines the margin. Elements of the given vector correspond respectively to left, top,
    /// right and bottom sides.
    pub fn with_margin(mut self, margin: Vec4) -> Self {
        self.margin = margin;
        self
    }

    /// Appends a new widget to the layout.
    pub fn add(&mut self, widget: SharedWidget) {
        self.core.children.push(widget.clone());
        self.children.push(Item {
            widget: Some(widget.clone()),
            min: 0.0,
            weight: 0.0,
            previous_visible: widget.borrow().core().visible,
        });
    }

    /// Appends a new widget to the layout, allowed to grow to fill remaining space with the given
    /// weight priority.
    pub fn add_expand(&mut self, widget: SharedWidget, weight: f32) {
        self.core.children.push(widget.clone());
        self.children.push(Item {
            widget: Some(widget.clone()),
            min: 0.0,
            weight,
            previous_visible: widget.borrow().core().visible,
        });
    }

    /// Adds an empty space with given expansion weight.
    pub fn add_space(&mut self, min: f32, weight: f32) {
        self.children.push(Item {
            widget: None,
            min,
            weight,
            previous_visible: true,
        })
    }
}

impl Widget for Layout {
    impl_widget_core!();

    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        self.core.layout(rect);
        // Apply margin.
        let rect = Rect::new(
            rect.x1 + self.margin.x,
            rect.y1 + self.margin.y,
            rect.x2 - self.margin.z,
            rect.y2 - self.margin.w,
        );
        // Get minimum required size
        let minimum_size = self.minimum_size(graphics).axis(self.axis);
        let available_size = rect.size_axis(self.axis);
        let mut size_to_distribute = (available_size - minimum_size).max(0.0);
        // Calculate the sum of all weight
        let mut weight_sum: f32 = self.children.iter().map(|item| item.weight).sum();

        let mut offset = 0.0f32;
        for item in self.children.iter_mut().filter(|item| {
            item.widget
                .as_ref()
                .is_none_or(|w| w.borrow().core().visible)
        }) {
            let mut size = item
                .widget
                .as_ref()
                .map(|w| w.borrow_mut().minimum_size(graphics))
                .unwrap_or(vec2(0.0, 0.0));
            size[self.axis as usize] = size[self.axis as usize].max(item.min);
            if item.weight > 0.0 {
                let extra = ((item.weight / weight_sum) * size_to_distribute).floor();
                size_to_distribute -= extra;
                weight_sum -= item.weight;
                size += extra;
            }
            let add = size.axis(self.axis);
            let rect = match self.axis {
                Axis::X => Rect::new(rect.x1 + offset, rect.y1, rect.x1 + offset + add, rect.y2),
                Axis::Y => Rect::new(rect.x1, rect.y1 + offset, rect.x2, rect.y1 + offset + add),
            };
            if let Some(widget) = &mut item.widget {
                let mut widget = widget.borrow_mut();
                widget.layout(graphics, rect);
            }
            offset += add;
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        let mut a: f32 = 0.0;
        let mut b: f32 = 0.0;
        for item in self.children.iter().filter(|item| {
            item.widget
                .as_ref()
                .is_none_or(|w| w.borrow().core().visible)
        }) {
            let size = item
                .widget
                .as_ref()
                .map(|w| w.borrow_mut().minimum_size(graphics))
                .unwrap_or(vec2(0.0, 0.0));
            a += size.axis(self.axis).max(item.min);
            b = b.max(size.axis(self.axis.next()));
        }
        let m = vec2(self.margin.x + self.margin.z, self.margin.y + self.margin.w);
        match self.axis {
            Axis::X => vec2(a, b) + m,
            Axis::Y => vec2(b, a) + m,
        }
    }

    fn update(&mut self, graphics: &mut Graphics, _input: &Input) {
        // Detect visibility change in children and trigger layout recalculation if necessary.
        let mut recalculate = false;
        for item in self.children.iter_mut() {
            if let Some(widget) = &mut item.widget {
                let visible = widget.borrow().core().visible;
                if visible != item.previous_visible {
                    recalculate = true;
                    item.previous_visible = visible;
                }
            }
        }
        if recalculate {
            self.relayout(graphics);
        }
    }
}
