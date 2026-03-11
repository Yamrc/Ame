use super::Transition;

pub struct Linear;
#[allow(dead_code)]
pub struct EaseInQuad;
#[allow(dead_code)]
pub struct EaseOutQuad;
#[allow(dead_code)]
pub struct EaseInOutQuad;
#[allow(dead_code)]
pub struct EaseInOutCubic;
#[allow(dead_code)]
pub struct EaseOutSine;
#[allow(dead_code)]
pub struct EaseInExpo;

impl Transition for Linear {
    fn calculate(&self, t: f32) -> f32 {
        t
    }
}

impl Transition for EaseInQuad {
    fn calculate(&self, t: f32) -> f32 {
        t * t
    }
}

impl Transition for EaseOutQuad {
    fn calculate(&self, t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
}

impl Transition for EaseInOutQuad {
    fn calculate(&self, t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }
}

impl Transition for EaseInOutCubic {
    fn calculate(&self, t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }
}

impl Transition for EaseOutSine {
    fn calculate(&self, t: f32) -> f32 {
        (t * std::f32::consts::PI / 2.0).sin()
    }
}

impl Transition for EaseInExpo {
    fn calculate(&self, t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else {
            (2.0f32).powf(10.0 * t - 10.0)
        }
    }
}
