pub struct Tween {
    elapsed: f32,
    delay: f32,
    duration: f32,
}

impl Tween {
    pub const fn new(duration: f32, delay: f32) -> Self {
        Self { elapsed: 0.0, delay, duration }
    }

    pub fn step(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    pub fn t(&self) -> f32 {
        ((self.elapsed - self.delay) / self.duration).clamp(0.0, 1.0)
    }

    #[allow(dead_code)]
    pub fn value(&self, ease: fn(f32) -> f32) -> f32 {
        ease(self.t())
    }

    #[allow(dead_code)]
    pub fn is_done(&self) -> bool {
        self.t() >= 1.0
    }

    #[allow(dead_code)]
    pub fn restart(&mut self) {
        self.elapsed = 0.0;
    }
}
