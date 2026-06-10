use crate::anim::Spring;
use crate::gfx::{shapes, Color, Framebuffer};
use crate::input::Snapshot;

const TRAIL_LEN: usize = 22;
const FLOOR_FROM_BOTTOM: f32 = 110.0;
const GRAVITY: f32 = 1800.0;
const BALL_RADIUS: f32 = 38.0;
const MAX_RIPPLES: usize = 8;
const RIPPLE_LIFETIME: f32 = 0.95;

#[derive(Copy, Clone)]
struct Ripple {
    x: f32,
    y: f32,
    age: f32,
    alive: bool,
}

pub struct Scene {
    width: f32,
    height: f32,
    elapsed: f32,
    ball_x: f32,
    ball_y: f32,
    vx: f32,
    vy: f32,
    squash: Spring,
    trail: [(f32, f32); TRAIL_LEN],
    trail_idx: usize,
    pulse: Spring,
    sun_x: f32,
    sun_y: f32,
    cursor_target_x: f32,
    cursor_target_y: f32,
    cursor_x: Spring,
    cursor_y: Spring,
    ripples: [Ripple; MAX_RIPPLES],
    next_ripple: usize,
}

impl Scene {
    pub fn new(width: usize, height: usize) -> Self {
        let w = width as f32;
        let h = height as f32;
        let mid_x = w * 0.5;
        let mid_y = h * 0.5;
        Self {
            width: w,
            height: h,
            elapsed: 0.0,
            ball_x: mid_x,
            ball_y: BALL_RADIUS + 10.0,
            vx: 220.0,
            vy: 0.0,
            squash: Spring::new(0.0, 220.0, 14.0),
            trail: [(mid_x, BALL_RADIUS + 10.0); TRAIL_LEN],
            trail_idx: 0,
            pulse: Spring::new(1.0, 120.0, 12.0),
            sun_x: w - w * 0.18,
            sun_y: h * 0.22,
            cursor_target_x: mid_x,
            cursor_target_y: mid_y,
            cursor_x: Spring::new(mid_x, 8000.0, 200.0),
            cursor_y: Spring::new(mid_y, 8000.0, 200.0),
            ripples: [Ripple { x: 0.0, y: 0.0, age: 0.0, alive: false }; MAX_RIPPLES],
            next_ripple: 0,
        }
    }

    pub fn update(&mut self, dt: f32, input: &Snapshot) {
        self.elapsed += dt;

        self.cursor_target_x = (self.cursor_target_x + input.mouse_dx as f32).clamp(0.0, self.width - 1.0);
        self.cursor_target_y = (self.cursor_target_y + input.mouse_dy as f32).clamp(0.0, self.height - 1.0);
        self.cursor_x.set_target(self.cursor_target_x);
        self.cursor_y.set_target(self.cursor_target_y);
        self.cursor_x.step(dt);
        self.cursor_y.step(dt);

        if input.buttons_just_pressed & 0x01 != 0 {
            let click_x = self.cursor_x.current;
            let click_y = self.cursor_y.current;
            self.spawn_ripple(click_x, click_y);
            let dx = click_x - self.ball_x;
            let dy = click_y - self.ball_y;
            let len_sq = dx * dx + dy * dy;
            if len_sq < (BALL_RADIUS * 1.4) * (BALL_RADIUS * 1.4) {
                let scale = if len_sq > 4.0 { 1.0 / BALL_RADIUS } else { 0.0 };
                self.vx = -dx * scale * 760.0;
                self.vy = -dy * scale * 760.0 - 320.0;
                self.squash.nudge(-15.0);
                self.pulse.nudge(-3.0);
            }
        }

        if input.key_pressed_space {
            self.vy = -780.0;
            self.vx += crate::rng::sign() * 80.0;
            self.squash.nudge(-10.0);
            self.pulse.nudge(-2.0);
        }

        if input.key_pressed_r {
            self.ball_x = self.width * 0.5;
            self.ball_y = BALL_RADIUS + 10.0;
            self.vx = 220.0;
            self.vy = 0.0;
            self.trail = [(self.ball_x, self.ball_y); TRAIL_LEN];
        }

        self.vy += GRAVITY * dt;
        self.ball_x += self.vx * dt;
        self.ball_y += self.vy * dt;

        let floor_y = self.height - FLOOR_FROM_BOTTOM;

        if self.ball_x - BALL_RADIUS < 0.0 {
            self.ball_x = BALL_RADIUS;
            self.vx = self.vx.abs() * 0.88;
            self.squash.nudge(8.0);
            self.pulse.nudge(-2.0);
        }
        if self.ball_x + BALL_RADIUS > self.width {
            self.ball_x = self.width - BALL_RADIUS;
            self.vx = -self.vx.abs() * 0.88;
            self.squash.nudge(8.0);
            self.pulse.nudge(-2.0);
        }
        if self.ball_y + BALL_RADIUS > floor_y {
            self.ball_y = floor_y - BALL_RADIUS;
            if self.vy.abs() < 60.0 {
                self.vy = -560.0;
                self.vx += crate::rng::sign() * 80.0;
            } else {
                self.vy = -self.vy.abs() * 0.84;
            }
            self.squash.nudge(-12.0);
            self.pulse.nudge(-4.0);
        }
        if self.ball_y - BALL_RADIUS < 0.0 {
            self.ball_y = BALL_RADIUS;
            self.vy = self.vy.abs() * 0.7;
            self.squash.nudge(-6.0);
        }

        self.squash.set_target(0.0);
        self.squash.step(dt);
        self.pulse.set_target(1.0);
        self.pulse.step(dt);

        self.trail[self.trail_idx] = (self.ball_x, self.ball_y);
        self.trail_idx = (self.trail_idx + 1) % TRAIL_LEN;

        for r in self.ripples.iter_mut() {
            if !r.alive { continue; }
            r.age += dt;
            if r.age > RIPPLE_LIFETIME { r.alive = false; }
        }
    }

