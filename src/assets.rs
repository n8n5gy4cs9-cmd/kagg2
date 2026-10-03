//! Asset registry (T03): generated textures by name (with checkerboard fallback) and meshes.
//!
//! Textures come from `assets/textures/*.png` written by `texgen`; if the folder is missing the
//! loader regenerates it first, so a fresh checkout runs without an extra step.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use macroquad::prelude::*;

use crate::meshgen::{self, Model};
use crate::texgen;

/// Where `texgen` writes and the game reads textures.
pub const TEXTURE_DIR: &str = "assets/textures";

#[derive(Debug)]
pub enum AssetError {
    Generate(texgen::TexError),
    Load { name: String, reason: String },
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Generate(e) => write!(f, "texture generation failed: {e}"),
            AssetError::Load { name, reason } => write!(f, "cannot load texture {name}: {reason}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<texgen::TexError> for AssetError {
    fn from(e: texgen::TexError) -> Self {
        AssetError::Generate(e)
    }
}

pub struct Assets {
    textures: HashMap<String, Texture2D>,
    fallback: Texture2D,
    models: HashMap<&'static str, Model>,
    model_order: Vec<&'static str>,
    /// Textures that failed to load (replaced by the fallback).
    pub missing: Vec<String>,
}

fn checkerboard() -> Texture2D {
    let mut img = Image::gen_image_color(64, 64, Color::new(0.1, 0.1, 0.1, 1.0));
    for y in 0..64 {
        for x in 0..64 {
            if (x / 8 + y / 8) % 2 == 0 {
                img.set_pixel(x, y, Color::new(1.0, 0.0, 1.0, 1.0));
            }
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

impl Assets {
    /// Load every texture and build every mesh. Regenerates textures if none exist.
    pub async fn load() -> Result<Assets, AssetError> {
        let dir = Path::new(TEXTURE_DIR);
        if !dir.join("manifest.json").exists() {
            eprintln!("assets/textures missing - generating (one-off, a few seconds)...");
            texgen::write_all(dir)?;
        }
        let mut textures = HashMap::new();
        let mut missing = Vec::new();
        for spec in texgen::all() {
            let path = dir.join(format!("{}.png", spec.name));
            match load_texture(&path.to_string_lossy()).await {
                Ok(t) => {
                    // Soft backdrops look better smoothed; everything else stays crisp pixel art.
                    let soft = matches!(spec.name.as_str(), "sky_nebula" | "sky_stars");
                    t.set_filter(if soft {
                        FilterMode::Linear
                    } else {
                        FilterMode::Nearest
                    });
                    textures.insert(spec.name, t);
                }
                Err(e) => {
                    eprintln!(
                        "{}",
                        AssetError::Load {
                            name: spec.name.clone(),
                            reason: e.to_string()
                        }
                    );
                    missing.push(spec.name);
                }
            }
        }
        let list = meshgen::all();
        let model_order = list.iter().map(|m| m.name).collect();
        let models = list.into_iter().map(|m| (m.name, m)).collect();
        Ok(Assets {
            textures,
            fallback: checkerboard(),
            models,
            model_order,
            missing,
        })
    }

    /// Texture by name, or the magenta checkerboard.
    pub fn tex(&self, name: &str) -> &Texture2D {
        self.textures.get(name).unwrap_or(&self.fallback)
    }

    pub fn has_tex(&self, name: &str) -> bool {
        self.textures.contains_key(name)
    }

    pub fn model(&self, name: &str) -> Option<&Model> {
        self.models.get(name)
    }

    /// Model names in catalogue order (model viewer).
    pub fn model_names(&self) -> &[&'static str] {
        &self.model_order
    }
}
