use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::anim::{easing, Spring, Tween};
use crate::apps::{App, AppInput, AppKind};
use crate::display::Frame;
use crate::gfx::{font_data, shapes, text, Canvas, Color, Surface};

const TITLE_BAR: i32 = 38;
const CORNER: i32 = 20;
const CONTENT_INSET: i32 = 8;
const OPEN_SECONDS: f32 = 0.42;
const CLOSE_SECONDS: f32 = 0.24;
const CLOSE_DOT: i32 = 11;

pub struct Window {
    id: u32,
    kind: AppKind,
    app: Box<dyn App>,
    surface: Surface,
    x: f32,
    y: f32,
    width: i32,
    height: i32,
    open: Tween,
    closing: Option<Tween>,
    /// 0 at rest, 1 while the window is being carried around.
    lift: Spring,
}

impl Window {
    /// Where the window sits right now, and how solid it is: entrance and exit
    /// both work by scaling around the centre and fading.
    fn presentation(&self) -> (f32, u8) {
        if let Some(close) = &self.closing {
            let t = close.t();
            let scale = 1.0 - 0.12 * easing::ease_in_quad(t);
            return (scale, ((1.0 - t) * 255.0) as u8);
        }
        let t = self.open.t();
        let scale = 0.86 + 0.14 * easing::ease_out_back(t);
        (scale, (easing::ease_out_quad(t) * 255.0) as u8)
    }

    fn frame_rect(&self) -> (i32, i32, i32, i32) {
        let (scale, _) = self.presentation();
        let lift = 1.0 + 0.02 * self.lift.current.clamp(0.0, 1.5);
        let scale = scale * lift;
        let w = (self.width as f32 * scale) as i32;
        let h = (self.height as f32 * scale) as i32;
        let cx = self.x + self.width as f32 * 0.5;
        let cy = self.y + self.height as f32 * 0.5;
        ((cx - w as f32 * 0.5) as i32, (cy - h as f32 * 0.5) as i32, w, h)
    }

    /// Hit areas are tested against the settled rectangle, not the animated
    /// one: a window that is still growing should not dodge the pointer.
    fn hit_rect(&self) -> (i32, i32, i32, i32) {
        (self.x as i32, self.y as i32, self.width, self.height)
    }

    fn contains(&self, px: f32, py: f32) -> bool {
        let (x, y, w, h) = self.hit_rect();
        px >= x as f32 && py >= y as f32 && px < (x + w) as f32 && py < (y + h) as f32
    }

    fn on_title_bar(&self, px: f32, py: f32) -> bool {
        self.contains(px, py) && py < self.y + TITLE_BAR as f32
    }

    fn close_dot(&self) -> (i32, i32) {
        let (x, y, _, _) = self.hit_rect();
        (x + 20, y + TITLE_BAR / 2)
    }

    fn on_close(&self, px: f32, py: f32) -> bool {
        let (cx, cy) = self.close_dot();
        let dx = px - cx as f32;
        let dy = py - cy as f32;
        dx * dx + dy * dy < ((CLOSE_DOT + 5) * (CLOSE_DOT + 5)) as f32
    }

    fn content_origin(&self) -> (f32, f32) {
        (
            self.x + CONTENT_INSET as f32,
            self.y + TITLE_BAR as f32,
        )
    }
}

pub struct WindowManager {
    /// Back to front: the last window is the focused one.
    windows: Vec<Window>,
    dragging: Option<(u32, f32, f32)>,
    next_id: u32,
    screen: (f32, f32),
}

