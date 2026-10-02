//! Opens a window showing a sentence rendered at several sizes from a single distance field font
//! atlas, then with an outline and with a shadow.

use glam::{Vec2, vec2};
use piment::{
    App,
    color::Color,
    font_sdf::FontSdf,
    graphics::Graphics,
    impl_widget_core,
    painter::FontStyle,
    rect::Rect,
    widgets::{Panel, Share, Shared, Widget, WidgetCore},
};

/// Sentence painted on every row.
const TEXT: &str = "The quick brown fox jumps over the lazy dog";

/// Paints one row of text per style.
struct Samples {
    /// Widget common properties.
    core: WidgetCore,
    /// Font used for every row.
    font: FontSdf,
    /// Style of each row.
    styles: Vec<FontStyle>,
}

impl Samples {
    fn new() -> Self {
        let mut styles: Vec<FontStyle> = [11, 14, 18, 24, 32, 48, 72]
            .into_iter()
            .map(|size| FontStyle::new().size(size))
            .collect();
        styles.push(FontStyle::new().size(32).outline_color(Color::BLACK));
        styles.push(
            FontStyle::new()
                .size(32)
                .shadow_color(Color::BLACK.with_alpha(0.75))
                .shadow_offset(vec2(2.0, 2.0)),
        );
        Self {
            core: WidgetCore::new(),
            font: FontSdf::from_system(),
            styles,
        }
    }

    /// Returns the height of a row painted with `style`.
    fn row_height(style: &FontStyle) -> f32 {
        (style.size as f32 * 1.5).ceil()
    }
}

impl Widget for Samples {
    impl_widget_core!();

    fn minimum_size(&mut self, _graphics: &mut Graphics) -> Vec2 {
        let width = self
            .styles
            .iter()
            .map(|style| self.font.text_width(TEXT, style.size as f32))
            .fold(0.0, f32::max);
        let height = self.styles.iter().map(Self::row_height).sum();
        vec2(width, height)
    }

    fn render(&mut self, graphics: &mut Graphics) {
        if self.font.texture().is_none() {
            self.font.build_texture(&graphics.device, &graphics.queue);
        }
        let mut y = self.core.rect.y1;
        for style in self.styles.iter() {
            let height = Self::row_height(style);
            let rect = Rect::new(self.core.rect.x1, y, self.core.rect.x2, y + height);
            graphics.painter.text_sdf(&self.font, TEXT, rect, *style);
            y += height;
        }
    }
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
        let mut panel = Panel::new();
        panel.set_child(Samples::new().shared());
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
    App::new("Font SDF").run(MainWidget::new().shared());
}
