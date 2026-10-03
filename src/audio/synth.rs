//! Sample generators: every effect is a few oscillators, noise and an envelope.

use std::f32::consts::TAU;

pub const RATE: u32 = 22_050;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sfx {
    Lance,
    Torpedo,
    Missile,
    Explosion,
    ShieldHit,
    HullCreak,
    Alarm,
    DockClamp,
    Beep,
    Warp,
    Scan,
    Footstep,
    Gunshot,
    Door,
}

impl Sfx {
    pub const ALL: [Sfx; 14] = [
        Sfx::Lance,
        Sfx::Torpedo,
        Sfx::Missile,
        Sfx::Explosion,
        Sfx::ShieldHit,
        Sfx::HullCreak,
        Sfx::Alarm,
        Sfx::DockClamp,
        Sfx::Beep,
        Sfx::Warp,
        Sfx::Scan,
        Sfx::Footstep,
        Sfx::Gunshot,
        Sfx::Door,
    ];
}

/// Tiny deterministic white-noise source.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 23) as f32 - 1.0
    }
}

/// One-pole low-pass filter state.
struct Lp(f32);

impl Lp {
    fn run(&mut self, x: f32, a: f32) -> f32 {
        self.0 += (x - self.0) * a;
        self.0
    }
}

fn len(sec: f32) -> usize {
    (sec * RATE as f32) as usize
}

fn square(p: f32) -> f32 {
    if (p % 1.0) < 0.5 { 1.0 } else { -1.0 }
}

fn saw(p: f32) -> f32 {
    (p % 1.0) * 2.0 - 1.0
}

/// Integrate a frequency curve into phase (cycles).
fn sweep(n: usize, f: impl Fn(f32) -> f32) -> Vec<f32> {
    let mut ph = 0.0;
    (0..n)
        .map(|i| {
            ph += f(i as f32 / n as f32) / RATE as f32;
            ph
        })
        .collect()
}

pub fn render(s: Sfx) -> Vec<f32> {
    let mut nz = Noise(0xC0FFEE ^ s as u32);
    let mut v: Vec<f32> = match s {
        Sfx::Lance => {
            let n = len(0.30);
            let ph = sweep(n, |t| 2000.0 - 1500.0 * t);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    (saw(ph[i]) * 0.5 + (ph[i] * TAU).sin() * 0.5) * (1.0 - t).powf(1.6)
                })
                .collect()
        }
        Sfx::Torpedo => {
            let n = len(0.7);
            let ph = sweep(n, |t| 140.0 - 80.0 * t);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let whoosh = lp.run(nz.next(), 0.12) * 1.6;
                    ((ph[i] * TAU).sin() * 0.8 + whoosh * 0.6)
                        * (1.0 - t).powf(1.2)
                        * (t * 12.0).min(1.0)
                })
                .collect()
        }
        Sfx::Missile => {
            let n = len(0.55);
            let ph = sweep(n, |t| 400.0 + 900.0 * t);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let hiss = nz.next() - lp.run(nz.next(), 0.3);
                    (hiss * 0.5 + (ph[i] * TAU).sin() * 0.25)
                        * (1.0 - t).powf(0.8)
                        * (t * 20.0).min(1.0)
                })
                .collect()
        }
        Sfx::Explosion => {
            let n = len(1.4);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let cutoff = 0.35 * (1.0 - t) + 0.02;
                    let body = lp.run(nz.next(), cutoff) * 2.2;
                    let thump = ((60.0 - 30.0 * t) * t * TAU * 1.4).sin();
                    (body * 0.8 + thump * 0.6) * (-t * 4.5).exp()
                })
                .collect()
        }
        Sfx::ShieldHit => {
            let n = len(0.22);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let f = 900.0 + 120.0 * (t * 40.0).sin();
                    ((t * f * 0.22 * TAU).sin() * 0.6 + nz.next() * 0.25) * (1.0 - t).powf(2.0)
                })
                .collect()
        }
        Sfx::HullCreak => {
            let n = len(0.7);
            let ph = sweep(n, |t| 70.0 + 18.0 * (t * 9.0).sin());
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    saw(ph[i]) * 0.5 * (t * TAU / 2.0).sin()
                })
                .collect()
        }
        Sfx::Alarm => {
            let n = len(0.7);
            (0..n)
                .map(|i| {
                    let t = i as f32 / RATE as f32;
                    let f = if ((t * 2.857) as u32).is_multiple_of(2) {
                        640.0
                    } else {
                        880.0
                    };
                    square(t * f)
                        * 0.35
                        * if i + 400 > n {
                            (n - i) as f32 / 400.0
                        } else {
                            1.0
                        }
                })
                .collect()
        }
        Sfx::DockClamp => {
            let n = len(0.35);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let click = lp.run(nz.next(), 0.5) * (-t * 30.0).exp();
                    let thud = ((t * 0.35 * 95.0) * TAU).sin() * (-t * 9.0).exp();
                    click * 0.8 + thud
                })
                .collect()
        }
        Sfx::Beep => {
            let n = len(0.08);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    (i as f32 / RATE as f32 * 880.0 * TAU).sin() * 0.5 * (1.0 - t)
                })
                .collect()
        }
        Sfx::Warp => {
            let n = len(1.0);
            let ph = sweep(n, |t| 120.0 + 1800.0 * t * t);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    ((ph[i] * TAU).sin() * 0.5 + saw(ph[i] * 0.5) * 0.25)
                        * (t * 8.0).min(1.0)
                        * (1.0 - t * t)
                })
                .collect()
        }
        Sfx::Scan => {
            let n = len(0.4);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let f = 1200.0 + 400.0 * (t * 28.0).sin();
                    (i as f32 / RATE as f32 * f * TAU).sin() * 0.35 * (1.0 - t)
                })
                .collect()
        }
        Sfx::Footstep => {
            let n = len(0.12);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    lp.run(nz.next(), 0.18) * 1.8 * (-t * 14.0).exp()
                })
                .collect()
        }
        Sfx::Gunshot => {
            let n = len(0.25);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    (nz.next() * 0.7 + lp.run(nz.next(), 0.1)) * (-t * 14.0).exp()
                })
                .collect()
        }
        Sfx::Door => {
            let n = len(0.6);
            let ph = sweep(n, |t| 180.0 - 90.0 * t);
            let mut lp = Lp(0.0);
            (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let hiss = lp.run(nz.next(), 0.1) * 0.8;
                    (saw(ph[i]) * 0.3 + hiss) * (t * 10.0).min(1.0) * (1.0 - t).powf(0.7)
                })
                .collect()
        }
    };
    let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(1e-6);
    let gain = 0.85 / peak.max(0.85);
    for x in &mut v {
        *x *= gain;
    }
    v
}

