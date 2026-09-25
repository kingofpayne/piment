use crate::{
    axis::Axis,
    color::Color,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::FontStyle,
    rect::Rect,
    theme::THEME,
    widgets::{CheckBox, Layout, Share, Shared, Widget, WidgetCore},
};
use glam::{vec2, Vec2, Vec4};

/// Menu item.
pub struct MenuItem {
    /// Common widget properties.
    core: WidgetCore,
    /// Orientation of the menu item.
    axis: Axis,
    /// Text displayed on the menu.
    pub label: String,
    /// Sub-menu.
    sub_menu: Option<Shared<Menu>>,
    /// Optional checkbox.
    checkbox: Option<Shared<CheckBox>>,
    /// When true, menu is highlighted.
    highlighted: bool,
    /// When true, user has open the menu.
    open: bool,
    /// Menu click event.
    pub clicked: bool,
}

impl MenuItem {
    pub fn new(axis: Axis, label: &str) -> Self {
        Self {
            core: WidgetCore::new("MenuItem"),
            axis,
            label: label.into(),
            sub_menu: None,
            checkbox: None,
            highlighted: false,
            open: false,
            clicked: false,
        }
    }

    pub fn set_sub_menu(&mut self, menu: Shared<Menu>) {
        self.sub_menu = Some(menu);
        self.refresh_children();
    }

    pub fn add_checkbox(&mut self) -> Shared<CheckBox> {
        let checkbox = CheckBox::new().shared();
        self.checkbox = Some(checkbox.clone());
        self.refresh_children();
        checkbox
    }

    /// Rebuilds the list of widget in `core`.
    fn refresh_children(&mut self) {
        self.core.children.clear();
        if let Some(w) = &self.sub_menu {
            self.core.children.push(w.clone());
        }
        if let Some(w) = &self.checkbox {
            self.core.children.push(w.clone());
        }
    }

    /// Show or hide the submenu depending on current stage.
    fn update_sub_menu_visibility(&mut self) {
        if let Some(menu) = &self.sub_menu {
            let mut menu = menu.borrow_mut();
            menu.core.visible = self.highlighted && self.open && !menu.clicked;
            self.open &= !menu.clicked;
        }
    }

    /// Mark the menu as open or close.
    fn set_open(&mut self, value: bool) {
        self.open = value;
        self.update_sub_menu_visibility();
    }
}

impl Widget for MenuItem {
    impl_widget_core!();

    fn layout(&mut self, graphics: &mut Graphics, rect: Rect) {
        self.core.layout(rect);
        if let Some(checkbox) = &self.checkbox {
            checkbox.borrow_mut().layout(graphics, rect.expand(-2.0));
        }
        if let Some(menu) = &self.sub_menu {
            let mut menu = menu.borrow_mut();
            let size = menu.minimum_size(graphics);
            let anchor = match self.axis {
                Axis::Y => rect.x2y1(),
                Axis::X => rect.x1y2(),
            };
            menu.layout(graphics, Rect::from_pos_size(anchor, size));
        }
    }

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        let margin = match self.axis {
            Axis::X => 10.0,
            Axis::Y => 30.0,
        };
        vec2(
            graphics.font.text_width(&self.label, THEME.font_size) + margin,
            20.0,
        )
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        self.core.update(input);
        self.clicked = false;
        if input.mouse_left_press && input.hit {
        } else if input.mouse_left_release && input.hit {
            self.clicked = self.core.rect.contains(input.mouse_pos)
        } else if input.mouse_moved() {
            let u = self.core.rect.contains_x(input.mouse_pos.x);
            let v = self.core.rect.contains_y(input.mouse_pos.y);
            self.highlighted = (u && v)
                || (self.open
                    && self.highlighted
                    && !match self.axis {
                        Axis::Y => u,
                        Axis::X => v,
                    });
        }
        if let Some(sub_menu) = &self.sub_menu {
            self.clicked |= sub_menu.borrow().clicked;
        }
        self.update_sub_menu_visibility();
    }

    fn render(&mut self, graphics: &mut Graphics) {
        if self.highlighted {
            let rad_in = Vec4::splat(-1.0);
            let rad_out = Vec4::splat(3.0);
            graphics.painter.round_rectangle(
                self.core.rect + vec2(0.0, 1.0),
                rad_out,
                rad_in,
                [Color::BLACK.with_alpha(0.25); 4],
            );
            graphics.painter.round_rectangle(
                self.core.rect,
                rad_out,
                rad_in,
                [Color::new_gray(0.247); 4],
            );
        }
        let margin = match self.axis {
            Axis::X => 5.0,
            Axis::Y => 25.0,
        };
        graphics.painter.text(
            &graphics.font,
            &self.label,
            self.core.rect + Rect::new(margin, 0.0, 0.0, 0.0),
            FontStyle::new(),
        );
    }
}

