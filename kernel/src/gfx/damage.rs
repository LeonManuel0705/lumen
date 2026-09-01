use super::Rect;

/// How many separate regions a frame may repaint before they start getting
/// merged. Each region costs a background restore and a present, so a handful
/// of tight rectangles beats a long list of tiny ones.
pub const MAX_REGIONS: usize = 8;

/// Above this share of the screen, tracking regions stops paying for itself and
/// the frame is repainted whole.
const FULL_SCREEN_FRACTION: i64 = 70;

/// The regions of the screen that changed this frame.
///
/// The invariant the whole scheme rests on: the back buffer always holds the
/// last frame that was presented. So a region only needs repainting if
/// something will be drawn there this frame, or something was drawn there last
/// frame and has since moved away.
#[derive(Copy, Clone)]
pub struct Damage {
    regions: [Rect; MAX_REGIONS],
    len: usize,
    full: bool,
    screen: Rect,
}

impl Damage {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            regions: [Rect::EMPTY; MAX_REGIONS],
            len: 0,
            full: false,
            screen: Rect::new(0, 0, width, height),
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
        self.full = false;
    }

    pub fn mark_all(&mut self) {
        self.full = true;
        self.len = 0;
    }

    pub fn is_empty(&self) -> bool {
        !self.full && self.len == 0
    }

    pub fn add(&mut self, rect: Rect) {
        if self.full {
            return;
        }
        let rect = rect.intersect(&self.screen);
        if rect.is_empty() {
            return;
        }

        // Fold into an overlapping region if there is one. Merging can make the
        // merged region overlap a third, so keep folding until it settles.
        let mut candidate = rect;
        let mut i = 0;
        while i < self.len {
            if self.regions[i].intersects(&candidate) {
                candidate = self.regions[i].union(&candidate);
                self.len -= 1;
                self.regions[i] = self.regions[self.len];
                i = 0;
                continue;
            }
            i += 1;
        }

        if self.len < MAX_REGIONS {
            self.regions[self.len] = candidate;
            self.len += 1;
        } else {
            // Full: grow whichever region swallows it most cheaply.
            let mut best = 0;
            let mut best_cost = i64::MAX;
            for i in 0..self.len {
                let cost = self.regions[i].union(&candidate).area() - self.regions[i].area();
                if cost < best_cost {
                    best_cost = cost;
                    best = i;
                }
            }
            self.regions[best] = self.regions[best].union(&candidate);
        }

        if self.total_area() * 100 > self.screen.area() * FULL_SCREEN_FRACTION {
            self.mark_all();
        }
    }

    pub fn add_all(&mut self, other: &Damage) {
        if other.full {
            self.mark_all();
            return;
        }
        for i in 0..other.len {
            self.add(other.regions[i]);
        }
    }

    fn total_area(&self) -> i64 {
        (0..self.len).map(|i| self.regions[i].area()).sum()
    }

    /// The regions to repaint, in no particular order.
    pub fn regions(&self) -> &[Rect] {
        if self.full {
            core::slice::from_ref(&self.screen)
        } else {
            &self.regions[..self.len]
        }
    }
}
