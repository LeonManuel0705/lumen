use crate::anim::{easing, Spring, Tween};
use crate::apps::AppKind;
use crate::display::Frame;
use crate::gfx::{font_data, shapes, Canvas, Color, Damage, Framebuffer, Rect};
use crate::input::{Key, KeyBatch, Snapshot};
use crate::widgets::{self, RollingClock};
use crate::wm::WindowManager;

#[derive(PartialEq, Copy, Clone)]
enum ShellMode {
    Locked,
    Unlocking,
    Desktop,
}

const FLOOR_FROM_BOTTOM: f32 = 110.0;
const MAX_RIPPLES: usize = 8;
const DOCK_ICONS: usize = 6;
/// Matches the reach of the shadow the shapes module draws.
/// How far the lock screen's entrance lifts its contents.
const LOCK_RISE: i32 = 26;
const DOCK_APPS: [Option<AppKind>; DOCK_ICONS] = [
    Some(AppKind::Ball),
    Some(AppKind::Clock),
    None,
    None,
    None,
    None,
];
const DOCK_TINTS: [Color; DOCK_ICONS] = [
    Color::LUMEN_ACCENT,
    Color::LUMEN_GLOW,
    Color::rgb(255, 150, 180),
    Color::rgb(150, 230, 200),
    Color::rgb(200, 180, 255),
    Color::LUMEN_INK,
];
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
    clock_phase: f32,
    sun_x: f32,
    sun_y: f32,
    cursor_target_x: f32,
    cursor_target_y: f32,
    cursor_x: Spring,
    cursor_y: Spring,
    ripples: [Ripple; MAX_RIPPLES],
    next_ripple: usize,
    mode: ShellMode,
    top_clock: RollingClock,
    big_clock: RollingClock,
    clock_entrance: Tween,
    lock_entrance: Tween,
    unlock: Tween,
    date_buf: [u8; 48],
    date_len: usize,
    windows: WindowManager,
    damage: Damage,
    /// Where the cursor was drawn last, so a still cursor costs nothing.
    last_cursor: (i32, i32),
    /// Set when the date string changes, which is otherwise pixel-identical
    /// from one frame to the next.
    date_dirty: bool,
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
            clock_phase: 0.0,
            sun_x: w - w * 0.18,
            sun_y: h * 0.22,
            cursor_target_x: mid_x,
            cursor_target_y: mid_y,
            cursor_x: Spring::new(mid_x, 8000.0, 200.0),
            cursor_y: Spring::new(mid_y, 8000.0, 200.0),
            ripples: [Ripple { x: 0.0, y: 0.0, age: 0.0, alive: false }; MAX_RIPPLES],
            next_ripple: 0,
            mode: ShellMode::Locked,
            top_clock: RollingClock::new(&font_data::FONT_CLOCK, true),
            big_clock: RollingClock::new(&font_data::FONT_BIG, false),
            clock_entrance: Tween::new(0.7, 0.35),
            lock_entrance: Tween::new(0.8, 0.3),
            unlock: Tween::new(0.55, 0.0),
            date_buf: [0; 48],
            date_len: 0,
            windows: WindowManager::new(w, h),
            damage: Damage::new(width as i32, height as i32),
            last_cursor: (mid_x as i32, mid_y as i32),
            date_dirty: true,
        }
    }

    /// Closes out a frame: draws the app surfaces once, works out what changed,
    /// and hands the regions over. Simulation may have stepped several times to
    /// get here, but only the state it ended on is ever drawn.
    pub fn finish_frame(&mut self) -> Damage {
        if self.mode != ShellMode::Locked {
            self.windows.render_surfaces();
        }
        self.collect_damage();
        let out = self.damage;
        self.damage.clear();
        out
    }

    pub fn repaint_everything(&mut self) {
        self.damage.mark_all();
    }

    fn cursor_rect_at(at: (i32, i32)) -> Rect {
        Rect::from_size(at.0 - 24, at.1 - 24, 48, 48)
    }

    /// The pixels the top bar's clock can occupy, including the vertical room
    /// the digit roll needs above and below the baseline.
    fn top_clock_rect(&self) -> Rect {
        let (bx, by, bw, bh) = self.top_bar_rect();
        let baseline = by + (bh + self.top_clock.digit_height()) / 2;
        self.top_clock.changed_rect(bx + bw / 2, baseline)
    }

    /// The lock screen's three moving parts. The entrance lifts everything by
    /// up to LOCK_RISE, so every band has to cover its whole travel or the
    /// bottom of it smears on the way in.
    fn lock_rects(&self) -> (Rect, Rect, Rect) {
        let cx = (self.width * 0.5) as i32;
        let baseline = (self.height * 0.382) as i32;
        let clock = self
            .big_clock
            .changed_rect(cx, baseline)
            .union(&self.big_clock.changed_rect(cx, baseline + LOCK_RISE));

        let font = &font_data::FONT_UI;
        let date_half = self.date_len as i32 * 12 + 24;
        let date_top = baseline + 66 - font.ascent as i32 - 4;
        let date = Rect::new(
            cx - date_half,
            date_top,
            cx + date_half,
            baseline + LOCK_RISE + 66 - font.descent as i32 + 4,
        );

        let hint_y = (self.height - 110.0) as i32;
        let hint = Rect::new(cx - 300, hint_y - 32, cx + 300, hint_y + 16);
        (clock, date, hint)
    }

    pub fn set_clock(&mut self, day_seconds: u32) {
        self.top_clock.set(day_seconds);
        self.big_clock.set(day_seconds);
        self.windows.set_clock(day_seconds);
    }

    pub fn set_date(&mut self, bytes: &[u8]) {
        let n = bytes.len().min(self.date_buf.len());
        if self.date_len != n || self.date_buf[..n] != bytes[..n] {
            self.date_dirty = true;
        }
        self.date_buf[..n].copy_from_slice(&bytes[..n]);
        self.date_len = n;
    }

    fn begin_unlock(&mut self) {
        self.mode = ShellMode::Unlocking;
        self.unlock.restart();
        self.clock_entrance.restart();
        // The desktop arrives with something on it, one window after the other.
        self.windows.open(AppKind::Ball, self.width * 0.19, self.height * 0.28, 0.24);
        self.windows.open(AppKind::Clock, self.width * 0.63, self.height * 0.34, 0.44);
    }

    fn dock_icon_rect(&self, i: usize) -> (i32, i32, i32, i32) {
        let (dx, dy, dw, dh) = self.dock_rect();
        let size = 52;
        let gap = (dw - size * DOCK_ICONS as i32 - 24) / (DOCK_ICONS as i32 - 1).max(1);
        (dx + 12 + i as i32 * (size + gap), dy + (dh - size) / 2, size, size)
    }

    /// True when a point is on the shell's own chrome, which sits above every
    /// window and therefore swallows the press.
    fn chrome_hit(&self, px: f32, py: f32) -> bool {
        let inside = |(x, y, w, h): (i32, i32, i32, i32)| {
            px >= x as f32 && px < (x + w) as f32 && py >= y as f32 && py < (y + h) as f32
        };
        inside(self.top_bar_rect()) || inside(self.dock_rect())
    }

    /// Takes the keystrokes the shell claims for itself and hands back the
    /// rest for the focused app. A shortcut the shell acts on never also
    /// reaches an app, which is what keeps one key from meaning two things.
    fn handle_shortcuts(&mut self, keys: &KeyBatch) -> KeyBatch {
        let mut rest = KeyBatch::EMPTY;
        for event in keys.iter() {
            if !event.pressed {
                rest.push(*event);
                continue;
            }
            match event.key {
                Key::Escape => self.windows.close_focused(),
                Key::Char('w') if event.mods.ctrl => self.windows.close_focused(),
                Key::Tab => self.windows.cycle_focus(),
                Key::Char(c) if ('1'..='6').contains(&c) => {
                    let slot = c as usize - '1' as usize;
                    if let Some(kind) = DOCK_APPS[slot] {
                        self.windows
                            .open(kind, self.width * 0.32, self.height * 0.3, 0.0);
                    }
                }
                _ => rest.push(*event),
            }
        }
        rest
    }

    fn dock_hit(&self, px: f32, py: f32) -> Option<AppKind> {
        for i in 0..DOCK_ICONS {
            let (x, y, w, h) = self.dock_icon_rect(i);
            // Generous vertical slack: the icons are small targets.
            if px >= x as f32 && px < (x + w) as f32 && py >= (y - 6) as f32 && py < (y + h + 10) as f32 {
                return DOCK_APPS[i];
            }
        }
        None
    }

    pub fn update(&mut self, dt: f32, input: &Snapshot) {
        self.clock_phase += dt;
        if self.clock_phase >= 1.0 {
            self.clock_phase -= 1.0;
        }

        self.cursor_target_x = (self.cursor_target_x + input.mouse_dx as f32).clamp(0.0, self.width - 1.0);
        self.cursor_target_y = (self.cursor_target_y + input.mouse_dy as f32).clamp(0.0, self.height - 1.0);
        self.cursor_x.set_target(self.cursor_target_x);
        self.cursor_y.set_target(self.cursor_target_y);
        self.cursor_x.step(dt);
        self.cursor_y.step(dt);

        let click = input.buttons_just_pressed & 0x01 != 0;
        let held = input.buttons & 0x01 != 0;
        let cursor = (self.cursor_x.current, self.cursor_y.current);

        if self.mode == ShellMode::Locked {
            if click {
                self.spawn_ripple(cursor.0, cursor.1);
            }
            if input.keys.iter().any(|e| e.pressed) || click {
                self.begin_unlock();
            }
        } else {
            if click {
                self.spawn_ripple(cursor.0, cursor.1);
            }
            let mut taken = false;
            if click && self.mode == ShellMode::Desktop {
                if let Some(kind) = self.dock_hit(cursor.0, cursor.1) {
                    self.windows.open(kind, self.width * 0.32, self.height * 0.3, 0.0);
                    taken = true;
                } else if self.chrome_hit(cursor.0, cursor.1) {
                    // The glass is a surface, not a hole: a press that lands on
                    // the dock or the top bar stops there.
                    taken = true;
                }
            }
            let for_apps = self.handle_shortcuts(&input.keys);
            self.windows.update(dt, cursor, click && !taken, held, &for_apps);
        }

        self.clock_entrance.step(dt);
        if self.mode == ShellMode::Locked {
            self.lock_entrance.step(dt);
        }
        if self.mode == ShellMode::Unlocking {
            self.unlock.step(dt);
            if self.unlock.is_done() {
                self.mode = ShellMode::Desktop;
            }
        }
        self.top_clock.update(dt);
        self.big_clock.update(dt);

        for r in self.ripples.iter_mut() {
            if !r.alive { continue; }
            r.age += dt;
            if r.age > RIPPLE_LIFETIME { r.alive = false; }
        }
    }

    fn collect_damage(&mut self) {
        // A cursor that has not moved is already on screen where it belongs.
        // One that has must claim where it was as well as where it is: anything
        // that can stay still for a frame has to carry its own history, because
        // the frame-to-frame union only covers what was repainted last frame.
        let now = (self.cursor_x.current as i32, self.cursor_y.current as i32);
        if now != self.last_cursor {
            self.damage.add(Self::cursor_rect_at(self.last_cursor));
            self.damage.add(Self::cursor_rect_at(now));
            self.last_cursor = now;
        }

        for r in &self.ripples {
            if !r.alive {
                continue;
            }
            let radius = (16.0 + (r.age / RIPPLE_LIFETIME) * 180.0) as i32 + 6;
            self.damage.add(Rect::from_size(
                r.x as i32 - radius,
                r.y as i32 - radius,
                radius * 2,
                radius * 2,
            ));
        }

        match self.mode {
            // Everything is in motion during the unlock, and the whole lock UI
            // slides at once. Not worth being clever about.
            ShellMode::Unlocking => self.damage.mark_all(),
            ShellMode::Locked => {
                let (clock, date, hint) = self.lock_rects();
                self.damage.add(clock);
                // The hint breathes every frame; the date only moves while the
                // lock screen is arriving, or when the day rolls over.
                self.damage.add(hint);
                if self.lock_entrance.t() < 1.0 || self.date_dirty {
                    self.damage.add(date);
                    self.date_dirty = false;
                }
            }
            ShellMode::Desktop => {
                if self.clock_entrance.t() < 1.0 {
                    // The bar is still settling, so all of it moves.
                    let (bx, by, bw, bh) = self.top_bar_rect();
                    self.damage
                        .add(Rect::from_size(bx, by, bw, bh).union(&shapes::shadow_bounds(bx, by, bw, bh)));
                } else {
                    // Afterwards only the digits roll and the colon breathes.
                    self.damage.add(self.top_clock_rect());
                }
                self.windows.damage_into(&mut self.damage);
            }
        }
        // Windows keep animating behind the lock screen and while it lifts, so
        // their damage is collected in every mode, not only on the desktop.
        if self.mode != ShellMode::Desktop {
            self.windows.forget_damage();
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

    /// How far the shell chrome has arrived: 0 while locked, 1 on the desktop.
    fn chrome_t(&self) -> f32 {
        match self.mode {
            ShellMode::Locked => 0.0,
            ShellMode::Unlocking => easing::ease_out_quad(self.unlock.t()),
            ShellMode::Desktop => 1.0,
        }
    }

    fn draw_chrome(&self, frame: &mut Frame) {
        let t = self.chrome_t();
        if t <= 0.01 {
            return;
        }
        let alpha = |a: u8| (a as f32 * t) as u8;
        let clip = frame.clip();

        let (bx, by, bw, bh) = self.top_bar_rect();
        let bar_lift = ((1.0 - t) * 22.0) as i32;
        if Rect::from_size(bx, by - bar_lift, bw, bh)
            .union(&shapes::shadow_bounds(bx, by - bar_lift, bw, bh))
            .intersects(&clip)
        {
            shapes::drop_shadow(frame, bx, by - bar_lift, bw, bh, 22);
            frame.glass_fill(bx, by - bar_lift, bw, bh, 22, Color::LUMEN_CARD.with_alpha(130), alpha(255));
            shapes::glass_highlights(frame, bx, by - bar_lift, bw, bh, 22, alpha(255));
        }

        let (dx, dy, dw, dh) = self.dock_rect();
        let dock_lift = ((1.0 - t) * 40.0) as i32;
        if !Rect::from_size(dx, dy + dock_lift, dw, dh)
            .union(&shapes::shadow_bounds(dx, dy + dock_lift, dw, dh))
            .intersects(&clip)
        {
            return;
        }
        shapes::drop_shadow(frame, dx, dy + dock_lift, dw, dh, 26);
        frame.glass_fill(dx, dy + dock_lift, dw, dh, 26, Color::LUMEN_CARD.with_alpha(130), alpha(255));
        shapes::glass_highlights(frame, dx, dy + dock_lift, dw, dh, 26, alpha(255));

        let icon_size = 52;
        let icon_gap = (dw - icon_size * DOCK_ICONS as i32 - 24) / (DOCK_ICONS as i32 - 1).max(1);
        let icon_y = dy + dock_lift + (dh - icon_size) / 2;
        for (i, tint) in DOCK_TINTS.iter().enumerate() {
            // Each icon lands a beat after the one before it.
            let start = 0.20 + i as f32 * 0.07;
            let local = ((t - start) / 0.45).clamp(0.0, 1.0);
            if local <= 0.0 {
                continue;
            }
            let pop = easing::ease_out_back(local);
            let size = (icon_size as f32 * (0.55 + 0.45 * pop)) as i32;
            let ix = dx + 12 + i as i32 * (icon_size + icon_gap) + (icon_size - size) / 2;
            let iy = icon_y + (icon_size - size) / 2;
            let a = (local * 255.0) as u8;
            shapes::fill_rounded_rect(frame, ix, iy, size, size, size * 27 / 100, tint.with_alpha(a));
            shapes::fill_rect(frame, ix + 8, icon_y + icon_size + 4, size - 16, 2, Color::WHITE.with_alpha(a / 2));
        }
    }

    fn draw_top_clock<C: Canvas>(&self, fb: &mut C) {
        let t_in = self.clock_entrance.t();
        if t_in <= 0.0 {
            return;
        }
        let (bx, by, bw, bh) = self.top_bar_rect();
        let master = easing::ease_out_quad(t_in);
        let rise = ((1.0 - easing::ease_out_back(t_in)) * 14.0) as i32;
        let baseline = by + (bh + self.top_clock.digit_height()) / 2 + rise;
        let pulse = 4.0 * self.clock_phase * (1.0 - self.clock_phase);
        self.top_clock.draw(fb, bx + bw / 2, baseline, master, pulse);
    }

    fn draw_lock<C: Canvas>(&self, fb: &mut C) {
        let t_in = self.lock_entrance.t();
        let t_out = self.unlock.t();
        let master = easing::ease_out_quad(t_in) * (1.0 - t_out);
        if master <= 0.01 {
            return;
        }
        let rise = ((1.0 - easing::ease_out_back(t_in)) * 24.0) as i32;
        let slide = (easing::ease_in_quad(t_out) * self.height * 0.45) as i32;
        let cx = (self.width * 0.5) as i32;
        let big_baseline = (self.height * 0.382) as i32 + rise - slide;
        let pulse = 4.0 * self.clock_phase * (1.0 - self.clock_phase);

        self.big_clock.draw(fb, cx, big_baseline, master, pulse);

        if self.date_len > 0 {
            if let Ok(date) = core::str::from_utf8(&self.date_buf[..self.date_len]) {
                widgets::text_embossed(fb, &font_data::FONT_UI, date, cx, big_baseline + 66, (master * 220.0) as u8);
            }
        }

        let hint_alpha = master * (110.0 + 70.0 * pulse);
        let hint_y = (self.height - 110.0) as i32 - slide / 3;
        widgets::text_embossed(fb, &font_data::FONT_UI, "Leertaste oder Klick zum Entsperren", cx, hint_y, hint_alpha as u8);
    }

    pub fn draw(&self, fb: &mut Frame) {
        if self.mode != ShellMode::Locked {
            self.windows.draw(fb);
        }
        self.draw_chrome(fb);
        if self.mode != ShellMode::Locked {
            self.draw_top_clock(fb);
        }

        let clip = fb.clip();
        for r in &self.ripples {
            if !r.alive { continue; }
            let t = r.age / RIPPLE_LIFETIME;
            let radius = (16.0 + t * 180.0) as i32;
            let alpha = ((1.0 - t) * (1.0 - t) * 200.0) as u8;
            if alpha < 6 { continue; }
            // A ripple 400 px across still costs thousands of coverage tests
            // when it is nowhere near the region being repainted.
            if !Rect::from_size(r.x as i32 - radius - 4, r.y as i32 - radius - 4, radius * 2 + 8, radius * 2 + 8)
                .intersects(&clip)
            {
                continue;
            }
            shapes::stroke_circle(fb, r.x as i32, r.y as i32, radius, 3, Color::WHITE.with_alpha(alpha));
            shapes::stroke_circle(fb, r.x as i32, r.y as i32, (radius - 8).max(1), 2, Color::LUMEN_ACCENT.with_alpha(alpha / 2));
        }

        if self.mode != ShellMode::Desktop {
            self.draw_lock(fb);
        }

        let cx = self.cursor_x.current as i32;
        let cy = self.cursor_y.current as i32;
        shapes::fill_circle(fb, cx, cy, 22, Color::LUMEN_ACCENT.with_alpha(35));
        shapes::fill_circle(fb, cx, cy, 14, Color::LUMEN_ACCENT.with_alpha(110));
        shapes::fill_circle(fb, cx, cy, 7,  Color::WHITE.with_alpha(220));
        shapes::fill_circle(fb, cx, cy, 3,  Color::WHITE);
    }

}
