//! Opens a window showing a gallery of the available widgets, one per row. Each row displays the
//! widget name next to an instance of the widget.

use glam::Vec2;
use piment::{
    App,
    graphics::Graphics,
    impl_widget_core,
    widgets::{
        Button, CheckBox, Grid, Label, LineBox, Numeric, Panel, RootMenu, Share, Shared,
        SharedWidget, Widget, WidgetCore,
    },
};

/// Builds a menu with a single button opening three sub-menus.
fn build_menu() -> RootMenu {
    let mut menu = RootMenu::new();
    let entries = menu.add_submenu("Open me");
    let mut entries = entries.borrow_mut();
    for i in 1..=3 {
        let sub_menu = entries.add_submenu(&format!("Menu {i}"));
        let mut sub_menu = sub_menu.borrow_mut();
        sub_menu.add("Item 1");
        sub_menu.add("Item 2");
    }
    menu
}

/// Root widget of the application.
struct MainWidget {
    /// Widget common properties.
    core: WidgetCore,
    /// Panel holding the whole interface.
    panel: Shared<Panel>,
}

impl MainWidget {
    fn new() -> Self {
        let rows: [(&str, SharedWidget); 6] = [
            ("Label", Label::new("Some text").shared()),
            ("Button", Button::new("Click me").shared()),
            ("CheckBox", CheckBox::new().with_label("Check me").shared()),
            ("LineBox", LineBox::new().with_text("Edit me").shared()),
            (
                "Numeric",
                Numeric::new()
                    .with_min(0.0)
                    .with_max(100.0)
                    .with_value(42.0)
                    .shared(),
            ),
            ("RootMenu", build_menu().shared()),
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
