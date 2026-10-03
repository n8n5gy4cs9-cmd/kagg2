//! Audio playback (T17): loads the procedurally synthesised effects and the music loop.

use std::collections::HashMap;

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};

use crate::audio::{self, Sfx};

/// User volume sliders (`0.0..=1.0`).
#[derive(Debug, Clone, Copy)]
pub struct Volumes {
    pub master: f32,
    pub sfx: f32,
    pub music: f32,
}

impl Default for Volumes {
    fn default() -> Self {
        Volumes {
            master: 0.7,
            sfx: 0.8,
            music: 0.45,
        }
    }
}

pub struct Mixer {
    sfx: HashMap<Sfx, Sound>,
    music: Option<Sound>,
    pub vol: Volumes,
    music_on: bool,
}

impl Mixer {
    pub async fn new() -> Mixer {
        let mut sfx = HashMap::new();
        for s in Sfx::ALL {
            let wav = audio::to_wav(&audio::render(s), audio::synth::RATE);
            if let Ok(snd) = load_sound_from_bytes(&wav).await {
                sfx.insert(s, snd);
            }
        }
        let wav = audio::to_wav(&audio::music(), audio::synth::RATE);
        let music = load_sound_from_bytes(&wav).await.ok();
        Mixer {
            sfx,
            music,
            vol: Volumes::default(),
            music_on: false,
        }
    }

    /// Play `s` at relative volume `v` (`0..=1`).
    pub fn play(&self, s: Sfx, v: f32) {
        if let Some(snd) = self.sfx.get(&s) {
            let volume = (v * self.vol.sfx * self.vol.master).clamp(0.0, 1.0);
            if volume > 0.01 {
                play_sound(
                    snd,
                    PlaySoundParams {
                        looped: false,
                        volume,
                    },
                );
            }
        }
    }

    pub fn start_music(&mut self) {
        if let Some(m) = &self.music
            && !self.music_on
        {
            play_sound(
                m,
                PlaySoundParams {
                    looped: true,
                    volume: self.vol.music * self.vol.master,
                },
            );
            self.music_on = true;
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(m) = &self.music {
            stop_sound(m);
        }
        self.music_on = false;
    }

    /// Re-apply the sliders to the running music.
    pub fn refresh(&self) {
        if let (Some(m), true) = (&self.music, self.music_on) {
            set_sound_volume(m, self.vol.music * self.vol.master);
        }
    }

    pub fn music_on(&self) -> bool {
        self.music_on
    }
}
