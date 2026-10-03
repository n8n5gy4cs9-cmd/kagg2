//! 3D drawing helpers: low-resolution pixel render target, vertex-lit models, billboards and the
//! post-process pass (palette-friendly dithering, bloom, vignette) that gives the retro-modern look.

use macroquad::material::{Material, MaterialParams, load_material};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

use crate::assets::Assets;
use crate::meshgen::Model;

/// Target height of the internal pixel-art image (rounded to an integer upscale factor).
const TARGET_H: f32 = 270.0;

const VERT: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
}";

const FRAG_TEX: &str = "#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}";

/// Upscale pass: soft bloom from bright pixels, ordered dither + colour quantisation, vignette.
const FRAG_POST: &str = "#version 100
precision mediump float;
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 Res;
float bayer(vec2 p) {
    vec2 q = mod(floor(p), 4.0);
    float x0 = mod(q.x, 2.0); float y0 = mod(q.y, 2.0);
    float x1 = floor(q.x / 2.0); float y1 = floor(q.y / 2.0);
    return (abs(x0 - y0) * 8.0 + y0 * 4.0 + abs(x1 - y1) * 2.0 + y1) / 16.0 - 0.5;
}
void main() {
    vec2 px = 1.0 / Res;
    vec3 c = texture2D(Texture, uv).rgb;
    vec3 glow = vec3(0.0);
    for (int i = 0; i < 4; i++) {
        float a = float(i) * 1.5708 + 0.785;
        vec2 o = vec2(cos(a), sin(a));
        glow += max(texture2D(Texture, uv + o * px * 2.0).rgb - 0.62, 0.0);
        glow += max(texture2D(Texture, uv + o * px * 5.0).rgb - 0.62, 0.0) * 0.6;
    }
    c += glow * 0.32;
    c = (c - 0.5) * 1.07 + 0.5;
    vec2 d = uv - 0.5;
    c *= 1.0 - dot(d, d) * 0.85;
    float levels = 15.0;
    c = floor(c * levels + 0.5 + bayer(uv * Res) * 0.9) / levels;
    gl_FragColor = vec4(c, 1.0);
}";

pub struct Renderer {
    additive: Material,
    post: Material,
    rt: Option<RenderTarget>,
    size: (u32, u32),
    pub scale: u32,
}

impl Renderer {
    pub fn new() -> Result<Renderer, macroquad::Error> {
        let additive = load_material(
            ShaderSource::Glsl {
                vertex: VERT,
                fragment: FRAG_TEX,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    depth_write: false,
                    depth_test: Comparison::LessOrEqual,
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::One,
                    )),
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;
        let post = load_material(
            ShaderSource::Glsl {
                vertex: VERT,
                fragment: FRAG_POST,
            },
            MaterialParams {
                uniforms: vec![UniformDesc::new("Res", UniformType::Float2)],
                ..Default::default()
            },
        )?;
        Ok(Renderer {
            additive,
            post,
            rt: None,
            size: (0, 0),
            scale: 3,
        })
    }

    /// Size of the internal pixel image for the current window.
    pub fn internal_size(&self) -> (u32, u32) {
        let s = (screen_height() / TARGET_H).round().max(1.0);
        (
            (screen_width() / s).ceil() as u32,
            (screen_height() / s).ceil() as u32,
        )
    }

    /// (Re)create the render target if the window size changed; returns the internal size.
    pub fn ensure_target(&mut self) -> (u32, u32) {
        let s = (screen_height() / TARGET_H).round().max(1.0);
        let size = self.internal_size();
        if self.rt.is_none() || self.size != size || self.scale != s as u32 {
            let rt = render_target_ex(
                size.0,
                size.1,
                RenderTargetParams {
                    sample_count: 1,
                    depth: true,
                },
            );
            rt.texture.set_filter(FilterMode::Nearest);
            self.rt = Some(rt);
            self.size = size;
            self.scale = s as u32;
        }
        size
    }

    /// 3D camera that renders into the pixel target.
    pub fn camera(&self, position: Vec3, target: Vec3, up: Vec3, fov_deg: f32) -> Camera3D {
        Camera3D {
            position,
            target,
            up,
            fovy: fov_deg.to_radians(),
            aspect: Some(self.size.0 as f32 / self.size.1.max(1) as f32),
            projection: Projection::Perspective,
            render_target: self.rt.clone(),
            viewport: None,
            z_near: 1.0,
            z_far: 400_000.0,
        }
    }

    /// 2D camera over the pixel target (cockpit frame etc.), pixel coordinates, y down.
    pub fn camera_2d(&self) -> Camera2D {
        let mut c = Camera2D::from_display_rect(Rect::new(
            0.0,
            0.0,
            self.size.0 as f32,
            self.size.1 as f32,
        ));
        c.render_target = self.rt.clone();
        c
    }

    /// Blit the pixel target to the window through the post-process shader.
    pub fn present(&self) {
        let Some(rt) = &self.rt else { return };
        set_default_camera();
        gl_use_material(&self.post);
        self.post
            .set_uniform("Res", vec2(self.size.0 as f32, self.size.1 as f32));
        draw_texture_ex(
            &rt.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(
                    (self.size.0 * self.scale) as f32,
                    (self.size.1 * self.scale) as f32,
                )),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();
    }

    /// Run `f` with additive blending (glows, beams, dust).
    pub fn additive(&self, f: impl FnOnce()) {
        gl_use_material(&self.additive);
        f();
        gl_use_default_material();
    }
}

