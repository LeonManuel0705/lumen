use super::Color;

/// Anything the drawing primitives can paint into: the screen, an off-screen
/// buffer, or a window's own surface. Keeping this one trait between the shapes
/// and the pixels is what lets a window draw itself with the same code the
/// desktop uses.
pub trait Canvas {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn put_pixel(&mut self, x: usize, y: usize, c: Color);
    fn blend_pixel(&mut self, x: usize, y: usize, c: Color);
    fn read_pixel(&self, x: usize, y: usize) -> Color;

    fn paint(&mut self, x: usize, y: usize, c: Color) {
        if c.a == 255 {
            self.put_pixel(x, y, c);
        } else {
            self.blend_pixel(x, y, c);
        }
    }
}
