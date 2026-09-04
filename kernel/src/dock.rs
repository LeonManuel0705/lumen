use crate::anim::easing;
use crate::apps::AppKind;
use crate::display::Frame;
use crate::gfx::{shapes, Canvas, Color, Rect};

pub const SLOTS: usize = 6;
const ICON: i32 = 52;
const PANEL_HEIGHT: i32 = 84;
const PANEL_BOTTOM_MARGIN: i32 = 22;
const PANEL_RADIUS: i32 = 26;
const PANEL_INSET: i32 = 12;

/// Which app each slot launches. Empty slots are the roadmap: they are drawn
/// so the dock has its final shape, but they do nothing yet.
const APPS: [Option<AppKind>; SLOTS] = [
    Some(AppKind::Ball),
    Some(AppKind::Clock),
    None,
    None,
    None,
    None,
];

const TINTS: [Color; SLOTS] = [
    Color::LUMEN_ACCENT,
    Color::LUMEN_GLOW,
    Color::rgb(255, 150, 180),
    Color::rgb(150, 230, 200),
    Color::rgb(200, 180, 255),
    Color::LUMEN_INK,
];

/// The launcher strip along the bottom of the desktop: a glass panel with one
/// icon per app slot.
pub struct Dock {
    panel: Rect,
}

impl Dock {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        let w = (screen_w as f32 * 0.42) as i32;
        let x = (screen_w - w) / 2;
        let y = screen_h - PANEL_HEIGHT - PANEL_BOTTOM_MARGIN;
        Self {
            panel: Rect::from_size(x, y, w, PANEL_HEIGHT),
        }
    }

    pub fn rect(&self) -> Rect {
        self.panel
    }

    pub fn app_in_slot(slot: usize) -> Option<AppKind> {
        APPS.get(slot).copied().flatten()
    }

    fn gap(&self) -> i32 {
        let w = self.panel.x1 - self.panel.x0;
        (w - ICON * SLOTS as i32 - PANEL_INSET * 2) / (SLOTS as i32 - 1).max(1)
    }

    pub fn icon_rect(&self, slot: usize) -> Rect {
        let x = self.panel.x0 + PANEL_INSET + slot as i32 * (ICON + self.gap());
        let y = self.panel.y0 + (PANEL_HEIGHT - ICON) / 2;
        Rect::from_size(x, y, ICON, ICON)
    }

    /// The slot under a point, with vertical slack above and below the icon
    /// because 52 px squares are small targets.
    pub fn slot_at(&self, px: f32, py: f32) -> Option<usize> {
        (0..SLOTS).find(|&slot| {
            let r = self.icon_rect(slot);
            px >= r.x0 as f32
                && px < r.x1 as f32
                && py >= (r.y0 - 6) as f32
                && py < (r.y1 + 10) as f32
        })
    }

    /// Draws the panel and its icons. `entrance` runs 0 to 1 while the desktop
    /// arrives: the panel rises into place and the icons pop in one after
    /// another.
    pub fn draw(&self, frame: &mut Frame, entrance: f32) {
        let t = entrance;
        if t <= 0.01 {
            return;
        }
        let alpha = |a: u8| (a as f32 * t) as u8;
        let lift = ((1.0 - t) * 40.0) as i32;
        let (dx, dy) = (self.panel.x0, self.panel.y0 + lift);
        let (dw, dh) = (self.panel.x1 - self.panel.x0, PANEL_HEIGHT);

        if !Rect::from_size(dx, dy, dw, dh)
            .union(&shapes::shadow_bounds(dx, dy, dw, dh))
            .intersects(&frame.clip())
        {
            return;
        }
        shapes::drop_shadow(frame, dx, dy, dw, dh, PANEL_RADIUS);
        frame.glass_fill(dx, dy, dw, dh, PANEL_RADIUS, Color::LUMEN_CARD.with_alpha(130), alpha(255));
        shapes::glass_highlights(frame, dx, dy, dw, dh, PANEL_RADIUS, alpha(255));

        let icon_y = dy + (dh - ICON) / 2;
        for (slot, tint) in TINTS.iter().enumerate() {
            // Each icon lands a beat after the one before it.
            let start = 0.20 + slot as f32 * 0.07;
            let local = ((t - start) / 0.45).clamp(0.0, 1.0);
            if local <= 0.0 {
                continue;
            }
            let pop = easing::ease_out_back(local);
            let size = (ICON as f32 * (0.55 + 0.45 * pop)) as i32;
            let cell = self.icon_rect(slot);
            let ix = cell.x0 + (ICON - size) / 2;
            let iy = icon_y + (ICON - size) / 2;
            let a = (local * 255.0) as u8;
            shapes::fill_rounded_rect(frame, ix, iy, size, size, size * 27 / 100, tint.with_alpha(a));
            shapes::fill_rect(frame, ix + 8, icon_y + ICON + 4, size - 16, 2, Color::WHITE.with_alpha(a / 2));
        }
    }
}