/// Sun-lit colour for a surface normal: warm key light, cool ambient, optional cel bands.
pub fn lighting(normal: Vec3, sun: Vec3, bands: bool) -> [f32; 3] {
    let d = normal.dot(sun);
    let lit = if bands {
        (d.max(0.0) * 3.0 + 0.45).floor().clamp(0.0, 3.0) / 3.0
    } else {
        let t = ((d + 0.12) / 0.5).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let amb = if bands {
        [0.36, 0.38, 0.50]
    } else {
        [0.20, 0.22, 0.34]
    };
    let key = [1.0, 0.94, 0.82];
    std::array::from_fn(|i| (amb[i] + key[i] * lit * 0.8).min(1.0))
}

fn to_u8(c: [f32; 3], tint: [f32; 3], a: f32) -> [u8; 4] {
    [
        (c[0] * tint[0] * 255.0).clamp(0.0, 255.0) as u8,
        (c[1] * tint[1] * 255.0).clamp(0.0, 255.0) as u8,
        (c[2] * tint[2] * 255.0).clamp(0.0, 255.0) as u8,
        (a * 255.0) as u8,
    ]
}

/// Where and how big a model is drawn.
#[derive(Clone, Copy)]
pub struct Pose {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: f32,
}

impl Pose {
    pub fn new(pos: Vec3, rot: Quat, scale: f32) -> Pose {
        Pose { pos, rot, scale }
    }
}

/// How a model is lit: sun direction, cel bands vs smooth, or full-bright.
#[derive(Clone, Copy)]
pub struct Shade {
    pub sun: Vec3,
    pub bands: bool,
    pub lit: bool,
}

impl Shade {
    pub fn new(sun: Vec3, bands: bool, lit: bool) -> Shade {
        Shade { sun, bands, lit }
    }
}

/// Draw a model with the given pose, lit by the sun direction in `shade`.
/// `tex_name` is used for parts without their own texture.
pub fn draw_model(assets: &Assets, model: &Model, tex_name: &str, pose: Pose, shade: Shade) {
    for part in &model.parts {
        let name = if part.texture.is_empty() {
            tex_name
        } else {
            part.texture
        };
        let verts: Vec<Vertex> = part
            .verts
            .iter()
            .map(|v| {
                let n = pose.rot * v.normal;
                let c = if shade.lit {
                    lighting(n, shade.sun, shade.bands)
                } else {
                    [1.0; 3]
                };
                Vertex {
                    position: pose.pos + pose.rot * (v.pos * pose.scale),
                    uv: v.uv,
                    color: to_u8(c, v.tint, 1.0),
                    normal: Vec4::new(n.x, n.y, n.z, 0.0),
                }
            })
            .collect();
        draw_mesh(&Mesh {
            vertices: verts,
            indices: part.idx.clone(),
            texture: Some(assets.tex(name).clone()),
        });
    }
}

/// Camera-facing textured quad of world `size`, centred on `pos`. Use inside `Renderer::additive`.
pub fn draw_billboard(tex: &Texture2D, pos: Vec3, size: f32, color: Color, right: Vec3, up: Vec3) {
    let (r, u) = (right * size * 0.5, up * size * 0.5);
    let c = [
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    ];
    let v = |p: Vec3, uv: Vec2| Vertex {
        position: p,
        uv,
        color: c,
        normal: Vec4::ZERO,
    };
    draw_mesh(&Mesh {
        vertices: vec![
            v(pos - r + u, vec2(0.0, 0.0)),
            v(pos + r + u, vec2(1.0, 0.0)),
            v(pos + r - u, vec2(1.0, 1.0)),
            v(pos - r - u, vec2(0.0, 1.0)),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(tex.clone()),
    });
}

/// Like [`draw_billboard`] but samples only the `uv` rectangle `(u0, v0, u1, v1)` of the texture.
pub fn draw_billboard_uv(
    tex: &Texture2D,
    pos: Vec3,
    size: f32,
    color: Color,
    right: Vec3,
    up: Vec3,
    uv: (f32, f32, f32, f32),
) {
    let (r, u) = (right * size * 0.5, up * size * 0.5);
    let c = [
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    ];
    let v = |p: Vec3, uv: Vec2| Vertex {
        position: p,
        uv,
        color: c,
        normal: Vec4::ZERO,
    };
    draw_mesh(&Mesh {
        vertices: vec![
            v(pos - r + u, vec2(uv.0, uv.1)),
            v(pos + r + u, vec2(uv.2, uv.1)),
            v(pos + r - u, vec2(uv.2, uv.3)),
            v(pos - r - u, vec2(uv.0, uv.3)),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: Some(tex.clone()),
    });
}

/// Streak between two world points with the given width, facing the camera at `eye`.
pub fn draw_streak(a: Vec3, b: Vec3, width: f32, color: Color, eye: Vec3) {
    let side = (b - a).cross(a - eye).normalize_or_zero() * width * 0.5;
    let c = [
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a * 255.0) as u8,
    ];
    let v = |p: Vec3| Vertex {
        position: p,
        uv: vec2(0.5, 0.5),
        color: c,
        normal: Vec4::ZERO,
    };
    draw_mesh(&Mesh {
        vertices: vec![v(a - side), v(a + side), v(b + side), v(b - side)],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: None,
    });
}
