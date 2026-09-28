//! Opens a window showing a gallery of the available widgets, one per row. Each row displays the
//! widget name next to an instance of the widget.

use glam::Vec2;
use piment::{
    App,
    graphics::Graphics,
    impl_widget_core,
    widgets::{
        Button, CheckBox, Grid, Label, LineBox, Panel, Share, Shared, SharedWidget, Widget,
        WidgetCore,
    },
};

/// Root widget of the application.
struct MainWidget {
    /// Widget common properties.
    core: WidgetCore,
    /// Panel holding the whole interface.
    panel: Shared<Panel>,
}

impl MainWidget {
    fn new() -> Self {
        let rows: [(&str, SharedWidget); 4] = [
            ("Label", Label::new("Some text").shared()),
            ("Button", Button::new("Click me").shared()),
            ("CheckBox", CheckBox::new().with_label("Check me").shared()),
            ("LineBox", LineBox::new().with_text("Edit me").shared()),
        ];

        let mut grid = Grid::new();
        for (row, (name, widget)) in rows.into_iter().enumerate() {
            grid.insert(row, 0, Label::new(name).shared());
            grid.insert(row, 1, widget);
        }

        let mut panel = Panel::new();
        panel.set_child(grid.shared());
        let panel = panel.shared();

        Self {
            core: WidgetCore::new().with_children(vec![panel.clone()]),
            panel,
        }
    }
}

impl Widget for MainWidget {
    impl_widget_core!();

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        self.panel.borrow_mut().minimum_size(graphics)
    }
}

fn main() {
    App::new().run(MainWidget::new().shared());
}
