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

/// What an app is told about the world each frame. Coordinates are local to the
/// app's own surface, so an app never learns where its window sits, and the
/// keystrokes are only the ones the shell did not claim for itself. An
/// unfocused window is handed an empty batch and never sees a click it did not
/// get, so an app never has to ask whether the input was meant for it.
pub struct AppInput<'a> {
    pub cursor: (f32, f32),
    pub clicked: bool,
    pub keys: &'a KeyBatch,
}

pub trait App {
    fn title(&self) -> &'static str;
    fn update(&mut self, dt: f32, input: &AppInput<'_>);

    /// The region this app will paint into its surface this frame, in surface
    /// coordinates, or `None` if the surface is unchanged since it was last
    /// drawn. The compositor remembers where the app painted last time and
    /// repaints both, so an app only has to describe the present.
    ///
    /// Reporting too small a rectangle leaves a stale smear on screen, so when
    /// in doubt an app should claim more than it touches.
    fn painted(&self) -> Option<Rect>;

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
            AppKind::Clock => Box::new(clock::ClockApp::new(width, height)),
        }
    }
}
