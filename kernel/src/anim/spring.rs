#[derive(Copy, Clone)]
pub struct Spring {
    pub current: f32,
    pub velocity: f32,
    pub target: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Spring {
    pub const fn new(value: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            current: value,
            velocity: 0.0,
            target: value,
            stiffness,
            damping,
        }
    }

    pub fn step(&mut self, dt: f32) {
        const SUBSTEPS: usize = 4;
        let sdt = dt / SUBSTEPS as f32;
        for _ in 0..SUBSTEPS {
            let force = -self.stiffness * (self.current - self.target);
            let damp = -self.damping * self.velocity;
            let accel = force + damp;
            self.velocity += accel * sdt;
            self.current += self.velocity * sdt;
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn nudge(&mut self, velocity: f32) {
        self.velocity += velocity;
    }
}
