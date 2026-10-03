//! Fixed-timestep accumulator (60 Hz).

/// Simulation step length in seconds.
pub const DT: f32 = 1.0 / 60.0;

/// Longest frame we will try to catch up on (avoids the spiral of death).
const MAX_FRAME: f32 = 0.25;

#[derive(Debug, Default, Clone, Copy)]
pub struct Stepper {
    acc: f32,
}

impl Stepper {
    /// Feed a frame time; returns how many fixed steps to run.
    pub fn advance(&mut self, frame_dt: f32) -> u32 {
        self.acc += frame_dt.clamp(0.0, MAX_FRAME);
        let mut n = 0;
        while self.acc >= DT {
            self.acc -= DT;
            n += 1;
        }
        n
    }

    /// Fraction of the next step already elapsed — render interpolation factor.
    pub fn alpha(&self) -> f32 {
        self.acc / DT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_and_clamps() {
        let mut s = Stepper::default();
        assert_eq!(s.advance(0.01), 0);
        assert_eq!(s.advance(0.01), 1);
        assert!((0.0..1.0).contains(&s.alpha()));
        assert!(s.advance(10.0) <= 16);
    }
}
