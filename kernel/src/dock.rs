use crate::anim::{easing, Spring, Tween};
use crate::apps::AppKind;
use crate::display::{Frame, GlassCache};
use core::cell::RefCell;
use crate::gfx::color::mul255;
use crate::gfx::{font_data, icons, shapes, text, Canvas, Color, Damage, Rect};
use crate::widgets;

pub const SLOTS: usize = 6;
const ICON: f32 = 52.0;
const PITCH: f32 = 92.0;
const PANEL_W: i32 = 560;
const PANEL_H: i32 = 84;
const PANEL_R: i32 = 26;
const PANEL_BOTTOM_MARGIN: i32 = 22;
const PAD: i32 = 24;
const BASELINE_INSET: i32 = (PANEL_H + ICON as i32) / 2;
const FLOOR_OFFSET: i32 = 2;
const IND_INSET: i32 = 74;
const IND_H: i32 = 5;

const R_FIELD: f32 = 150.0;
const GROWTH: f32 = 0.54;
const SPREAD_K: f32 = 20.8;
const LIFT: f32 = 6.0;
const SHAKE_DUR: f32 = 0.44;
const SHAKE_AMP: f32 = 9.0;
const GRAVITY: f32 = 2600.0;
const TIP_DWELL: f32 = 0.26;

const APPS: [Option<AppKind>; SLOTS] = [
    Some(AppKind::Ball),
    Some(AppKind::Clock),
    None,
    None,
    None,
    None,
];