/// Vertical menu.
pub struct Menu {
    /// Common widget properties.
    core: WidgetCore,
    /// Menu vertical layout.
    layout: Shared<Layout>,
    /// Menu items.
    items: Vec<Shared<MenuItem>>,
    /// Clicked event. Fired when a child `MenuItem` is clicked.
    /// This is used to close the menu.
    clicked: bool,
}

impl Menu {
    /// Creates a new menu.
    ///
    /// If `axis` is [Axis::Y], the menu is vertical (suited for most menus).
    /// If `axis` is [Axis::X], the menu is horizontal (suited for a main menu bar).
    ///
    /// [Self::horizontal] or [Self::vertical] shall be preferably used whenever possible.
    pub fn new(axis: Axis) -> Self {
        let layout = Layout::new(axis).with_margin(Vec4::splat(3.0)).shared();
        Self {
            core: WidgetCore::new("Menu")
                .with_children(vec![layout.clone()])
                .with_above(),
            layout,
            items: Vec::new(),
            clicked: false,
        }
    }

    /// Creates a new vertical menu.
    pub fn vertical() -> Self {
        Self::new(Axis::Y)
    }

    /// Creates a new horizontal menu bar.
    pub fn horizontal() -> Self {
        Self::new(Axis::X)
    }

    /// Creates and returns a new menu entry with given `label`.
    pub fn add(&mut self, label: &str) -> Shared<MenuItem> {
        let item = MenuItem::new(Axis::Y, label).shared();
        self.layout.borrow_mut().add(item.clone());
        self.items.push(item.clone());
        item
    }

    /// Creates and returns a new sub-menu.
    pub fn add_submenu(&mut self, label: &str) -> Shared<Menu> {
        let item = self.add(label);
        let sub_menu = Menu::vertical().shared();
        item.borrow_mut().set_sub_menu(sub_menu.clone());
        sub_menu
    }

    /// Creates a new checkbox menu item.
    pub fn add_checkbox(&mut self, label: &str) -> Shared<CheckBox> {
        let item = self.add(label);
        item.borrow_mut().add_checkbox()
    }
}

impl Widget for Menu {
    impl_widget_core!();

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        self.layout.borrow_mut().minimum_size(graphics)
    }

    fn update(&mut self, _graphics: &mut Graphics, _input: &Input) {
        self.clicked = self.items.iter().any(|item| item.borrow().clicked);
    }

    fn render(&mut self, graphics: &mut Graphics) {
        graphics.painter.round_rectangle(
            self.core.rect,
            Vec4::splat(4.0),
            Vec4::splat(-1.0),
            [Color::new_gray(0.141); 4],
        );
        graphics.painter.round_rectangle(
            self.core.rect.expand(-1.0),
            Vec4::splat(3.0),
            Vec4::splat(-1.0),
            [Color::new_gray(0.094); 4],
        );
    }
}

/// Root horizontal menu.
pub struct RootMenu {
    /// Common widget properties.
    core: WidgetCore,
    /// Menu horizontal layout.
    layout: Shared<Layout>,
    /// Menu items.
    items: Vec<Shared<MenuItem>>,
    /// True when menu is open.
    open: bool,
    /// When true, paint background color.
    draw_background: bool,
}

impl RootMenu {
    /// Creates a new menu.
    pub fn new() -> Self {
        let layout = Layout::horizontal().shared();
        Self {
            core: WidgetCore::new("RootMenu").with_children(vec![layout.clone()]),
            layout,
            items: Vec::new(),
            open: false,
            draw_background: false,
        }
    }

    /// Creates and returns a new menu item with given `label`.
    pub fn add(&mut self, label: &str) -> Shared<MenuItem> {
        let item = MenuItem::new(Axis::X, label).shared();
        self.layout.borrow_mut().add(item.clone());
        self.items.push(item.clone());
        item
    }

    /// Creates and returns a new sub-menu with given `label`.
    pub fn add_submenu(&mut self, label: &str) -> Shared<Menu> {
        let item = self.add(label);
        let sub_menu = Menu::vertical().shared();
        item.borrow_mut().set_sub_menu(sub_menu.clone());
        sub_menu
    }
}

impl Default for RootMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for RootMenu {
    impl_widget_core!();

    fn minimum_size(&mut self, graphics: &mut Graphics) -> Vec2 {
        self.layout.borrow_mut().minimum_size(graphics)
    }

    fn update(&mut self, _graphics: &mut Graphics, _input: &Input) {
        self.open ^= self.items.iter().any(|item| item.borrow().clicked);
        for item in self.items.iter_mut() {
            item.borrow_mut().set_open(self.open);
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        if self.draw_background {
            graphics.painter.round_rectangle(
                self.core.rect,
                Vec4::splat(1.0),
                Vec4::splat(-1.0),
                [Color::new_gray(0.094); 4],
            );
        }
    }
}
