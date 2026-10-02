use crate::{
    color::Color,
    font::{Font, TextHorizontalAlign},
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::FontStyle,
    rect::{IRect, Rect},
    theme::THEME,
    widgets::{Widget, WidgetCore},
};
use glam::{Vec2, Vec4, vec2};
use winit::{keyboard::KeyCode, window::CursorIcon};

/// A single line text box.
pub struct LineBox {
    /// Widget common properties.
    core: WidgetCore,
    /// Text being edited.
    text: String,
    /// Text when the edition started, restored by the escape key.
    initial_text: String,
    /// Character index where the current selection started.
    cursor_start: usize,
    /// Character index of the cursor.
    cursor_end: usize,
    /// Horizontal offset of the text in pixels, used to keep the cursor visible when the text is
    /// wider than the widget.
    scroll: f32,
    /// True while the user is selecting text by dragging the mouse.
    selecting: bool,
    /// True while the mouse button which gave the focus is held, to select the whole text when it
    /// is released, unless a range has been selected by dragging.
    select_all_on_release: bool,
    /// Set to true on the frames the text has been edited by the user.
    pub on_change: bool,
    /// Set to true on the frame the user presses the enter key or the focus is lost, only if the
    /// text has changed since the edition started.
    pub on_submit: bool,
}

impl LineBox {
    /// Horizontal space between the widget border and the text.
    const PADDING: f32 = 5.0;
    /// Width of the cursor bar.
    const CURSOR_WIDTH: f32 = 2.0;
    /// Widget height.
    const HEIGHT: f32 = 21.0;

    /// Creates a new empty line box.
    pub fn new() -> Self {
        Self {
            core: WidgetCore::new().with_cursor(CursorIcon::Text),
            text: String::new(),
            initial_text: String::new(),
            cursor_start: 0,
            cursor_end: 0,
            scroll: 0.0,
            selecting: false,
            select_all_on_release: false,
            on_change: false,
            on_submit: false,
        }
    }

    /// Builds a line box holding the given `text`.
    pub fn with_text(mut self, text: &str) -> Self {
        self.set_text(text);
        self
    }

    /// Returns the edited text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text and moves the cursor at its end. The new text is not considered as a
    /// user edit: it is the one restored by the escape key and won't raise [Self::on_submit].
    pub fn set_text(&mut self, text: &str) {
        self.text = text.into();
        self.initial_text = self.text.clone();
        self.set_cursor(self.char_count());
    }

    /// Returns the selection bounds as character indexes, lowest first. Both values are equal when
    /// no text is selected.
    pub fn selection(&self) -> (usize, usize) {
        (
            self.cursor_start.min(self.cursor_end),
            self.cursor_start.max(self.cursor_end),
        )
    }

    /// Returns the selected text, which is empty when there is no selection.
    pub fn selected_text(&self) -> &str {
        let (start, end) = self.selection();
        &self.text[self.byte_index(start)..self.byte_index(end)]
    }

    /// Selects the whole text.
    pub fn select_all(&mut self) {
        self.cursor_start = 0;
        self.cursor_end = self.char_count();
    }

    /// Returns the number of characters of the text.
    fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    /// Converts a character index into a byte offset in [Self::text].
    fn byte_index(&self, index: usize) -> usize {
        self.text
            .char_indices()
            .nth(index)
            .map(|(offset, _)| offset)
            .unwrap_or(self.text.len())
    }

    /// Moves both cursors at `index`, discarding the selection.
    fn set_cursor(&mut self, index: usize) {
        self.cursor_end = index.min(self.char_count());
        self.cursor_start = self.cursor_end;
    }

    /// Moves the end cursor at `index`. When `extend` is false, the start cursor is moved as well,
    /// which discards the selection.
    fn move_cursor(&mut self, index: usize, extend: bool) {
        self.cursor_end = index.min(self.char_count());
        if !extend {
            self.cursor_start = self.cursor_end;
        }
    }

    /// Returns the area the text is painted in, before horizontal scrolling is applied.
    fn text_rect(&self) -> Rect {
        Rect::new(
            self.core.rect.x1 + Self::PADDING,
            self.core.rect.y1,
            (self.core.rect.x2 - Self::PADDING).max(self.core.rect.x1 + Self::PADDING),
            (self.core.rect.y2 - 1.0).max(self.core.rect.y1),
        )
    }

