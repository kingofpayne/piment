//! Opens a window showing a horizontal sine wave on a black background. The phase and the
//! amplitude of the wave are animated over time.

use glam::{Vec2, vec2};
use piment::{
    App,
    color::Color,
    graphics::Graphics,
    impl_widget_core,
    input::Input,
    painter::Stroke,
    widgets::{Share, Widget, WidgetCore},
};
use std::f32::consts::TAU;

/// Widget drawing an animated sine wave.
struct SineWave {
    /// Widget common properties.
    core: WidgetCore,
    /// Phase of the wave, in radians.
    phase: f32,
    /// Amplitude of the wave, relative to the widget height.
    amplitude: f32,
}

impl SineWave {
    fn new() -> Self {
        Self {
            core: WidgetCore::new(),
            phase: 0.0,
            amplitude: 0.0,
        }
    }
}

impl Widget for SineWave {
    impl_widget_core!();

    fn minimum_size(&mut self, _graphics: &mut Graphics) -> Vec2 {
        vec2(600.0, 300.0)
    }

    fn update(&mut self, _graphics: &mut Graphics, input: &Input) {
        let time = input.time.as_secs_f32();
        self.phase = (TAU * 0.5 * time) % TAU;
        self.amplitude = 0.25 + 0.15 * (TAU * 0.2 * time).sin();
        self.core.request_animation_frame();
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let rect = self.core.rect;
        graphics.painter.rectangle(rect, [Color::BLACK; 4]);

        let amplitude = self.amplitude * rect.height();
        let center_y = rect.center().y;
        let points: Vec<Vec2> = (0..=256)
            .map(|i| {
                let t = i as f32 / 256.0;
                let x = rect.x1 + t * rect.width();
                let y = center_y + amplitude * (t * 3.0 * TAU + self.phase).sin();
                vec2(x, y)
            })
            .collect();
        graphics
            .painter
            .polyline(&points, Stroke::new(Color::CYAN, 2.0));
    }
}

fn main() {
    App::new("Sine wave").run(SineWave::new().shared());
}
