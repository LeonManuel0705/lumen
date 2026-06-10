#![allow(dead_code)]

pub fn ease_in_quad(t: f32) -> f32 { t * t }

pub fn ease_out_quad(t: f32) -> f32 { 1.0 - (1.0 - t) * (1.0 - t) }

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        let u = -2.0 * t + 2.0;
        1.0 - (u * u * u) * 0.5
    }
}

pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    let u = t - 1.0;
    1.0 + c3 * u * u * u + c1 * u * u
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
