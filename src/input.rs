use glam::{Vec2, vec2};
use std::collections::BTreeSet;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::KeyCode,
};

pub type Key = winit::keyboard::KeyCode;

/// User input state given to widgets.
/// This is simpler than dealing with a list of events.
#[derive(Debug, Clone)]
pub struct Input {
    /// Left mouse button has been pressed.
    pub mouse_left_press: bool,
    /// Left mouse button has been released.
    pub mouse_left_release: bool,
    /// Middle mouse button has been pressed.
    pub mouse_middle_press: bool,
    /// Middle mouse button has been released.
    pub mouse_middle_release: bool,
    /// Right mouse button has been pressed.
    pub mouse_right_press: bool,
    /// Right mouse button has been released.
    pub mouse_right_release: bool,
    /// Previous mouse position.
    /// Used to calculate delta when moving the cursor.
    pub previous_mouse_pos: Vec2,
    /// Latest mouse position.
    pub mouse_pos: Vec2,
    /// Mouse displacement.
    pub mouse_move: Vec2,
    /// True if the widget is directly under the mouse pointer and not under another widget.
    pub hit: bool,
    /// True if the widget has focus.
    /// Widgets can request or give focus using [crate::widgets::WidgetCore::request_focus] or
    /// [crate::widgets::WidgetCore::give_focus].
    pub focus: bool,
    /// True if the widget has gained the focus. This is raised during an update dedicated to the
    /// focus change, without any other event.
    pub focus_gained: bool,
    /// True if the widget has lost the focus. This is raised during an update dedicated to the
    /// focus change, without any other event.
    pub focus_lost: bool,
    /// Scroll delta.
    pub scroll: Vec2,
    /// Keyboard key has been pressed.
    key_press_value: Option<Key>,
    /// Keyboard key has been released.
    key_release_value: Option<Key>,
    /// Set of currently pressed keyboard keys.
    keys_down: BTreeSet<Key>,
    /// Enteredd text.
    pub text: Option<String>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            mouse_left_press: false,
            mouse_left_release: false,
            mouse_middle_press: false,
            mouse_middle_release: false,
            mouse_right_press: false,
            mouse_right_release: false,
            previous_mouse_pos: Vec2::ZERO,
            mouse_pos: Vec2::ZERO,
            mouse_move: Vec2::ZERO,
            hit: false,
            focus: false,
            focus_gained: false,
            focus_lost: false,
            scroll: Vec2::ZERO,
            key_press_value: None,
            key_release_value: None,
            keys_down: BTreeSet::new(),
            text: None,
        }
    }

    /// Update input state from a received winit window event.
    pub fn update_from_winit_event(&mut self, event: &WindowEvent) {
        self.mouse_left_press = false;
        self.mouse_left_release = false;
        self.mouse_middle_press = false;
        self.mouse_middle_release = false;
        self.mouse_right_press = false;
        self.mouse_right_release = false;
        self.previous_mouse_pos = self.mouse_pos;
        self.scroll = Vec2::ZERO;
        self.key_press_value = None;
        self.key_release_value = None;
        self.text = None;
        match event {
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => match (button, state) {
                (MouseButton::Left, ElementState::Pressed) => self.mouse_left_press = true,
                (MouseButton::Left, ElementState::Released) => self.mouse_left_release = true,
                (MouseButton::Middle, ElementState::Pressed) => self.mouse_middle_press = true,
                (MouseButton::Middle, ElementState::Released) => self.mouse_middle_release = true,
                (MouseButton::Right, ElementState::Pressed) => self.mouse_right_press = true,
                (MouseButton::Right, ElementState::Released) => self.mouse_right_release = true,
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    self.scroll = vec2(*x, *y);
                }
                MouseScrollDelta::PixelDelta(pos) => {
                    self.scroll = vec2(pos.x as f32, pos.y as f32);
                }
            },
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => self.mouse_pos = vec2(position.x as f32, position.y as f32),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => {
                if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.key_press_value = Some(code);
                            self.keys_down.insert(code);
                            self.text = event.text.as_ref().map(|t| t.to_string());
                        }
                        ElementState::Released => {
                            self.key_release_value = Some(code);
                            self.keys_down.remove(&code);
                        }
                    }
                }
            }
            _ => {}
        }
        self.mouse_move = self.mouse_pos - self.previous_mouse_pos;
    }

    /// Returns a copy of this input without any event, keeping only the mouse position and the
    /// keys down.
    pub fn without_events(&self) -> Self {
        Self {
            mouse_pos: self.mouse_pos,
            previous_mouse_pos: self.mouse_pos,
            keys_down: self.keys_down.clone(),
            ..Self::new()
        }
    }

    /// Returns `true` if any mouse button is pressed.
    pub fn mouse_press(&self) -> bool {
        self.mouse_left_press || self.mouse_right_press || self.mouse_middle_press
    }

    /// Returns `true` if `mouse_move` is not zero.
    pub fn mouse_moved(&self) -> bool {
        self.mouse_move != Vec2::ZERO
    }

    /// Returns `true` if mouse is scrolling vertically.
    pub fn scrolling_y(&self) -> bool {
        self.scroll.y != 0.0
    }

    /// Returns `true` if the given `key` has been pressed.
    pub fn key_press(&self, key: Key) -> bool {
        self.key_press_value == Some(key)
    }

    /// Returns `true` if the given `key` has been pressed and the widget has the focus.
    pub fn key_press_with_focus(&self, key: Key) -> bool {
        self.focus && self.key_press(key)
    }

    /// Returns `true` if the given `key` is currently down.
    pub fn key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns `true` if left control key or right control key is currently down.
    pub fn control_key_down(&self) -> bool {
        self.key_down(KeyCode::ControlLeft) || self.key_down(KeyCode::ControlRight)
    }

    /// Returns `true` if left shift key or right shift key is currently down.
    pub fn shift_key_down(&self) -> bool {
        self.key_down(KeyCode::ShiftLeft) || self.key_down(KeyCode::ShiftRight)
    }

    /// Returns true if typed text is equal to argument.
    pub fn text_is(&self, text: &str) -> bool {
        if let Some(s) = &self.text {
            s.as_str() == text
        } else {
            false
        }
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