    /// Returns the abscissa the first character of the text is painted at.
    fn text_origin(&self) -> f32 {
        (self.text_rect().x1 - self.scroll).round()
    }

    /// Returns the distance in pixels between the beginning of the text and the character at
    /// `index`.
    fn char_offset(&self, font: &Font, index: usize) -> f32 {
        font.text_width(&self.text[..self.byte_index(index)], THEME.font_size)
    }

    /// Returns the index of the character boundary which is the closest to the window abscissa
    /// `x`.
    fn index_at_x(&self, font: &Font, x: f32) -> usize {
        let x = x - self.text_origin();
        let mut offset = 0.0;
        let mut buffer = [0u8; 4];
        for (index, char) in self.text.chars().enumerate() {
            let advance = font.text_width(char.encode_utf8(&mut buffer), THEME.font_size);
            if x < offset + advance / 2.0 {
                return index;
            }
            offset += advance;
        }
        self.char_count()
    }

    /// Updates [Self::scroll] to keep the cursor visible in the widget.
    fn scroll_to_cursor(&mut self, font: &Font) {
        let width = self.text_rect().width();
        // Don't leave a gap on the right when the text has been shortened.
        let text_width = font.text_width(&self.text, THEME.font_size);
        self.scroll = self
            .scroll
            .min(text_width + Self::CURSOR_WIDTH - width)
            .max(0.0);
        let cursor = self.char_offset(font, self.cursor_end);
        if cursor < self.scroll {
            self.scroll = cursor;
        } else if cursor + Self::CURSOR_WIDTH - self.scroll > width {
            self.scroll = cursor + Self::CURSOR_WIDTH - width;
        }
    }

    /// Removes the characters in the `start..end` index range.
    fn remove_range(&mut self, start: usize, end: usize) {
        self.text
            .replace_range(self.byte_index(start)..self.byte_index(end), "");
    }

    /// Removes the selected text and places the cursor where the selection was. Returns false if
    /// there was nothing to remove.
    fn remove_selection(&mut self) -> bool {
        let (start, end) = self.selection();
        if start == end {
            return false;
        }
        self.remove_range(start, end);
        self.set_cursor(start);
        true
    }

    /// Inserts `text` at the cursor position, replacing the selection if any.
    fn insert(&mut self, text: &str) {
        self.remove_selection();
        let offset = self.byte_index(self.cursor_end);
        self.text.insert_str(offset, text);
        self.set_cursor(self.cursor_end + text.chars().count());
    }

    /// Raises [Self::on_submit] if the text has changed since the edition started, and makes the
    /// current text the new initial one.
    fn submit(&mut self) {
        if self.text != self.initial_text {
            self.initial_text = self.text.clone();
            self.on_submit = true;
        }
    }

    /// Handles the keyboard keys moving the cursor or editing the text.
    fn update_from_keyboard(&mut self, input: &Input) {
        let extend = input.shift_key_down();
        let (start, end) = self.selection();

        if input.key_press(KeyCode::ArrowLeft) {
            if !extend && start != end {
                self.set_cursor(start);
            } else {
                self.move_cursor(self.cursor_end.saturating_sub(1), extend);
            }
        } else if input.key_press(KeyCode::ArrowRight) {
            if !extend && start != end {
                self.set_cursor(end);
            } else {
                self.move_cursor(self.cursor_end + 1, extend);
            }
        } else if input.key_press(KeyCode::Home) {
            self.move_cursor(0, extend);
        } else if input.key_press(KeyCode::End) {
            self.move_cursor(self.char_count(), extend);
        } else if input.key_press(KeyCode::Backspace) {
            if self.remove_selection() {
                self.on_change = true;
            } else if self.cursor_end > 0 {
                self.remove_range(self.cursor_end - 1, self.cursor_end);
                self.set_cursor(self.cursor_end - 1);
                self.on_change = true;
            }
        } else if input.key_press(KeyCode::Delete) {
            if self.remove_selection() {
                self.on_change = true;
            } else if self.cursor_end < self.char_count() {
                self.remove_range(self.cursor_end, self.cursor_end + 1);
                self.on_change = true;
            }
        } else if input.key_press(KeyCode::Enter) {
            self.submit();
            self.core.release_focus();
        } else if input.key_press(KeyCode::Escape) {
            if self.text != self.initial_text {
                self.text = self.initial_text.clone();
                self.set_cursor(self.char_count());
                self.on_change = true;
            }
            self.core.release_focus();
        } else if input.key_press(KeyCode::KeyA) && input.control_key_down() {
            self.select_all();
        }

        // Control characters, such as the ones produced by the enter or backspace keys, are
        // handled above and must not be inserted in the text.
        if let Some(text) = &input.text
            && !input.control_key_down()
        {
            let text: String = text.chars().filter(|char| !char.is_control()).collect();
            if !text.is_empty() {
                self.insert(&text);
                self.on_change = true;
            }
        }
    }
}

