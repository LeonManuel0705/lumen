pub mod ball;
pub mod clock;

use alloc::boxed::Box;

use crate::gfx::Surface;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AppKind {
    Ball,
    Clock,
}

/// What an app is told about the world each frame. Coordinates are local to the
/// app's own surface, so an app never learns where its window sits.
pub struct AppInput {
    pub cursor: (f32, f32),
    pub clicked: bool,
    pub focused: bool,
    pub space: bool,
    pub reset: bool,
}

pub trait App {
    fn title(&self) -> &'static str;
    fn update(&mut self, dt: f32, input: &AppInput);
    /// Apps that show wall time get it from the shell, which owns the RTC.
    fn set_clock(&mut self, _day_seconds: u32) {}
    fn draw(&self, surface: &mut Surface);
}

impl AppKind {
    /// The size an app wants its content area to be.
    pub fn content_size(self) -> (usize, usize) {
        match self {
            AppKind::Ball => (460, 300),
            AppKind::Clock => (300, 130),
        }
    }

    pub fn spawn(self, width: usize, height: usize) -> Box<dyn App> {
        match self {
            AppKind::Ball => Box::new(ball::BallApp::new(width, height)),
            AppKind::Clock => Box::new(clock::ClockApp::new()),
        }
    }
}
