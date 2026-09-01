use crate::gfx::{font_data, shapes, Canvas, Color, Surface};
use crate::widgets::{self, RollingClock};

use super::{App, AppInput};

/// The top bar's rolling clock, given a window of its own with the seconds on.
pub struct ClockApp {
    clock: RollingClock,
    phase: f32,
}

impl ClockApp {
    pub fn new() -> Self {
        Self {
            clock: RollingClock::new(&font_data::FONT_CLOCK, true),
            phase: 0.0,
        }
    }
}

impl App for ClockApp {
    fn title(&self) -> &'static str {
        "Uhr"
    }

    fn set_clock(&mut self, day_seconds: u32) {
        self.clock.set(day_seconds);
    }

    fn update(&mut self, dt: f32, _input: &AppInput) {
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        self.clock.update(dt);
    }

    fn draw(&self, surface: &mut Surface) {
        shapes::vertical_gradient(
            surface,
            Color::rgba(255, 255, 255, 16),
            Color::rgba(255, 255, 255, 34),
            Color::rgba(255, 255, 255, 54),
        );
        let cx = surface.width() as i32 / 2;
        let baseline = surface.height() as i32 / 2 + self.clock.digit_height() / 2 - 8;
        let pulse = 4.0 * self.phase * (1.0 - self.phase);
        self.clock.draw(surface, cx, baseline, 1.0, pulse);
        widgets::text_embossed(surface, &font_data::FONT_UI, "Ortszeit", cx, baseline + 34, 150);
    }
}
