//! Tiny immediate-mode widgets for the station panels (pixel font, mouse, no layout engine).

use macroquad::prelude::*;

use super::pixfont;
use crate::assets::Assets;
use crate::sim::command::Command;
use crate::sim::world::EntityId;

pub const CYAN: Color = Color::new(0.21, 0.78, 0.88, 1.0);
pub const BONE: Color = Color::new(0.85, 0.80, 0.70, 1.0);
pub const EMBER: Color = Color::new(1.0, 0.54, 0.17, 1.0);
pub const GOOD: Color = Color::new(0.35, 0.85, 0.45, 1.0);
pub const WARN: Color = Color::new(0.95, 0.75, 0.2, 1.0);
pub const BAD: Color = Color::new(0.95, 0.3, 0.25, 1.0);
pub const PANEL_BG: Color = Color::new(0.03, 0.05, 0.08, 0.97);

/// What a panel asks the scene to do.
#[derive(Debug, Clone)]
pub enum Action {
    Cmd(Command),
    Hail,
    Reply(usize),
    CloseDialog,
    SelectTarget(EntityId),
}

/// Per-frame drawing context shared by the widgets.
pub struct Ctx<'a> {
    pub assets: &'a Assets,
    /// Font pixel scale for panel text.
    pub fs: f32,
    pub mouse: Vec2,
    pub clicked: bool,
    pub right_clicked: bool,
    pub down: bool,
}

impl<'a> Ctx<'a> {
    pub fn new(assets: &'a Assets, fs: f32) -> Ctx<'a> {
        let (mx, my) = mouse_position();
        Ctx {
            assets,
            fs,
            mouse: vec2(mx, my),
            clicked: is_mouse_button_pressed(MouseButton::Left),
            right_clicked: is_mouse_button_pressed(MouseButton::Right),
            down: is_mouse_button_down(MouseButton::Left),
        }
    }

    /// Line height in pixels.
    pub fn lh(&self) -> f32 {
        10.0 * self.fs
    }

    pub fn text(&self, s: &str, x: f32, y: f32, c: Color) {
        pixfont::draw(self.assets, s, x, y, self.fs, c);
    }

    pub fn text_right(&self, s: &str, right: f32, y: f32, c: Color) {
        pixfont::draw_right(self.assets, s, right, y, self.fs, c);
    }

    pub fn text_center(&self, s: &str, cx: f32, y: f32, c: Color) {
        pixfont::draw_centered(self.assets, s, cx, y, self.fs, c);
    }

    pub fn hover(&self, r: Rect) -> bool {
        r.contains(self.mouse)
    }

    /// Filled panel with a title bar.
    pub fn panel(&self, r: Rect, title: &str) {
        draw_rectangle(r.x, r.y, r.w, r.h, PANEL_BG);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.21, 0.78, 0.88, 0.6));
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            self.lh() + 4.0,
            Color::new(0.21, 0.78, 0.88, 0.18),
        );
        self.text(title, r.x + 8.0, r.y + 4.0, CYAN);
    }

    /// Clickable button; returns true on the frame it is pressed.
    pub fn button(&self, r: Rect, label: &str, active: bool) -> bool {
        let hot = self.hover(r);
        let bg = match (active, hot) {
            (true, _) => Color::new(0.85, 0.36, 0.20, 0.9),
            (false, true) => Color::new(0.25, 0.32, 0.40, 0.95),
            _ => Color::new(0.12, 0.16, 0.22, 0.9),
        };
        draw_rectangle(r.x, r.y, r.w, r.h, bg);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.5, 0.6, 0.7, 0.7));
        let c = if active { WHITE } else { BONE };
        self.text_center(label, r.x + r.w / 2.0, r.y + (r.h - 8.0 * self.fs) / 2.0, c);
        hot && self.clicked
    }

    /// Horizontal bar with fraction `f` (clamped); optional right-aligned label.
    pub fn bar(&self, r: Rect, f: f32, color: Color) {
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_rectangle(r.x, r.y, r.w * f.clamp(0.0, 1.0), r.h, color);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.5, 0.6, 0.7, 0.5));
    }

    /// Draggable slider over `0..=max`; returns the new value when the user changed it.
    pub fn slider(&self, r: Rect, value: f32, max: f32, color: Color) -> Option<f32> {
        self.bar(r, value / max, color);
        if self.down && self.hover(r) {
            return Some(((self.mouse.x - r.x) / r.w * max).clamp(0.0, max));
        }
        None
    }
}

/// Colour from a health fraction: green → amber → red.
pub fn health_color(f: f32) -> Color {
    if f > 0.6 {
        GOOD
    } else if f > 0.3 {
        WARN
    } else {
        BAD
    }
}

/// Word-wrap `s` to at most `cols` characters per line.
pub fn wrap(s: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        if !cur.is_empty() && cur.len() + 1 + word.len() > cols {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_respects_width() {
        let l = wrap("the quick brown fox jumps over the lazy dog", 12);
        assert!(l.iter().all(|s| s.len() <= 12));
        assert_eq!(l.join(" "), "the quick brown fox jumps over the lazy dog");
    }
}
