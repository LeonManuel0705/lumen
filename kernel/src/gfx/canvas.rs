use super::{Color, Rect};

pub trait Canvas {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn clip(&self) -> Rect {
        Rect::new(0, 0, self.width() as i32, self.height() as i32)
    }
    fn put_pixel(&mut self, x: usize, y: usize, c: Color);
    fn blend_pixel(&mut self, x: usize, y: usize, c: Color);
    fn read_pixel(&self, x: usize, y: usize) -> Color;

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

    fn fill_row(&mut self, x: usize, y: usize, len: usize, c: Color) {
        for i in 0..len {
            self.paint(x + i, y, c);
        }
    }

    fn blend_row(&mut self, x: usize, y: usize, src: &[Color], opacity: u8) {
        for (i, c) in src.iter().enumerate() {
            if c.a == 0 {
                continue;
            }
            self.paint(x + i, y, c.fade(opacity));
        }
    }
}
