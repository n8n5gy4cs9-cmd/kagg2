//! Procedural audio (T17): sound effects and music rendered to PCM in code (no audio files).
//!
//! `synth` is pure and headless; playback lives in `gfx::sound`.

pub mod synth;

pub use synth::{Sfx, music, render, to_wav};
