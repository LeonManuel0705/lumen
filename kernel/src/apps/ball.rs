use crate::anim::Spring;
use crate::gfx::{shapes, Color, Rect, Surface};

use super::{App, AppInput};

const TRAIL_LEN: usize = 22;
const GRAVITY: f32 = 1800.0;
const RADIUS: f32 = 24.0;
const WALL_DAMPING: f32 = 0.88;
const FLOOR_DAMPING: f32 = 0.84;

/// The bouncing ball that has been Lumen's animation testbed since phase two,
/// now living inside a window instead of owning the whole screen.
pub struct BallApp {
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    squash: Spring,
    pulse: Spring,
    trail: [(f32, f32); TRAIL_LEN],
    trail_idx: usize,
}

impl BallApp {
    pub fn new(width: usize, height: usize) -> Self {
        let w = width as f32;
        let start = (w * 0.5, RADIUS + 8.0);
        Self {
            width: w,
            height: height as f32,
            x: start.0,
            y: start.1,
            vx: 220.0,
            vy: 0.0,
            squash: Spring::new(0.0, 220.0, 14.0),
            pulse: Spring::new(1.0, 120.0, 12.0),
            trail: [start; TRAIL_LEN],
            trail_idx: 0,
        }
    }

    /// Everything the ball puts on the surface: the trail, the ball itself,
    /// and the shadow on the floor below it, generously inflated.
    fn ink(&self) -> Rect {
        let mut r = Rect::new(i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for (tx, ty) in self.trail.iter() {
            r.x0 = r.x0.min(*tx as i32);
            r.y0 = r.y0.min(*ty as i32);
            r.x1 = r.x1.max(*tx as i32);
            r.y1 = r.y1.max(*ty as i32);
        }
        r.x0 = r.x0.min(self.x as i32);
        r.y0 = r.y0.min(self.y as i32);
        r.x1 = r.x1.max(self.x as i32);
        r.y1 = r.y1.max(self.y as i32);
        // The widest thing drawn is the outer glow at 1.18x the radius, times
        // the pulse spring's ceiling, plus a pixel of antialiasing.
        let pad = (RADIUS * 1.18 * 1.4) as i32 + 3;
        let r = r.expand(pad);
        // The shadow only reaches as wide as the ball itself, on the floor.
        let shadow_half = (RADIUS * 1.3) as i32 + 4;
        let floor = Rect::new(
            self.x as i32 - shadow_half,
            self.height as i32 - (RADIUS as i32),
            self.x as i32 + shadow_half,
            self.height as i32,
        );
        r.union(&floor)
    }

    fn reset(&mut self) {
        self.x = self.width * 0.5;
        self.y = RADIUS + 8.0;
        self.vx = 220.0;
        self.vy = 0.0;
        self.trail = [(self.x, self.y); TRAIL_LEN];
    }
}

impl App for BallApp {
    fn title(&self) -> &'static str {
        "Ball"
    }

    fn painted(&self) -> Option<Rect> {
        Some(self.ink())
    }