const LABELS: [&str; SLOTS] = [
    "Ball",
    "Uhr",
    "Farben · bald",
    "Skizze · bald",
    "Einstellungen · bald",
    "Wetter · bald",
];

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DockAction {
    None,
    Consumed,
    Launch(AppKind),
    Denied,
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
struct SlotVisual {
    live: bool,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    alpha: u8,
    emblem: u8,
    floor_y: i32,
    shadow_rx: i32,
    shadow_ry: i32,
    shadow_a: u8,
    ring_a: u8,
    dim: u8,
    ind_y: i32,
    ind_x: i32,
    ind_w: i32,
    ind_a: u8,
}

fn qi(v: f32) -> i32 {
    if v >= 0.0 { (v + 0.5) as i32 } else { (v - 0.5) as i32 }
}

fn qu(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
struct PanelVisual {
    y: i32,
    alpha: u8,
}

#[derive(Copy, Clone, PartialEq, Eq, Default)]
struct TipVisual {
    cx: i32,
    y: i32,
    w: i32,
    alpha: u8,
    slot: u8,
}

struct Slot {
    mag: Spring,
    emblem: Spring,
    indicator: Spring,
    press: Spring,
    squash: Spring,
    bounce_y: f32,
    bounce_v: f32,
    bounce_v0: f32,
    bounce_hops: u8,
    bounce_delay: f32,
    bouncing: bool,
    run_delay: f32,
    press_hold: f32,
    shake_t: f32,
    entrance: Tween,
    vis: SlotVisual,
    drawn: SlotVisual,
}

impl Slot {
    fn new(index: usize) -> Self {
        Self {
            mag: Spring::new(0.0, 520.0, 32.0),
            emblem: Spring::new(0.0, 220.0, 20.0),
            indicator: Spring::new(0.0, 480.0, 24.0),
            press: Spring::new(0.0, 900.0, 38.0),
            squash: Spring::new(0.0, 620.0, 26.0),
            bounce_y: 0.0,
            bounce_v: 0.0,
            bounce_v0: 0.0,
            bounce_hops: 0,
            bounce_delay: 0.0,
            bouncing: false,
            run_delay: 0.0,
            press_hold: 0.0,
            shake_t: SHAKE_DUR,
            entrance: Tween::new(0.50, 0.16 + 0.055 * index as f32),
            vis: SlotVisual::default(),
            drawn: SlotVisual::default(),
        }
    }

    fn shake_env(&self) -> f32 {
        if self.shake_t >= SHAKE_DUR {
            return 0.0;
        }
        let u = self.shake_t / SHAKE_DUR;
        (1.0 - u) * (1.0 - u)
    }

    fn shake_offset(&self) -> f32 {
        if self.shake_t >= SHAKE_DUR {
            return 0.0;
        }
        let u = self.shake_t / SHAKE_DUR;
        let p = u * 3.5;
        let f = p - (p as i32) as f32;
        let tri = if f < 0.5 { 4.0 * f - 1.0 } else { 3.0 - 4.0 * f };
        let wave = tri * (1.5 - 0.5 * tri * tri);
        SHAKE_AMP * self.shake_env() * wave
    }
}

pub struct Dock {
    panel: Rect,
    presence: f32,
    slots: [Slot; SLOTS],
    state: [u8; SLOTS],
    hovered: Option<usize>,
    dwell: f32,
    held: Option<usize>,
    tip_slot: u8,
    tip_hold: f32,
    tip_alpha: Spring,
    tip_x: Spring,
    panel_vis: PanelVisual,
    panel_drawn: PanelVisual,
    tip_vis: TipVisual,
    tip_drawn: TipVisual,
    landing: Option<(f32, f32)>,
    panel_glass: RefCell<GlassCache>,
}

impl Dock {
    pub fn new(screen_w: i32, screen_h: i32) -> Self {
        let x = ((screen_w - PANEL_W) / 2).max(0);
        let y = screen_h - PANEL_H - PANEL_BOTTOM_MARGIN;
        let panel = Rect::from_size(x, y, PANEL_W, PANEL_H);
        let cx0 = (x + PAD) as f32 + ICON * 0.5;
        Self {
            panel,
            presence: 0.0,
            slots: [Slot::new(0), Slot::new(1), Slot::new(2), Slot::new(3), Slot::new(4), Slot::new(5)],
            state: [0; SLOTS],
            hovered: None,
            dwell: 0.0,
            held: None,
            tip_slot: 0,
            tip_hold: 0.0,
            tip_alpha: Spring::new(0.0, 500.0, 40.0),
            tip_x: Spring::new(cx0, 420.0, 30.0),
            panel_vis: PanelVisual::default(),
            panel_drawn: PanelVisual::default(),
            tip_vis: TipVisual::default(),
            tip_drawn: TipVisual::default(),
            landing: None,
            panel_glass: RefCell::new(GlassCache::new()),
        }
    }

    pub fn app_in_slot(slot: usize) -> Option<AppKind> {
        APPS.get(slot).copied().flatten()
    }

    fn home_cx(&self, slot: usize) -> f32 {
        (self.panel.x0 + PAD) as f32 + ICON * 0.5 + PITCH * slot as f32
    }

    fn baseline(&self) -> f32 {
        (self.panel.y0 + BASELINE_INSET) as f32
    }

    fn floor_y(&self) -> i32 {
        self.panel.y0 + BASELINE_INSET + FLOOR_OFFSET
    }

    fn ind_y(&self) -> i32 {
        self.panel.y0 + IND_INSET
    }

    pub fn set_presence(&mut self, t: f32) {
        self.presence = t.clamp(0.0, 1.0);
    }

    pub fn begin_entrance(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.entrance.restart();
        }
    }

    pub fn slot_at(&self, px: f32, py: f32) -> Option<usize> {
        if py < (self.panel.y0 - 22) as f32 || py >= self.panel.y1 as f32 {
            return None;
        }
        if px < self.panel.x0 as f32 || px >= self.panel.x1 as f32 {
            return None;
        }
        let rel = px - (self.panel.x0 + PAD - (PITCH as i32 - ICON as i32) / 2) as f32;
        if rel < 0.0 {
            return None;
        }
        let slot = (rel / PITCH) as usize;
        (slot < SLOTS).then_some(slot)
    }

    fn on_panel(&self, px: f32, py: f32) -> bool {
        px >= self.panel.x0 as f32 && px < self.panel.x1 as f32 && py >= self.panel.y0 as f32 && py < self.panel.y1 as f32
    }

    pub fn activate(&mut self, slot: usize) -> DockAction {
        let running = self.state[slot] != 0;
        self.tip_slot = slot as u8;
        self.tip_hold = 0.9;
        let s = &mut self.slots[slot];
        s.bounce_delay = 0.07;
        s.bounce_y = 0.0;
        s.bouncing = true;
        s.squash.nudge(-9.0);
        match Self::app_in_slot(slot) {
            Some(kind) => {
                s.bounce_v0 = if running { 200.0 } else { 360.0 };
                s.bounce_hops = if running { 0 } else { 1 };
                if !running {
                    s.run_delay = 0.22;
                }
                DockAction::Launch(kind)
            }
            None => {
                s.bouncing = false;
                s.shake_t = 0.0;
                DockAction::Denied
            }
        }
    }

    pub fn activate_from_keyboard(&mut self, slot: usize) -> DockAction {
        let action = self.activate(slot);
        let s = &mut self.slots[slot];
        s.press_hold = 0.10;
        s.mag.nudge(14.0);
        if self.tip_alpha.current < 0.05 {
            self.tip_x.current = self.home_cx(slot);
            self.tip_x.velocity = 0.0;
        }
        action
    }

    pub fn take_landing(&mut self) -> Option<(f32, f32)> {
        self.landing.take()
    }

    pub fn hover_tint(&self) -> Option<Color> {
        self.hovered.map(|i| icons::PLATES[i].indicator)
    }

    pub fn update(
        &mut self,
        dt: f32,
        cursor: (f32, f32),
        pressed: bool,
        held: bool,
        state: &[u8; SLOTS],
        interactive: bool,
    ) -> DockAction {
        self.state = *state;

        let mut action = DockAction::None;
        if pressed && interactive {
            if let Some(slot) = self.slot_at(cursor.0, cursor.1) {
                action = self.activate(slot);
                self.held = Some(slot);
            } else if self.on_panel(cursor.0, cursor.1) {
                action = DockAction::Consumed;
            }
        }
        if !held {
            self.held = None;
        }

        let gate_top = (self.panel.y0 - 8) as f32;
        let vertical = if cursor.1 >= gate_top {
            1.0
        } else {
            (1.0 - (gate_top - cursor.1) / 44.0).clamp(0.0, 1.0)
        };
        let mut fields = [0.0f32; SLOTS];
        for (i, field) in fields.iter_mut().enumerate() {
            let a = ((cursor.0 - self.home_cx(i)).abs() / R_FIELD).min(1.0);
            let bump = (1.0 - a * a) * (1.0 - a * a);
            *field = if interactive { bump * vertical } else { 0.0 };
        }

        let floor = self.floor_y() as f32;
        let first_cx = (self.panel.x0 + PAD) as f32 + ICON * 0.5;
        for (i, slot) in self.slots.iter_mut().enumerate() {
            slot.mag.set_target(fields[i]);
            slot.emblem.set_target(fields[i]);
            let pressing = self.held == Some(i) || slot.press_hold > 0.0;
            slot.press.set_target(if pressing { 1.0 } else { 0.0 });

            let ind_target = if slot.run_delay > 0.0 {
                0.0
            } else {
                match self.state[i] {
                    0 => 0.0,
                    1 => 1.0,
                    _ => 1.6,
                }
            };
            slot.indicator.set_target(ind_target);
            slot.squash.set_target(0.0);

            slot.mag.step(dt);
            slot.emblem.step(dt);
            slot.indicator.step(dt);
            slot.press.step(dt);
            slot.squash.step(dt);
            slot.entrance.step(dt);
            slot.press_hold = (slot.press_hold - dt).max(0.0);
            slot.run_delay = (slot.run_delay - dt).max(0.0);
            if slot.shake_t < SHAKE_DUR {
                slot.shake_t = (slot.shake_t + dt).min(SHAKE_DUR);
            }

            if slot.bouncing {
                if slot.bounce_delay > 0.0 {
                    slot.bounce_delay -= dt;
                    if slot.bounce_delay <= 0.0 {
                        slot.bounce_v = slot.bounce_v0;
                    }
                } else {
                    slot.bounce_v -= GRAVITY * dt;
                    slot.bounce_y += slot.bounce_v * dt;
                    if slot.bounce_y <= 0.0 && slot.bounce_v < 0.0 {
                        slot.bounce_y = 0.0;
                        let impact = -slot.bounce_v;
                        slot.squash.nudge(impact * 0.053);
                        if slot.bounce_hops > 0 {
                            slot.bounce_hops -= 1;
                            slot.bounce_v = impact * 0.42;
                        } else {
                            slot.bouncing = false;
                            slot.bounce_v = 0.0;
                        }
                        if self.landing.is_none() {
                            self.landing = Some((first_cx + PITCH * i as f32, floor));
                        }
                    }
                }
            }
        }

        let mut hovered = None;
        let mut best = 0.55f32;
        for (i, f) in fields.iter().enumerate() {
            if *f > best {
                best = *f;
                hovered = Some(i);
            }
        }
        if hovered == self.hovered {
            self.dwell += dt;
        } else {
            self.hovered = hovered;
            self.dwell = 0.0;
        }

        self.tip_hold = (self.tip_hold - dt).max(0.0);
        let visible = self.tip_alpha.current > 0.05;
        if let Some(h) = hovered {
            if visible || self.dwell >= TIP_DWELL {
                self.tip_slot = h as u8;
            }
        }
        let wanted = self.tip_hold > 0.0 || (hovered.is_some() && (visible || self.dwell >= TIP_DWELL));
        if wanted && !visible {
            self.tip_x.current = self.home_cx(self.tip_slot as usize);
            self.tip_x.velocity = 0.0;
        }
        self.tip_x.set_target(self.home_cx(self.tip_slot as usize));
        self.tip_alpha.set_target(if wanted { 1.0 } else { 0.0 });
        self.tip_x.step(dt);
        self.tip_alpha.step(dt);

        self.rebuild_visuals(cursor);
        action
    }

    fn rebuild_visuals(&mut self, cursor: (f32, f32)) {
        let panel_lift = qi((1.0 - self.presence) * 40.0);
        self.panel_vis = PanelVisual {
            y: self.panel.y0 + panel_lift,
            alpha: (self.presence * 255.0) as u8,
        };

        let baseline = self.baseline();
        let floor_y = self.floor_y() + panel_lift;
        let ind_y = self.ind_y() + panel_lift;
        for i in 0..SLOTS {
            let home = self.home_cx(i);
            let s = &mut self.slots[i];
            let te = s.entrance.t();
            let alpha = qu(easing::ease_out_quad(te) * 255.0);
            if alpha == 0 {
                s.vis = SlotVisual::default();
                continue;
            }
            let pop = easing::ease_out_back(te);
            let m = s.mag.current.clamp(0.0, 1.2);
            let env = s.shake_env();
            let press = s.press.current.clamp(0.0, 1.0);
            let mult = (1.0 - 0.13 * press) * (1.0 - 0.05 * env);
            let base = ICON * (1.0 + GROWTH * m) * (0.55 + 0.45 * pop) * mult;
            let sq = s.squash.current.clamp(-0.45, 1.0);
            let size_x = base * (1.0 + 0.26 * sq);
            let size_y = base * (1.0 - 0.30 * sq);

            let u = ((cursor.0 - home) / R_FIELD).clamp(-1.0, 1.0);
            let dx = -u * (1.0 - u * u) * SPREAD_K * m;
            let cx = home + dx + s.shake_offset();
            let bottom = baseline - LIFT * m + (1.0 - pop) * 14.0 + 3.0 * press - s.bounce_y + panel_lift as f32;

            let lift = LIFT * m + s.bounce_y;
            let ind = s.indicator.current.max(0.0);
            let focus = ((ind - 1.0).clamp(0.0, 0.6)) / 0.6;
            let ind_w = 14.0 * ind.min(1.0) + 10.0 * focus;
            let ind_a = easing::lerp(150.0, 240.0, focus) * (ind_w / 6.0).min(1.0);

            s.vis = SlotVisual {
                live: true,
                x: qi(cx - size_x * 0.5),
                y: qi(bottom - size_y),
                w: qi(size_x).max(1),
                h: qi(size_y).max(1),
                alpha,
                emblem: qu(s.emblem.current.clamp(0.0, 1.0) * 255.0),
                floor_y,
                shadow_rx: qi((size_x * 0.42 - lift * 0.50).max(4.0)),
                shadow_ry: qi((4.0 - lift * 0.15).max(1.5)),
                shadow_a: mul255(qu((70.0 - lift * 2.2).max(12.0)), alpha),
                ring_a: qu(170.0 * env),
                dim: qu(70.0 * env),
                ind_y,
                ind_x: qi(cx - ind_w * 0.5),
                ind_w: qi(ind_w),
                ind_a: if ind_w >= 0.5 { mul255(qu(ind_a), alpha) } else { 0 },
            };
        }

        let slot = (self.tip_slot as usize).min(SLOTS - 1);
        let alpha = qu(self.tip_alpha.current.clamp(0.0, 1.0) * 255.0);
        let mag = self.slots[slot].mag.current.clamp(0.0, 1.0);
        self.tip_vis = if alpha < 4 {
            TipVisual::default()
        } else {
            TipVisual {
                cx: qi(self.tip_x.current),
                y: self.panel.y0 - 44 - qi(12.0 * mag),
                w: text::measure(&font_data::FONT_UI, LABELS[slot]) + 30,
                alpha,
                slot: slot as u8,
            }
        };
    }

    fn panel_bounds(&self, v: &PanelVisual) -> Rect {
        if v.alpha == 0 {
            return Rect::EMPTY;
        }
        Rect::from_size(self.panel.x0, v.y, PANEL_W, PANEL_H)
            .union(&shapes::shadow_bounds(self.panel.x0, v.y, PANEL_W, PANEL_H))
    }

    fn slot_bounds(&self, v: &SlotVisual) -> Rect {
        if !v.live {
            return Rect::EMPTY;
        }
        let mut r = Rect::from_size(v.x, v.y, v.w, v.h).expand(6);
        if v.shadow_a > 0 {
            let cx = v.x + v.w / 2;
            r = r.union(&Rect::new(cx - v.shadow_rx - 2, v.floor_y - v.shadow_ry - 2, cx + v.shadow_rx + 2, v.floor_y + v.shadow_ry + 2));
        }
        if v.ind_a > 0 {
            r = r.union(&Rect::from_size(v.ind_x - 1, v.ind_y - 1, v.ind_w + 2, IND_H + 2));
        }
        r
    }

    fn tip_bounds(v: &TipVisual) -> Rect {
        if v.alpha == 0 {
            return Rect::EMPTY;
        }
        Rect::from_size(v.cx - v.w / 2 - 2, v.y - 2, v.w + 4, 34)
    }

    pub fn damage(&mut self, into: &mut Damage) {
        let mut r = Rect::EMPTY;
        if self.panel_vis != self.panel_drawn {
            r = r.union(&self.panel_bounds(&self.panel_drawn)).union(&self.panel_bounds(&self.panel_vis));
            self.panel_drawn = self.panel_vis;
        }
        for i in 0..SLOTS {
            if self.slots[i].vis != self.slots[i].drawn {
                let (old, new) = (self.slots[i].drawn, self.slots[i].vis);
                r = r.union(&self.slot_bounds(&old)).union(&self.slot_bounds(&new));
                self.slots[i].drawn = new;
            }
        }
        if self.tip_vis != self.tip_drawn {
            r = r.union(&Self::tip_bounds(&self.tip_drawn)).union(&Self::tip_bounds(&self.tip_vis));
            self.tip_drawn = self.tip_vis;
        }
        if !r.is_empty() {
            into.add(r);
        }
    }

    pub fn forget(&mut self) {
        self.panel_drawn = self.panel_vis;
        for slot in self.slots.iter_mut() {
            slot.drawn = slot.vis;
        }
        self.tip_drawn = self.tip_vis;
    }

    pub fn draw(&self, frame: &mut Frame) {
        let panel = self.panel_vis;
        if panel.alpha == 0 {
            return;
        }
        let clip = frame.clip();
        if self.panel_bounds(&panel).intersects(&clip) {
            let (dx, dy) = (self.panel.x0, panel.y);
            shapes::drop_shadow(frame, dx, dy, PANEL_W, PANEL_H, PANEL_R);
            frame.glass_fill_cached(&mut self.panel_glass.borrow_mut(), dx, dy, PANEL_W, PANEL_H, PANEL_R, Color::LUMEN_CARD.with_alpha(130), panel.alpha);
            shapes::glass_highlights(frame, dx, dy, PANEL_W, PANEL_H, PANEL_R, panel.alpha);
        }

        for (i, slot) in self.slots.iter().enumerate() {
            let v = slot.vis;
            if !v.live || !self.slot_bounds(&v).intersects(&clip) {
                continue;
            }
            let cx = v.x + v.w / 2;
            if v.shadow_a > 0 {
                shapes::fill_ellipse(frame, cx, v.floor_y, v.shadow_rx, v.shadow_ry, Color::rgba(20, 50, 90, v.shadow_a));
            }
            icons::draw_slot(frame, i, v.x, v.y, v.w, v.h, mul255(v.alpha, 255 - v.dim), v.emblem);
            if v.ind_a > 0 {
                shapes::fill_rounded_rect(frame, v.ind_x, v.ind_y, v.ind_w, IND_H, 2, icons::PLATES[i].indicator.with_alpha(v.ind_a));
            }
            if v.ring_a > 0 {
                let r = (v.w.min(v.h) * 31 + 50) / 100;
                shapes::stroke_rounded_rect(frame, v.x - 3, v.y - 3, v.w + 6, v.h + 6, r + 3, Color::rgba(255, 110, 120, v.ring_a));
            }
        }

        let tip = self.tip_vis;
        if tip.alpha > 0 && Self::tip_bounds(&tip).intersects(&clip) {
            let x = tip.cx - tip.w / 2;
            frame.glass_fill(x, tip.y, tip.w, 30, 15, Color::LUMEN_CARD.with_alpha(150), tip.alpha);
            shapes::stroke_rounded_rect(frame, x, tip.y, tip.w, 30, 15, Color::WHITE.with_alpha(mul255(90, tip.alpha)));
            let label = LABELS[(tip.slot as usize).min(SLOTS - 1)];
            widgets::text_embossed(frame, &font_data::FONT_UI, label, tip.cx, tip.y + 21, mul255(235, tip.alpha));
        }
    }
}
