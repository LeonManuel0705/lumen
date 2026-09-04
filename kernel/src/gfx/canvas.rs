use super::{Color, Rect};

/// Anything the drawing primitives can paint into: the screen, an off-screen
/// buffer, or a window's own surface. Keeping this one trait between the shapes
/// and the pixels is what lets a window draw itself with the same code the
/// desktop uses.
pub trait Canvas {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    /// The region writes are allowed to land in. Primitives narrow their loops
    /// to this so that repainting a small damaged area does not walk the whole
    /// shape, and the pixel entry points enforce it so that a primitive which
    /// forgets cannot corrupt the rest of the buffer.
    fn clip(&self) -> Rect {
        Rect::new(0, 0, self.width() as i32, self.height() as i32)
    }
    fn put_pixel(&mut self, x: usize, y: usize, c: Color);
    fn blend_pixel(&mut self, x: usize, y: usize, c: Color);
    fn read_pixel(&self, x: usize, y: usize) -> Color;

    /// The clip as loop bounds: `(x0, y0, x1, y1)`.
    #[inline(always)]
    fn bounds(&self) -> (i32, i32, i32, i32) {
        let c = self.clip();
        (c.x0, c.y0, c.x1, c.y1)
    }

    fn paint(&mut self, x: usize, y: usize, c: Color) {
        if c.a == 255 {
            self.put_pixel(x, y, c);
        } else {
            self.blend_pixel(x, y, c);
        }
    }

    /// Fills a horizontal run with one colour. Worth overriding for the same
    /// reason as `blend_row`: the format and the blend weights are the same for
    /// every pixel in the run.
    fn fill_row(&mut self, x: usize, y: usize, len: usize, c: Color) {
        for i in 0..len {
            self.paint(x + i, y, c);
        }
    }

    /// Composites a run of pixels in one call. The per-pixel path has to decide
    /// the pixel format for every pixel it touches; a target that knows its own
    /// layout can hoist that decision out of the loop, which is most of the
    /// win when a window blits its content.
    fn blend_row(&mut self, x: usize, y: usize, src: &[Color], opacity: u8) {
        for (i, c) in src.iter().enumerate() {
            if c.a == 0 {
                continue;
            }
            self.paint(x + i, y, c.fade(opacity));
        }
    }
}