    fn update(&mut self, dt: f32, input: &AppInput) {
        if input.focused && input.reset {
            self.reset();
        }

        if input.clicked {
            // Throw the ball away from the click, hardest when it lands on it.
            let dx = input.cursor.0 - self.x;
            let dy = input.cursor.1 - self.y;
            let len_sq = dx * dx + dy * dy;
            if len_sq < (RADIUS * 2.2) * (RADIUS * 2.2) {
                let scale = if len_sq > 4.0 { 1.0 / RADIUS } else { 0.0 };
                self.vx = -dx * scale * 620.0;
                self.vy = -dy * scale * 620.0 - 260.0;
                self.squash.nudge(-15.0);
                self.pulse.nudge(-3.0);
            }
        }

        if input.focused && input.space {
            self.vy = -640.0;
            self.vx += crate::rng::sign() * 80.0;
            self.squash.nudge(-10.0);
            self.pulse.nudge(-2.0);
        }

        self.vy += GRAVITY * dt;
        self.x += self.vx * dt;
        self.y += self.vy * dt;

        if self.x - RADIUS < 0.0 {
            self.x = RADIUS;
            self.vx = self.vx.abs() * WALL_DAMPING;
            self.squash.nudge(8.0);
            self.pulse.nudge(-2.0);
        }
        if self.x + RADIUS > self.width {
            self.x = self.width - RADIUS;
            self.vx = -self.vx.abs() * WALL_DAMPING;
            self.squash.nudge(8.0);
            self.pulse.nudge(-2.0);
        }
        if self.y + RADIUS > self.height {
            self.y = self.height - RADIUS;
            if self.vy.abs() < 60.0 {
                // Never let it settle: a still ball is a boring window.
                self.vy = -460.0;
                self.vx += crate::rng::sign() * 80.0;
            } else {
                self.vy = -self.vy.abs() * FLOOR_DAMPING;
            }
            self.squash.nudge(-12.0);
            self.pulse.nudge(-4.0);
        }
        if self.y - RADIUS < 0.0 {
            self.y = RADIUS;
            self.vy = self.vy.abs() * 0.7;
            self.squash.nudge(-6.0);
        }

        self.squash.set_target(0.0);
        self.squash.step(dt);
        self.pulse.set_target(1.0);
        self.pulse.step(dt);

        self.trail[self.trail_idx] = (self.x, self.y);
        self.trail_idx = (self.trail_idx + 1) % TRAIL_LEN;
    }

    fn draw(&self, surface: &mut Surface) {
        // A hint of a room: brighter at the floor, with a line to bounce on.
        shapes::vertical_gradient(
            surface,
            Color::rgba(255, 255, 255, 16),
            Color::rgba(255, 255, 255, 34),
            Color::rgba(255, 255, 255, 62),
        );
        let floor_y = self.height as i32;
        shapes::fill_rect(surface, 0, floor_y - 2, self.width as i32, 2, Color::rgba(255, 255, 255, 150));

        for i in 0..TRAIL_LEN {
            let idx = (self.trail_idx + i) % TRAIL_LEN;
            let age = i as f32 / TRAIL_LEN as f32;
            let alpha = (age * age * 90.0) as u8;
            if alpha < 8 {
                continue;
            }
            let (tx, ty) = self.trail[idx];
            let r = (RADIUS * (0.4 + age * 0.5)) as i32;
            shapes::fill_circle(surface, tx as i32, ty as i32, r, Color::LUMEN_ACCENT.with_alpha(alpha));
        }

        let above = (floor_y as f32 - self.y - RADIUS).max(0.0);
        let shadow_t = (1.0 - (above / 260.0).min(1.0)).max(0.1);
        shapes::fill_ellipse(
            surface,
            self.x as i32,
            floor_y - 3,
            (RADIUS * (0.6 + shadow_t * 0.7)) as i32,
            ((RADIUS * 0.18 * (0.5 + shadow_t)) as i32).max(2),
            Color::rgba(20, 50, 90, (80.0 * shadow_t) as u8),
        );

        let stretch = self.squash.current.clamp(-18.0, 18.0) * 0.012;
        let pulse = self.pulse.current.clamp(0.7, 1.4);
        let rx = (RADIUS * (1.0 - stretch) * pulse) as i32;
        let ry = (RADIUS * (1.0 + stretch) * pulse) as i32;
        let (cx, cy) = (self.x as i32, self.y as i32);

        shapes::fill_ellipse(surface, cx, cy, (rx as f32 * 1.18) as i32, (ry as f32 * 1.18) as i32, Color::LUMEN_ACCENT.with_alpha(80));
        shapes::fill_ellipse(surface, cx, cy, rx, ry, Color::LUMEN_ACCENT);
        shapes::fill_ellipse(surface, cx - rx / 3, cy - ry / 3, rx / 3, ry / 3, Color::rgba(255, 255, 255, 200));
    }
}
