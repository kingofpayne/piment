use crate::{
    graphics::Graphics,
    impl_widget_core,
    rect::Rect,
    widgets::{SharedWidget, Widget, WidgetCore},
};
use glam::Vec2;

/// A widget with multiple children, but only one visible at a time.
pub struct MultiPage {
    /// Widget common properties.
    core: WidgetCore,
    /// Selected widget index.
    selected: usize,
}

impl MultiPage {
    pub fn new(children: Vec<SharedWidget>) -> Self {
        Self {
            core: WidgetCore::new().with_children(children),
            selected: 0,
        }
    }

    /// Changes selected page index.
    pub fn select(&mut self, graphics: &mut Graphics, index: usize) {
        if self.selected != index {
            self.selected = index;
            self.layout(graphics, self.core.rect);
        }
    }

    /// Changes selected page index and focus on the selected widget.
    pub fn select_and_focus(&mut self, graphics: &mut Graphics, index: usize) {
        let changed = self.selected != index;
        if changed {
            self.selected = index;
            self.layout(graphics, self.core.rect)
        }
        if index < self.core.children.len() {
            let target = self.core.children[index].borrow().uid();
            self.core.give_focus(target);
        }
    }
}

impl Widget for MultiPage {
    impl_widget_core!();

    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        self.core.layout(rect);
        for (i, w) in self.core.children.iter_mut().enumerate() {
            let mut w = w.borrow_mut();
            if i == self.selected {
                w.layout(graphics, rect);
                w.show();
            } else {
                w.layout(graphics, Rect::ZERO);
                w.hide();
            }
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        if self.core.children.is_empty() {
            return Vec2::ZERO;
        }
        self.core.children[self.selected]
            .borrow_mut()
            .minimum_size(graphics)
    }

    fn render(&mut self, _graphics: &mut Graphics) {}
}
