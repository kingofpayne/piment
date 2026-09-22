//! Opens a window showing a label with a button underneath it. Clicking the button increments a
//! counter displayed by the label.
//!
//! Must be run from the repository root.

use glam::Vec2;
use piment::{
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    run_widget,
    widgets::{Button, Label, Layout, Panel, Share, Shared, Widget, WidgetCore},
};

/// Root widget of the application.
///
/// It builds and embeds the widget tree, and counts how many times the button has been clicked.
struct App {
    /// Widget common properties.
    core: WidgetCore,
    /// Panel holding the whole interface.
    panel: Shared<Panel>,
    /// Label displaying the click count.
    label: Shared<Label>,
    /// Button incrementing the counter.
    button: Shared<Button>,
    /// Number of times the button has been clicked.
    counter: u32,
}

impl App {
    fn new() -> Self {
        let label = Label::new("Hello").shared();
        let button = Button::new("Click me").shared();

        let mut column = Layout::vertical();
        column.add(label.clone());
        column.add(button.clone());

        let mut panel = Panel::new();
        panel.set_child(column.shared());
        let panel = panel.shared();

        Self {
            core: WidgetCore::new("App").with_children(vec![panel.clone()]),
            panel,
            label,
            button,
            counter: 0,
        }
    }
}

impl Widget for App {
    impl_widget_core!();

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        self.panel.borrow_mut().minimum_size(graphics)
    }

    fn update(&mut self, _graphics: &mut Graphics, _input: &Input) {
        if self.button.borrow().on_click {
            self.counter += 1;
            self.label
                .borrow_mut()
                .set_text(&format!("Clicked {} times", self.counter));
        }
    }
}

fn main() {
    run_widget(App::new().shared());
}
