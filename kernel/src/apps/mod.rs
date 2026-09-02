pub mod ball;
pub mod clock;

use alloc::boxed::Box;

use crate::gfx::{Rect, Surface};
use crate::input::KeyBatch;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AppKind {
    Ball,
    Clock,
}

pub struct AppInput<'a> {
    pub cursor: (f32, f32),
    pub clicked: bool,
    pub keys: &'a KeyBatch,
}

pub trait App {
    fn title(&self) -> &'static str;
    fn update(&mut self, dt: f32, input: &AppInput<'_>);

    fn painted(&self) -> Option<Rect>;

    fn set_clock(&mut self, _day_seconds: u32) {}
    fn draw(&self, surface: &mut Surface);
}

impl AppKind {
    pub fn content_size(self) -> (usize, usize) {
        match self {
            AppKind::Ball => (460, 300),
            AppKind::Clock => (300, 130),
        }
    }

    pub fn spawn(self, width: usize, height: usize) -> Box<dyn App> {
        match self {
            AppKind::Ball => Box::new(ball::BallApp::new(width, height)),
            AppKind::Clock => Box::new(clock::ClockApp::new(width, height)),
        }
    }
}
