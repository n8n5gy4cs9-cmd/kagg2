//! Parallax starfield drifting past the camera (menu/briefing backdrop).

use macroquad::prelude::*;

struct Star {
    x: f32,
    y: f32,
    depth: f32,
}

pub struct Starfield {
    stars: Vec<Star>,
}

/// Tiny deterministic generator so the backdrop is identical every launch.
fn next(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / (1u32 << 24) as f32
}

impl Starfield {
    pub fn new(count: usize) -> Starfield {
        let mut seed = 0x4b41_4747u32;
        let stars = (0..count)
            .map(|_| Star {
                x: next(&mut seed),
                y: next(&mut seed),
                depth: 0.15 + next(&mut seed) * 0.85,
            })
            .collect();
        Starfield { stars }
    }

    /// Scroll stars leftwards; nearer stars (larger depth) move faster.
    pub fn update(&mut self, dt: f32) {
        for s in &mut self.stars {
            s.x -= dt * 0.02 * s.depth;
            if s.x < 0.0 {
                s.x += 1.0;
            }
        }
    }

    pub fn draw(&self) {
        let (w, h) = (screen_width(), screen_height());
        for s in &self.stars {
            let shade = 0.35 + 0.65 * s.depth;
            let color = Color::new(shade, shade, shade * 1.05, 1.0);
            draw_circle(s.x * w, s.y * h, 0.6 + 1.6 * s.depth, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_stay_in_unit_square_after_updates() {
        let mut f = Starfield::new(200);
        for _ in 0..1000 {
            f.update(0.016);
        }
        assert!(
            f.stars
                .iter()
                .all(|s| (0.0..=1.0).contains(&s.x) && (0.0..=1.0).contains(&s.y))
        );
    }
}