/// Seamless minor-key drone + "lur" melody loop (32 s).
pub fn music() -> Vec<f32> {
    const SECONDS: usize = 32;
    let n = SECONDS * RATE as usize;
    let mut out = vec![0.0f32; n];
    // Drone: frequencies snapped to whole cycles per loop so the loop point is seamless.
    let snap = |f: f32| (f * SECONDS as f32).round() / SECONDS as f32;
    let pad = [(55.0, 0.30), (82.5, 0.18), (110.0, 0.14), (130.8, 0.10)];
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f32 / RATE as f32;
        let lfo = 0.6 + 0.4 * (t * TAU / SECONDS as f32 * 2.0).sin();
        for (f, a) in pad {
            let f = snap(f);
            *o += ((t * f * TAU).sin() + 0.3 * (t * f * 2.0 * TAU).sin()) * a * lfo;
        }
    }
    // Slow horn-like melody (A minor pentatonic), one note every 4 s.
    let notes = [220.0, 261.6, 329.6, 293.7, 220.0, 196.0, 261.6, 220.0];
    for (k, f) in notes.iter().enumerate() {
        let start = k * 4 * RATE as usize;
        let dur = (3.4 * RATE as f32) as usize;
        for j in 0..dur {
            let t = j as f32 / RATE as f32;
            let env = (t * 3.0).min(1.0) * (1.0 - j as f32 / dur as f32).powf(1.5);
            let vib = 1.0 + 0.004 * (t * 5.5 * TAU).sin();
            let p = t * f * vib * TAU;
            let s = (p.sin() + 0.5 * (2.0 * p).sin() + 0.25 * (3.0 * p).sin()) * 0.11 * env;
            out[start + j] += s;
        }
    }
    let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs())).max(1e-6);
    for x in &mut out {
        *x *= 0.7 / peak;
    }
    out
}

/// Encode mono float samples as a 16-bit PCM WAV file.
pub fn to_wav(samples: &[f32], rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&1u16.to_le_bytes()); // mono
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_is_audible_finite_and_bounded() {
        for s in Sfx::ALL {
            let v = render(s);
            assert!(v.len() > 500, "{s:?} too short");
            assert!(v.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "{s:?}");
            let rms = (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
            assert!(rms > 0.02, "{s:?} is silent (rms {rms})");
        }
    }

    #[test]
    fn effects_are_deterministic() {
        assert_eq!(render(Sfx::Explosion), render(Sfx::Explosion));
    }

    #[test]
    fn wav_header_is_valid() {
        let w = to_wav(&[0.0, 0.5, -0.5], 22_050);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes([w[40], w[41], w[42], w[43]]), 6);
        assert_eq!(w.len(), 44 + 6);
    }

    #[test]
    fn music_loops_seamlessly() {
        let m = music();
        assert_eq!(m.len(), 32 * RATE as usize);
        assert!((m[0] - m[m.len() - 1]).abs() < 0.05, "click at loop point");
        assert!(m.iter().all(|x| x.is_finite() && x.abs() <= 0.71));
    }
}