    fn spawn_ripple(&mut self, x: f32, y: f32) {
        self.ripples[self.next_ripple] = Ripple { x, y, age: 0.0, alive: true };
        self.next_ripple = (self.next_ripple + 1) % MAX_RIPPLES;
    }

    pub fn draw_background(&self, fb: &mut Framebuffer) {
        shapes::vertical_gradient(fb, Color::LUMEN_BG_TOP, Color::LUMEN_BG_MID, Color::LUMEN_BG_BOTTOM);

        shapes::radial_glow(fb, self.sun_x as i32, self.sun_y as i32, 240, Color::LUMEN_GLOW.with_alpha(160), 0);
        shapes::radial_glow(fb, self.sun_x as i32, self.sun_y as i32, 95,  Color::LUMEN_ACCENT.with_alpha(220), 0);

        let w = self.width as i32;
        let h = self.height as i32;
        let clouds = [
            (w * 1 / 8,  h * 3 / 16, 18),
            (w * 4 / 7,  h * 5 / 16, 14),
            (w * 2 / 5,  h * 1 / 7,  20),
            (w * 11 / 12, h * 8 / 15, 12),
            (w / 14,      h * 5 / 8,  16),
        ];
        for (cx, cy, scale) in clouds {
            shapes::cloud(fb, cx, cy, scale, Color::LUMEN_CLOUD.with_alpha(180));
        }

        let floor_y = (self.height - FLOOR_FROM_BOTTOM) as i32;
        shapes::fill_rect(fb, 0, floor_y, w, h - floor_y, Color::rgba(255, 255, 255, 70));
        shapes::fill_rect(fb, 0, floor_y, w, 1, Color::rgba(255, 255, 255, 200));

        let (bx, by, bw, bh) = self.top_bar_rect();
        crate::gfx::blur::box_blur_region(fb, bx - 8, by - 8, bw + 16, bh + 16, 14, 2);
        let (dx, dy, dw, dh) = self.dock_rect();
        crate::gfx::blur::box_blur_region(fb, dx - 8, dy - 8, dw + 16, dh + 16, 14, 2);

        shapes::glass_panel(fb, bx, by, bw, bh, 22, Color::LUMEN_CARD.with_alpha(130));
        shapes::glass_panel(fb, dx, dy, dw, dh, 26, Color::LUMEN_CARD.with_alpha(130));

        let icon_count = 6;
        let icon_size = 52;
        let icon_gap = (dw - icon_size * icon_count - 24) / (icon_count - 1).max(1);
        let icon_y = dy + (dh - icon_size) / 2;
        let icon_colors = [
            Color::LUMEN_ACCENT,
            Color::LUMEN_GLOW,
            Color::rgb(255, 150, 180),
            Color::rgb(150, 230, 200),
            Color::rgb(200, 180, 255),
            Color::LUMEN_INK,
        ];
        for i in 0..icon_count {
            let ix = dx + 12 + i * (icon_size + icon_gap);
            shapes::fill_rounded_rect(fb, ix, icon_y, icon_size, icon_size, 14, icon_colors[i as usize]);
            shapes::fill_rect(fb, ix + 8, icon_y + icon_size + 4, icon_size - 16, 2, Color::WHITE.with_alpha(120));
        }
    }