impl WindowManager {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            windows: Vec::new(),
            dragging: None,
            next_id: 1,
            screen: (width, height),
        }
    }

    fn index_of_kind(&self, kind: AppKind) -> Option<usize> {
        self.windows
            .iter()
            .position(|w| w.kind == kind && w.closing.is_none())
    }

    /// Opens an app, or raises it if a window of that app is already up. The
    /// delay lets a boot sequence stagger its windows.
    pub fn open(&mut self, kind: AppKind, x: f32, y: f32, delay: f32) {
        if let Some(idx) = self.index_of_kind(kind) {
            let win = self.windows.remove(idx);
            self.windows.push(win);
            return;
        }

        let (cw, ch) = kind.content_size();
        let width = cw as i32 + CONTENT_INSET * 2;
        let height = ch as i32 + TITLE_BAR + CONTENT_INSET;
        let id = self.next_id;
        self.next_id += 1;

        self.windows.push(Window {
            id,
            kind,
            app: kind.spawn(cw, ch),
            surface: Surface::new(cw, ch),
            x: x.clamp(8.0, (self.screen.0 - width as f32 - 8.0).max(8.0)),
            y: y.clamp(80.0, (self.screen.1 - height as f32 - 8.0).max(80.0)),
            width,
            height,
            open: Tween::new(OPEN_SECONDS, delay),
            closing: None,
            lift: Spring::new(0.0, 190.0, 22.0),
        });
        crate::serial_println!("[wm] window {} opened ({}x{})", id, width, height);
    }

    pub fn set_clock(&mut self, day_seconds: u32) {
        for win in self.windows.iter_mut() {
            win.app.set_clock(day_seconds);
        }
    }

    /// Routes a press to the topmost window under the pointer: close button
    /// first, then the title bar to start a drag, and anything else goes to the
    /// app. Returns false when the press missed every window.
    fn press(&mut self, px: f32, py: f32) -> bool {
        let Some(idx) = self
            .windows
            .iter()
            .rposition(|w| w.closing.is_none() && w.contains(px, py))
        else {
            return false;
        };

        let win = self.windows.remove(idx);
        self.windows.push(win);
        let win = self.windows.last_mut().unwrap();

        if win.on_close(px, py) {
            win.closing = Some(Tween::new(CLOSE_SECONDS, 0.0));
            crate::serial_println!("[wm] window {} closing", win.id);
        } else if win.on_title_bar(px, py) {
            self.dragging = Some((win.id, px - win.x, py - win.y));
        }
        true
    }

    pub fn update(
        &mut self,
        dt: f32,
        cursor: (f32, f32),
        just_pressed: bool,
        held: bool,
        space: bool,
        reset: bool,
    ) -> bool {
        let mut consumed = false;
        if just_pressed {
            consumed = self.press(cursor.0, cursor.1);
        }
        if !held {
            self.dragging = None;
        }

        if let Some((id, grab_x, grab_y)) = self.dragging {
            let (sw, sh) = self.screen;
            if let Some(win) = self.windows.iter_mut().find(|w| w.id == id) {
                let max_x = (sw - win.width as f32 * 0.4).max(0.0);
                let max_y = (sh - TITLE_BAR as f32 - 8.0).max(0.0);
                win.x = (cursor.0 - grab_x).clamp(-win.width as f32 * 0.6, max_x);
                win.y = (cursor.1 - grab_y).clamp(8.0, max_y);
            }
        }

        let focused_id = self.windows.last().map(|w| w.id);
        let dragging_id = self.dragging.map(|(id, _, _)| id);

        for win in self.windows.iter_mut() {
            let focused = Some(win.id) == focused_id;
            win.lift
                .set_target(if Some(win.id) == dragging_id { 1.0 } else { 0.0 });
            win.lift.step(dt);
            win.open.step(dt);
            if let Some(close) = win.closing.as_mut() {
                close.step(dt);
                continue;
            }

            let (ox, oy) = win.content_origin();
            let local = (cursor.0 - ox, cursor.1 - oy);
            let inside = local.0 >= 0.0
                && local.1 >= 0.0
                && local.0 < win.surface.width() as f32
                && local.1 < win.surface.height() as f32;
            let input = AppInput {
                cursor: local,
                clicked: just_pressed && inside && focused,
                focused,
                space,
                reset,
            };
            win.app.update(dt, &input);
            // The compositor owns the surface lifecycle: an app always draws
            // onto a blank one and never has to think about the frame before.
            win.surface.clear(Color::TRANSPARENT);
            win.app.draw(&mut win.surface);
            win.surface.round_corners(CORNER - CONTENT_INSET, false, true);
        }

        self.windows
            .retain(|w| w.closing.as_ref().map_or(true, |c| !c.is_done()));
        consumed
    }

    pub fn draw(&self, frame: &mut Frame) {
        let focused_id = self.windows.last().map(|w| w.id);
        for win in &self.windows {
            self.draw_window(frame, win, Some(win.id) == focused_id);
        }
    }

    fn draw_window(&self, frame: &mut Frame, win: &Window, focused: bool) {
        let (_, opacity) = win.presentation();
        if opacity < 4 {
            return;
        }
        let (x, y, w, h) = win.frame_rect();
        let scale = w as f32 / win.width as f32;
        let radius = (CORNER as f32 * scale) as i32;
        let fade = |a: u32| ((a * opacity as u32) / 255) as u8;

        shapes::drop_shadow(frame, x, y, w, h, radius);

        let tint = if focused { 112 } else { 92 };
        frame.glass_fill(x, y, w, h, radius, Color::LUMEN_CARD.with_alpha(fade(tint)));
        shapes::glass_highlights(frame, x, y, w, h, radius, fade(255));

        // Title bar: a hairline under it, the close dot, and the app's name.
        let bar_h = (TITLE_BAR as f32 * scale) as i32;
        shapes::fill_rect(
            frame,
            x + 10,
            y + bar_h - 1,
            w - 20,
            1,
            Color::rgba(255, 255, 255, fade(90)),
        );

        let dot_x = x + (20.0 * scale) as i32;
        let dot_y = y + bar_h / 2;
        let dot_r = (CLOSE_DOT as f32 * scale) as i32;
        let dot_color = if focused {
            Color::rgb(255, 120, 130)
        } else {
            Color::rgba(255, 255, 255, 150)
        };
        shapes::fill_circle(frame, dot_x, dot_y, dot_r, dot_color.with_alpha(fade(dot_color.a as u32)));
        shapes::fill_circle(
            frame,
            dot_x - dot_r / 4,
            dot_y - dot_r / 4,
            (dot_r / 3).max(1),
            Color::rgba(255, 255, 255, fade(150)),
        );

        let title = win.app.title();
        let title_x = x + w / 2 - text::measure(&font_data::FONT_UI, title) / 2;
        let title_baseline = y + bar_h / 2 + 7;
        let ink = if focused { fade(230) } else { fade(150) };
        text::draw_text(frame, &font_data::FONT_UI, title, title_x, title_baseline + 1, Color::WHITE.with_alpha(fade(70)));
        text::draw_text(frame, &font_data::FONT_UI, title, title_x, title_baseline, Color::LUMEN_INK.with_alpha(ink));

        let content_cx = x as f32 + (CONTENT_INSET as f32 * scale) + (win.surface.width() as f32 * scale) * 0.5;
        let content_cy = y as f32 + bar_h as f32 + (win.surface.height() as f32 * scale) * 0.5;
        win.surface
            .blit_scaled(frame, content_cx, content_cy, scale, opacity);

        if focused {
            shapes::stroke_rounded_rect(
                frame,
                x,
                y,
                w,
                h,
                radius,
                Color::WHITE.with_alpha(fade(120)),
            );
        }
    }
}
