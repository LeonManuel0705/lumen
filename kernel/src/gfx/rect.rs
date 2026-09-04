/// A half-open rectangle in screen pixels: `x0..x1` by `y0..y1`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub const EMPTY: Rect = Rect { x0: 0, y0: 0, x1: 0, y1: 0 };

    pub const fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub const fn from_size(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x0: x, y0: y, x1: x + w, y1: y + h }
    }

    pub const fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    pub const fn area(&self) -> i64 {
        if self.is_empty() {
            0
        } else {
            (self.x1 - self.x0) as i64 * (self.y1 - self.y0) as i64
        }
    }

    #[inline(always)]
    pub fn contains(&self, x: usize, y: usize) -> bool {
        let (x, y) = (x as i64, y as i64);
        x >= self.x0 as i64 && x < self.x1 as i64 && y >= self.y0 as i64 && y < self.y1 as i64
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    pub fn intersect(&self, other: &Rect) -> Rect {
        Rect {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        }
    }

    /// The smallest rectangle containing both. An empty operand is ignored, so
    /// this can be folded over a list that starts out empty.
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Rect {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    pub fn expand(&self, by: i32) -> Rect {
        if self.is_empty() {
            return *self;
        }
        Rect {
            x0: self.x0 - by,
            y0: self.y0 - by,
            x1: self.x1 + by,
            y1: self.y1 + by,
        }
    }

    pub fn translate(&self, dx: i32, dy: i32) -> Rect {
        if self.is_empty() {
            return *self;
        }
        Rect {
            x0: self.x0 + dx,
            y0: self.y0 + dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }
}