    fn top_bar_rect(&self) -> (i32, i32, i32, i32) {
        let w = self.width as i32;
        let bar_w = (self.width * 0.32) as i32;
        let bar_h = 56;
        ((w - bar_w) / 2, 18, bar_w, bar_h)
    }

    fn dock_rect(&self) -> (i32, i32, i32, i32) {
        let w = self.width as i32;
        let h = self.height as i32;
        let dock_w = (self.width * 0.42) as i32;
        let dock_h = 84;
        ((w - dock_w) / 2, h - dock_h - 22, dock_w, dock_h)
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        let floor_y = (self.height - FLOOR_FROM_BOTTOM) as i32;

        for i in 0..TRAIL_LEN {
            let idx = (self.trail_idx + i) % TRAIL_LEN;
            let age = i as f32 / TRAIL_LEN as f32;
            let alpha = (age * age * 90.0) as u8;
            if alpha < 8 { continue; }
            let (tx, ty) = self.trail[idx];
            let r = (BALL_RADIUS * (0.4 + age * 0.5)) as i32;
            shapes::fill_circle(fb, tx as i32, ty as i32, r, Color::LUMEN_ACCENT.with_alpha(alpha));
        }

        let height_above_floor = (floor_y as f32 - self.ball_y - BALL_RADIUS).max(0.0);
        let shadow_t = (1.0 - (height_above_floor / 400.0).min(1.0)).max(0.1);
        let shadow_rx = (BALL_RADIUS * (0.6 + shadow_t * 0.7)) as i32;
        let shadow_ry = (BALL_RADIUS * 0.18 * (0.5 + shadow_t)) as i32;
        let shadow_alpha = (90.0 * shadow_t) as u8;
        shapes::fill_ellipse(
            fb,
            self.ball_x as i32,
            floor_y - 4,
            shadow_rx,
            shadow_ry.max(2),
            Color::rgba(20, 50, 90, shadow_alpha),
        );

        let s = self.squash.current.clamp(-18.0, 18.0);
        let stretch = s * 0.012;
        let rx = (BALL_RADIUS * (1.0 - stretch)) as i32;
        let ry = (BALL_RADIUS * (1.0 + stretch)) as i32;
        let pulse = self.pulse.current.clamp(0.7, 1.4);
        let core_rx = ((rx as f32) * pulse) as i32;
        let core_ry = ((ry as f32) * pulse) as i32;

        shapes::fill_ellipse(
            fb,
            self.ball_x as i32,
            self.ball_y as i32,
            (core_rx as f32 * 1.18) as i32,
            (core_ry as f32 * 1.18) as i32,
            Color::LUMEN_ACCENT.with_alpha(80),
        );
        shapes::fill_ellipse(
            fb,
            self.ball_x as i32,
            self.ball_y as i32,
            core_rx,
            core_ry,
            Color::LUMEN_ACCENT,
        );
        shapes::fill_ellipse(
            fb,
            self.ball_x as i32 - core_rx / 3,
            self.ball_y as i32 - core_ry / 3,
            core_rx / 3,
            core_ry / 3,
            Color::rgba(255, 255, 255, 200),
        );

        for r in &self.ripples {
            if !r.alive { continue; }
            let t = r.age / RIPPLE_LIFETIME;
            let radius = (16.0 + t * 180.0) as i32;
            let alpha = ((1.0 - t) * (1.0 - t) * 200.0) as u8;
            if alpha < 6 { continue; }
            shapes::stroke_circle(fb, r.x as i32, r.y as i32, radius, 3, Color::WHITE.with_alpha(alpha));
            shapes::stroke_circle(fb, r.x as i32, r.y as i32, (radius - 8).max(1), 2, Color::LUMEN_ACCENT.with_alpha(alpha / 2));
        }

        let cx = self.cursor_x.current as i32;
        let cy = self.cursor_y.current as i32;
        shapes::fill_circle(fb, cx, cy, 22, Color::LUMEN_ACCENT.with_alpha(35));
        shapes::fill_circle(fb, cx, cy, 14, Color::LUMEN_ACCENT.with_alpha(110));
        shapes::fill_circle(fb, cx, cy, 7,  Color::WHITE.with_alpha(220));
        shapes::fill_circle(fb, cx, cy, 3,  Color::WHITE);
    }
}