impl Default for LineBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for LineBox {
    impl_widget_core!();

    fn minimum_size(&mut self, _graphics: &mut Graphics) -> Vec2 {
        vec2(200.0, Self::HEIGHT)
    }

    fn update(&mut self, graphics: &mut Graphics, input: &Input) {
        self.core.update(input);
        self.on_change = false;
        self.on_submit = false;

        if input.focus_gained {
            self.initial_text = self.text.clone();
            // When the focus is gained by a click, the text is selected on release instead.
            if !self.selecting {
                self.select_all();
            }
        } else if input.focus_lost {
            self.submit();
        }

        if input.mouse_left_press {
            if input.hit && self.core.hover {
                self.select_all_on_release = !self.core.focused();
                self.core.request_focus();
                let index = self.index_at_x(&graphics.font, input.mouse_pos.x);
                self.move_cursor(index, input.shift_key_down());
                self.selecting = true;
            } else if self.core.focused() {
                self.core.release_focus();
            }
        }

        if self.selecting {
            if input.mouse_moved() {
                let index = self.index_at_x(&graphics.font, input.mouse_pos.x);
                self.move_cursor(index, true);
            }
            if input.mouse_left_release {
                self.selecting = false;
                if self.select_all_on_release && self.cursor_start == self.cursor_end {
                    self.select_all();
                }
                self.select_all_on_release = false;
            }
        }

        if self.core.focused() {
            self.update_from_keyboard(input);
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        self.scroll_to_cursor(&graphics.font);
        let rect = self.core.rect;

        // Shadow
        graphics.painter.round_rectangle(
            rect,
            Vec4::splat(4.0),
            Vec4::splat(-1.0),
            [THEME.shadow_color; 4],
        );

        // Background
        graphics.painter.round_rectangle(
            rect + Rect::new(0.0, 0.0, 0.0, -1.0),
            Vec4::splat(4.0),
            Vec4::splat(-1.0),
            [THEME.text_background_color; 4],
        );

        // Border
        graphics.painter.round_rectangle(
            rect + Rect::new(0.0, 0.0, 0.0, -1.0),
            Vec4::splat(4.0),
            Vec4::splat(3.0),
            [THEME.border_color; 4],
        );

        // The text may be wider than the widget, clip it to the padded area.
        let text_rect = self.text_rect();
        let previous_scissor = graphics.painter.scissor;
        graphics.painter.scissor = previous_scissor.intersection(IRect::new(
            text_rect.x1 as i32,
            text_rect.y1 as i32,
            text_rect.x2 as i32,
            text_rect.y2 as i32,
        ));

        let origin = self.text_origin();

        let (y1, y2) = {
            let this = &self;
            (this.core.rect.y1 + 1.0, this.core.rect.y2 - 2.0)
        };

        // Selection
        let (start, end) = self.selection();
        if start != end && self.core.focused() {
            graphics.painter.rectangle(
                Rect::new(
                    origin + self.char_offset(&graphics.font, start),
                    y1,
                    origin + self.char_offset(&graphics.font, end),
                    y2,
                ),
                [THEME.active_color; 4],
            );
        }

        // Text
        let style = FontStyle::new().color(Color::WHITE);
        let layout = graphics.font.layout(
            &self.text,
            text_rect + vec2(-self.scroll, 0.0),
            TextHorizontalAlign::Left,
            style.size,
        );
        graphics.painter.text_layout(&graphics.font, &layout, style);

        // Cursor
        if self.core.focused() {
            let x = origin + self.char_offset(&graphics.font, self.cursor_end);
            graphics.painter.rectangle(
                Rect::new(x, y1, x + Self::CURSOR_WIDTH, y2),
                [THEME.text_cursor_color; 4],
            );
        }

        graphics.painter.scissor = previous_scissor;
    }
}
