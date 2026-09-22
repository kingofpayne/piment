use crate::color::Color;

/// Groups UX appearance parameters, such as colors.
pub struct Theme {
    /// Main font size.
    pub font_size: i32,
    /// Panel and menu bar color.
    pub panel_color: Color,
    /// Widget active color (blue).
    /// Used for pressed buttons, checked checkboxes, selected menu items...
    pub active_color: Color,
    /// Border color for buttons, line box.
    pub border_color: Color,
    /// Idle button backgound color.
    pub button_idle_color: Color,
    /// Hovered button background color.
    pub button_hover_color: Color,
    /// Widget shadow color.
    pub shadow_color: Color,
    /// Selection color.
    pub selection_color: Color,
    /// Scrollbar thumb color.
    pub scroll_bar_color: Color,
    /// Scrollbar width (or height for horizontal) in pixels.
    pub scroll_bar_width: f32,
    /// Text editors background color (line box).
    pub text_background_color: Color,
    /// Text cursor color.
    pub text_cursor_color: Color,
}

pub const THEME: Theme = Theme {
    font_size: 11,
    panel_color: Color::new_gray(0.094),
    active_color: Color::from_hex_rgb(0x4772b3),
    border_color: Color::new_gray(0.239),
    button_idle_color: Color::new_gray(0.329),
    button_hover_color: Color::new_gray(0.396),
    shadow_color: Color::BLACK.with_alpha(0.25),
    selection_color: Color::new_rgb(1.0, 0.6, 0.0),
    scroll_bar_color: Color::new_gray(0.2),
    scroll_bar_width: 5.0,
    text_background_color: Color::new_gray(0.094),
    text_cursor_color: Color::from_hex_rgb(0x7db6ff).with_alpha(0.8),
};
