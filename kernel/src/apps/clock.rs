use crate::gfx::{font_data, shapes, Canvas, Color, Rect, Surface};
use crate::widgets::{self, RollingClock};

use super::{App, AppInput};

pub struct ClockApp {
    clock: RollingClock,
    phase: f32,
    width: i32,
    height: i32,
}

impl ClockApp {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            clock: RollingClock::new(&font_data::FONT_CLOCK, true),
            phase: 0.0,
            width: width as i32,
            height: height as i32,
        }
    }
}

impl App for ClockApp {
    fn title(&self) -> &'static str {
        "Uhr"
    }

    fn painted(&self) -> Option<Rect> {
        let digit_h = self.clock.digit_height();
        let baseline = self.height / 2 + digit_h / 2 - 8;
        let half = self.clock.width() / 2 + 8;
        Some(Rect::new(
            self.width / 2 - half,
            baseline - digit_h - digit_h,
            self.width / 2 + half,
            baseline + digit_h,
        ))
    }

    fn set_clock(&mut self, day_seconds: u32) {
        self.clock.set(day_seconds);
    }

    fn update(&mut self, dt: f32, _input: &AppInput<'_>) {
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
