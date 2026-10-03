//! Procedural texture generator (T02). Every texture in `ASSETS.md` is produced here from code,
//! seeded by its name, so reruns are byte-identical.
//!
//! Look: palette ramps + ordered dithering + optional "big pixel" blocks — retro pixel art
//! that stays crisp when drawn with nearest filtering.

pub mod board;
pub mod chars;
pub mod font;
pub mod image;
pub mod noise;
pub mod ships;
pub mod space;
pub mod ui;

use std::fmt;
use std::path::Path;

pub use image::Image;

/// Errors from writing textures.
#[derive(Debug)]
pub enum TexError {
    Io(std::io::Error),
    Png(png::EncodingError),
}

impl fmt::Display for TexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TexError::Io(e) => write!(f, "io error: {e}"),
            TexError::Png(e) => write!(f, "png error: {e}"),
        }
    }
}

impl std::error::Error for TexError {}

impl From<std::io::Error> for TexError {
    fn from(e: std::io::Error) -> Self {
        TexError::Io(e)
    }
}

impl From<png::EncodingError> for TexError {
    fn from(e: png::EncodingError) -> Self {
        TexError::Png(e)
    }
}

/// Summary of one generated texture.
#[derive(Debug, Clone)]
pub struct TexRow {
    pub name: String,
    pub w: u32,
    pub h: u32,
    pub mean: [u8; 3],
}

/// Generation context handed to every recipe.
#[derive(Clone, Copy)]
pub struct Ctx {
    pub seed: u32,
    pub w: u32,
    pub h: u32,
}

type Recipe = Box<dyn Fn(&Ctx) -> Image>;

/// One texture to generate.
pub struct Spec {
    pub name: String,
    pub w: u32,
    pub h: u32,
    /// Must wrap seamlessly (checked by tests).
    pub tile: bool,
    make: Recipe,
}

impl Spec {
    pub fn render(&self) -> Image {
        (self.make)(&Ctx {
            seed: noise::seed_of(&self.name),
            w: self.w,
            h: self.h,
        })
    }
}

/// Registry helper used by the recipe modules.
pub struct Registry(pub Vec<Spec>);

impl Registry {
    pub fn add(&mut self, name: &str, w: u32, h: u32, f: impl Fn(&Ctx) -> Image + 'static) {
        self.0.push(Spec {
            name: name.to_owned(),
            w,
            h,
            tile: false,
            make: Box::new(f),
        });
    }

    /// Like [`Registry::add`] but flagged as seamlessly tileable.
    pub fn tile(&mut self, name: &str, w: u32, h: u32, f: impl Fn(&Ctx) -> Image + 'static) {
        self.add(name, w, h, f);
        if let Some(s) = self.0.last_mut() {
            s.tile = true;
        }
    }
}

/// Every texture of the game.
pub fn all() -> Vec<Spec> {
    let mut r = Registry(Vec::new());
    space::register(&mut r);
    ships::register(&mut r);
    board::register(&mut r);
    chars::register(&mut r);
    ui::register(&mut r);
    r.0
}

/// Write every texture, `manifest.json` and `contact_sheet.png` into `dir`.
/// Returns one summary row per texture.
pub fn write_all(dir: &Path) -> Result<Vec<TexRow>, TexError> {
    std::fs::create_dir_all(dir)?;
    let mut rows = Vec::new();
    let mut images = Vec::new();
    for spec in all() {
        let img = spec.render();
        img.write_png(&dir.join(format!("{}.png", spec.name)))?;
        rows.push(TexRow {
            name: spec.name.clone(),
            w: img.w,
            h: img.h,
            mean: img.mean_rgb(),
        });
        images.push(img);
    }
    let mut json = String::from("{\n  \"textures\": [\n");
    for (
        i,
        TexRow {
            name,
            w,
            h,
            mean: m,
        },
    ) in rows.iter().enumerate()
    {
        json.push_str(&format!(
            "    {{\"name\": \"{name}\", \"w\": {w}, \"h\": {h}, \"mean\": [{}, {}, {}]}}{}\n",
            m[0],
            m[1],
            m[2],
            if i + 1 < rows.len() { "," } else { "" }
        ));
    }
    json.push_str("  ]\n}\n");
    std::fs::write(dir.join("manifest.json"), json)?;
    contact_sheet(&images).write_png(&dir.parent().unwrap_or(dir).join("contact_sheet.png"))?;
    Ok(rows)
}

/// All textures shrunk into cells on one sheet for eyeballing.
pub fn contact_sheet(images: &[Image]) -> Image {
    let (cell, cols) = (128u32, 14u32);
    let rows = (images.len() as u32).div_ceil(cols);
    let mut sheet = Image::filled(cols * (cell + 4), rows * (cell + 4), image::hex(0x202428));
    for (i, img) in images.iter().enumerate() {
        let (cx, cy) = (
            i as u32 % cols * (cell + 4) + 2,
            i as u32 / cols * (cell + 4) + 2,
        );
        let k = (img.w.max(img.h)).div_ceil(cell).max(1);
        let small = if k > 1 { img.shrink(k) } else { img.clone() };
        // checker behind alpha so transparent sprites are visible
        for y in 0..small.h {
            for x in 0..small.w {
                let g = if (x / 8 + y / 8) % 2 == 0 { 0.16 } else { 0.22 };
                sheet.put((cx + x) as i32, (cy + y) as i32, [g, g, g + 0.02, 1.0]);
            }
        }
        sheet.blit(&small, cx as i32, cy as i32);
    }
    sheet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_complete_and_unique() {
        let specs = all();
        assert!(specs.len() >= 110, "only {} textures", specs.len());
        let mut names: Vec<_> = specs.iter().map(|s| s.name.as_str()).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len(), "duplicate texture names");
        for needed in [
            "sky_stars",
            "sun",
            "mars",
            "earth",
            "hull_viking_a",
            "hull_minion_a",
            "font_atlas",
            "explosion_sheet",
            "weapon_valkyrie",
            "title_logo",
            "door_lock_gold",
        ] {
            assert!(names.contains(&needed), "missing {needed}");
        }
    }

    #[test]
    fn sizes_match_and_output_is_deterministic() {
        for spec in all() {
            let a = spec.render();
            assert_eq!((a.w, a.h), (spec.w, spec.h), "{}", spec.name);
            // spot-check determinism on small textures only (keeps the test quick)
            if spec.w * spec.h <= 128 * 128 {
                assert_eq!(a.hash(), spec.render().hash(), "{}", spec.name);
            }
        }
    }

    #[test]
    fn tileable_textures_have_no_seam() {
        for spec in all().into_iter().filter(|s| s.tile) {
            let r = spec.render().seam_ratio();
            assert!(r < 2.5, "{} seam ratio {r}", spec.name);
        }
    }
}
