//! Drawing. Spark's own pictures are made from simple shapes, so there are no image files to lose.
//! Your own pixel art is drawn as little textures.
//!
//! The editor and the game both use [`draw_type`], so a coin looks the same in both.

use crate::data::*;
use crate::engine::*;
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, pos2, vec2};
use std::cell::RefCell;
use std::collections::HashMap;

pub fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Mix a color with white (`amount` 0..1).
pub fn lighten(c: Color32, amount: f32) -> Color32 {
    let m = |v: u8| (v as f32 + (255.0 - v as f32) * amount).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(m(c.r()), m(c.g()), m(c.b()), c.a())
}

/// Mix a color with black (`amount` 0..1).
pub fn darken(c: Color32, amount: f32) -> Color32 {
    let m = |v: u8| (v as f32 * (1.0 - amount)).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(m(c.r()), m(c.g()), m(c.b()), c.a())
}

/// Little extras that change how a picture looks.
#[derive(Clone, Copy)]
pub struct Anim {
    pub time: f32,
    /// -1 looks left, +1 looks right.
    pub facing: f32,
    /// 0..1, used by springs when they are bouncing.
    pub squash: f32,
    /// Draw monsters as ghosts.
    pub ghost: bool,
}

impl Default for Anim {
    fn default() -> Self {
        Anim { time: 0.0, facing: 1.0, squash: 0.0, ghost: false }
    }
}

fn ellipse(center: Pos2, rx: f32, ry: f32) -> Vec<Pos2> {
    (0..24)
        .map(|i| {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            pos2(center.x + rx * a.cos(), center.y + ry * a.sin())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Your own pixel art
// ---------------------------------------------------------------------------

thread_local! {
    static TEXTURES: RefCell<HashMap<(u64, usize), egui::TextureHandle>> = RefCell::new(HashMap::new());
}

fn sprite_texture(ctx: &egui::Context, s: &Sprite, frame: usize) -> egui::TextureId {
    TEXTURES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 600 {
            cache.clear();
        }
        cache
            .entry((s.rev, frame))
            .or_insert_with(|| {
                let f = &s.frames[frame.min(s.frames.len() - 1)];
                let bytes: Vec<u8> = f.iter().flat_map(|p| p.iter().copied()).collect();
                let img = egui::ColorImage::from_rgba_unmultiplied([s.w, s.h], &bytes);
                ctx.load_texture(format!("sprite-{}-{frame}", s.rev), img, egui::TextureOptions::NEAREST)
            })
            .id()
    })
}

/// Draw pixel art. It is squeezed to fit `r` (keeping its shape) and flipped when facing left.
pub fn draw_pixels(p: &Painter, s: &Sprite, r: Rect, anim: Anim) {
    if s.w == 0 || s.h == 0 || s.frames.is_empty() {
        return;
    }
    let frame = s.frame_at(anim.time);
    let tex = sprite_texture(p.ctx(), s, frame);
    let scale = (r.width() / s.w as f32).min(r.height() / s.h as f32);
    let size = vec2(s.w as f32 * scale, s.h as f32 * scale);
    // Sit at the bottom middle of the box.
    let target = Rect::from_min_size(pos2(r.center().x - size.x / 2.0, r.max.y - size.y), size);
    let uv = if anim.facing < 0.0 {
        Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0))
    } else {
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
    };
    p.image(tex, target, uv, Color32::WHITE);
}

/// Draw an object type's picture inside `r` (its whole box).
pub fn draw_type(p: &Painter, t: &ObjectType, r: Rect, color: Color32, mut anim: Anim) {
    match &t.art {
        Art::Built(pic) => {
            anim.ghost |= t.behavior == Behavior::Follow && *pic == Pic::Enemy;
            draw_sprite(p, *pic, r, color, anim)
        }
        Art::Pixels(s) => draw_pixels(p, s, r, anim),
    }
}

// ---------------------------------------------------------------------------
// The pictures that come with Spark
// ---------------------------------------------------------------------------

/// Draw one built-in picture inside the rectangle `r`.
pub fn draw_sprite(p: &Painter, pic: Pic, r: Rect, color: Color32, anim: Anim) {
    let (w, h) = (r.width(), r.height());
    let s = w.min(h);
    let pt = |x: f32, y: f32| pos2(r.min.x + x * w, r.min.y + y * h);
    let fr = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(pt(x0, y0), pt(x1, y1));
    let thin = (s * 0.04).max(1.0);
    let outline = Stroke::new(thin, darken(color, 0.45));
    let dark = Color32::from_rgb(30, 30, 50);

    match pic {
        Pic::Wall => {
            p.rect_filled(r, s * 0.06, color);
            p.rect_filled(fr(0.0, 0.0, 1.0, 0.2), s * 0.06, lighten(color, 0.35));
            p.rect_filled(fr(0.0, 0.88, 1.0, 1.0), 0.0, darken(color, 0.25));
            let mortar = Stroke::new((s * 0.03).max(1.0), darken(color, 0.3));
            p.line_segment([pt(0.0, 0.52), pt(1.0, 0.52)], mortar);
            p.line_segment([pt(0.5, 0.2), pt(0.5, 0.52)], mortar);
            p.line_segment([pt(0.25, 0.52), pt(0.25, 0.88)], mortar);
            p.line_segment([pt(0.75, 0.52), pt(0.75, 0.88)], mortar);
            p.rect_stroke(r, s * 0.06, Stroke::new(thin, darken(color, 0.4)), StrokeKind::Inside);
        }
        Pic::Player => {
            let f = anim.facing.signum();
            p.rect_filled(fr(0.04, 0.08, 0.96, 1.0), s * 0.3, color);
            p.rect_filled(fr(0.12, 0.12, 0.88, 0.35), s * 0.2, lighten(color, 0.3));
            p.rect_stroke(fr(0.04, 0.08, 0.96, 1.0), s * 0.3, outline, StrokeKind::Inside);
            for ex in [0.3, 0.7] {
                let c = pt(ex + f * 0.05, 0.42);
                p.circle_filled(c, s * 0.15, Color32::WHITE);
                p.circle_filled(pos2(c.x + f * s * 0.05, c.y + s * 0.01), s * 0.075, dark);
            }
            let smile = vec![pt(0.34 + f * 0.04, 0.7), pt(0.5 + f * 0.04, 0.78), pt(0.66 + f * 0.04, 0.7)];
            p.add(Shape::line(smile, Stroke::new((s * 0.05).max(1.0), dark)));
        }
        Pic::Coin => {
            let phase = r.min.x * 0.37 + r.min.y * 0.21;
            let bob = (anim.time * 4.0 + phase).sin() * 0.04;
            let spin = if anim.time == 0.0 { 1.0 } else { (anim.time * 3.0 + phase).cos().abs().max(0.2) };
            let c = pt(0.5, 0.5 + bob);
            let ry = 0.36 * s;
            p.add(Shape::convex_polygon(ellipse(c, ry * spin, ry), darken(color, 0.3), Stroke::NONE));
            p.add(Shape::convex_polygon(ellipse(c, ry * 0.8 * spin, ry * 0.8), color, Stroke::NONE));
            p.add(Shape::line(
                vec![pos2(c.x - ry * 0.25 * spin, c.y - ry * 0.4), pos2(c.x - ry * 0.25 * spin, c.y + ry * 0.4)],
                Stroke::new((s * 0.06).max(1.0), lighten(color, 0.6)),
            ));
        }
        Pic::Enemy => {
            let f = if anim.facing < 0.0 { -1.0 } else { 1.0 };
            let wobble = (anim.time * 8.0).sin() * 0.025;
            let top = 0.12 + wobble;
            if anim.ghost {
                let ghost = lighten(color, 0.55);
                p.circle_filled(pt(0.5, top + 0.38), s * 0.38, ghost);
                p.rect_filled(fr(0.12, top + 0.38, 0.88, 0.88), 0.0, ghost);
                for i in 0..3 {
                    p.circle_filled(pt(0.2 + i as f32 * 0.3, 0.88), s * 0.1, ghost);
                }
                for ex in [0.36, 0.64] {
                    p.circle_filled(pt(ex, top + 0.36), s * 0.09, Color32::from_rgb(40, 30, 60));
                }
                p.circle_filled(pt(0.5, top + 0.58), s * 0.06, Color32::from_rgb(40, 30, 60));
            } else {
                p.rect_filled(fr(0.06, top, 0.94, 0.96), s * 0.32, color);
                p.rect_filled(fr(0.16, top + 0.04, 0.84, top + 0.2), s * 0.2, lighten(color, 0.25));
                p.rect_stroke(fr(0.06, top, 0.94, 0.96), s * 0.32, outline, StrokeKind::Inside);
                for ex in [0.33, 0.67] {
                    let c = pt(ex, top + 0.38);
                    p.circle_filled(c, s * 0.13, Color32::WHITE);
                    p.circle_filled(pos2(c.x + f * s * 0.04, c.y + s * 0.02), s * 0.07, Color32::from_rgb(30, 20, 30));
                }
                let brow = Stroke::new((s * 0.06).max(1.0), Color32::from_rgb(60, 20, 30));
                p.line_segment([pt(0.2, top + 0.14), pt(0.42, top + 0.26)], brow);
                p.line_segment([pt(0.8, top + 0.14), pt(0.58, top + 0.26)], brow);
                p.add(Shape::convex_polygon(vec![pt(0.38, 0.76), pt(0.46, 0.76), pt(0.42, 0.88)], Color32::WHITE, Stroke::NONE));
                p.add(Shape::convex_polygon(vec![pt(0.54, 0.76), pt(0.62, 0.76), pt(0.58, 0.88)], Color32::WHITE, Stroke::NONE));
            }
        }
        Pic::Goal => {
            let wave = (anim.time * 5.0).sin() * 0.05;
            p.rect_filled(fr(0.2, 0.06, 0.28, 1.0), s * 0.03, Color32::from_rgb(120, 110, 100));
            p.circle_filled(pt(0.24, 0.07), s * 0.07, Color32::from_rgb(255, 215, 80));
            p.add(Shape::convex_polygon(
                vec![pt(0.28, 0.12), pt(0.92 + wave, 0.26 - wave * 0.5), pt(0.28, 0.5)],
                color,
                Stroke::new(thin, darken(color, 0.4)),
            ));
            p.circle_filled(pt(0.5, 0.31), s * 0.05, Color32::WHITE);
        }
        Pic::Spikes => {
            p.rect_filled(fr(0.0, 0.88, 1.0, 1.0), 0.0, darken(color, 0.4));
            for i in 0..3 {
                let x0 = i as f32 / 3.0 + 0.02;
                let x1 = (i + 1) as f32 / 3.0 - 0.02;
                p.add(Shape::convex_polygon(vec![pt(x0, 0.9), pt((x0 + x1) / 2.0, 0.42), pt(x1, 0.9)], color, outline));
                p.line_segment([pt((x0 + x1) / 2.0, 0.5), pt((x0 + x1) / 2.0 - 0.04, 0.8)], Stroke::new(thin, lighten(color, 0.6)));
            }
        }
        Pic::Spring => {
            let top = 0.5 + anim.squash * 0.25;
            p.rect_filled(fr(0.1, 0.86, 0.9, 1.0), s * 0.05, Color32::from_rgb(90, 90, 105));
            let zig: Vec<Pos2> = (0..=6)
                .map(|i| {
                    let t = i as f32 / 6.0;
                    let x = if i % 2 == 0 { 0.28 } else { 0.72 };
                    pt(x, top + 0.08 + t * (0.86 - top - 0.08))
                })
                .collect();
            p.add(Shape::line(zig, Stroke::new((s * 0.07).max(1.5), Color32::from_rgb(190, 190, 205))));
            p.rect_filled(fr(0.06, top, 0.94, top + 0.12), s * 0.06, color);
            p.rect_filled(fr(0.1, top + 0.01, 0.9, top + 0.04), s * 0.02, lighten(color, 0.5));
        }
        Pic::Sign => {
            p.rect_filled(fr(0.44, 0.45, 0.56, 1.0), 0.0, darken(color, 0.35));
            p.rect_filled(fr(0.08, 0.1, 0.92, 0.58), s * 0.08, color);
            p.rect_stroke(fr(0.08, 0.1, 0.92, 0.58), s * 0.08, outline, StrokeKind::Inside);
            let ink = Stroke::new((s * 0.045).max(1.0), darken(color, 0.6));
            p.line_segment([pt(0.2, 0.26), pt(0.8, 0.26)], ink);
            p.line_segment([pt(0.2, 0.42), pt(0.62, 0.42)], ink);
        }
        Pic::Ball => {
            let c = pt(0.5, 0.5);
            let rad = s * 0.48;
            p.circle_filled(c, rad, color);
            p.circle_stroke(c, rad, Stroke::new(thin, darken(color, 0.4)));
            p.circle_filled(pos2(c.x - rad * 0.3, c.y - rad * 0.3), rad * 0.28, lighten(color, 0.7));
        }
        Pic::Bullet => {
            p.rect_filled(fr(0.2, 0.0, 0.8, 1.0), w.min(h) * 0.3, color);
            p.rect_filled(fr(0.38, 0.12, 0.62, 0.88), w.min(h) * 0.15, lighten(color, 0.7));
        }
        Pic::Brick => {
            p.rect_filled(fr(0.03, 0.06, 0.97, 0.94), s * 0.1, color);
            p.rect_filled(fr(0.03, 0.06, 0.97, 0.3), s * 0.1, lighten(color, 0.3));
            p.rect_filled(fr(0.03, 0.78, 0.97, 0.94), s * 0.1, darken(color, 0.2));
            p.rect_stroke(fr(0.03, 0.06, 0.97, 0.94), s * 0.1, Stroke::new(thin, darken(color, 0.45)), StrokeKind::Inside);
        }
        Pic::Paddle => {
            p.rect_filled(fr(0.0, 0.5, 1.0, 1.0), h * 0.25, color);
            p.rect_filled(fr(0.04, 0.54, 0.96, 0.7), h * 0.1, lighten(color, 0.5));
            p.rect_stroke(fr(0.0, 0.5, 1.0, 1.0), h * 0.25, outline, StrokeKind::Inside);
        }
        Pic::Crate => {
            p.rect_filled(fr(0.04, 0.04, 0.96, 0.96), s * 0.05, color);
            let wood = Stroke::new((s * 0.06).max(1.5), darken(color, 0.4));
            p.rect_stroke(fr(0.04, 0.04, 0.96, 0.96), s * 0.05, wood, StrokeKind::Inside);
            p.line_segment([pt(0.1, 0.1), pt(0.9, 0.9)], wood);
            p.line_segment([pt(0.9, 0.1), pt(0.1, 0.9)], wood);
        }
        Pic::Target => {
            let c = pt(0.5, 0.5);
            p.circle_stroke(c, s * 0.34, Stroke::new((s * 0.07).max(1.5), color));
            p.circle_filled(c, s * 0.12, color);
        }
        Pic::Key => {
            let stroke = Stroke::new((s * 0.09).max(1.5), color);
            p.circle_stroke(pt(0.3, 0.35), s * 0.2, stroke);
            p.line_segment([pt(0.42, 0.45), pt(0.88, 0.85)], stroke);
            p.line_segment([pt(0.7, 0.68), pt(0.62, 0.8)], stroke);
            p.line_segment([pt(0.82, 0.78), pt(0.75, 0.9)], stroke);
        }
        Pic::Door => {
            p.rect_filled(fr(0.08, 0.0, 0.92, 1.0), s * 0.06, color);
            p.rect_stroke(fr(0.08, 0.0, 0.92, 1.0), s * 0.06, outline, StrokeKind::Inside);
            p.rect_stroke(fr(0.2, 0.1, 0.8, 0.48), s * 0.04, Stroke::new(thin, darken(color, 0.3)), StrokeKind::Inside);
            p.rect_stroke(fr(0.2, 0.55, 0.8, 0.92), s * 0.04, Stroke::new(thin, darken(color, 0.3)), StrokeKind::Inside);
            p.circle_filled(pt(0.74, 0.52), s * 0.05, Color32::from_rgb(255, 220, 90));
        }
        Pic::Star => {
            let spin = (anim.time * 2.0).sin() * 0.06;
            let c = pt(0.5, 0.53 + spin * 0.3);
            let pts: Vec<Pos2> = (0..10)
                .map(|i| {
                    let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 10.0;
                    let rad = if i % 2 == 0 { s * 0.48 } else { s * 0.2 };
                    pos2(c.x + a.cos() * rad, c.y + a.sin() * rad)
                })
                .collect();
            for i in 0..10 {
                p.add(Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % 10]], color, Stroke::new(1.0, color)));
            }
            p.circle_filled(pos2(c.x - s * 0.05, c.y - s * 0.07), s * 0.06, lighten(color, 0.7));
        }
        Pic::Ship => {
            let flame = 0.9 + (anim.time * 30.0).sin().abs() * 0.1;
            p.add(Shape::convex_polygon(vec![pt(0.38, 0.78), pt(0.62, 0.78), pt(0.5, flame + 0.08)], Color32::from_rgb(255, 170, 60), Stroke::NONE));
            p.add(Shape::convex_polygon(vec![pt(0.5, 0.02), pt(0.92, 0.82), pt(0.5, 0.68)], color, outline));
            p.add(Shape::convex_polygon(vec![pt(0.5, 0.02), pt(0.5, 0.68), pt(0.08, 0.82)], darken(color, 0.15), outline));
            p.circle_filled(pt(0.5, 0.42), s * 0.1, Color32::from_rgb(40, 60, 110));
            p.circle_filled(pt(0.48, 0.4), s * 0.04, Color32::from_rgb(160, 210, 255));
        }
        Pic::Alien => {
            let step = ((anim.time * 4.0) as i32 % 2) as f32 * 0.06;
            p.rect_filled(fr(0.1, 0.22, 0.9, 0.72), s * 0.22, color);
            p.rect_stroke(fr(0.1, 0.22, 0.9, 0.72), s * 0.22, outline, StrokeKind::Inside);
            for ex in [0.32, 0.68] {
                p.circle_filled(pt(ex, 0.44), s * 0.1, Color32::WHITE);
                p.circle_filled(pt(ex, 0.46), s * 0.05, dark);
            }
            let ant = Stroke::new((s * 0.05).max(1.0), darken(color, 0.4));
            p.line_segment([pt(0.3, 0.24), pt(0.2, 0.08)], ant);
            p.line_segment([pt(0.7, 0.24), pt(0.8, 0.08)], ant);
            for lx in [0.22, 0.42, 0.62, 0.82] {
                p.rect_filled(fr(lx - 0.05, 0.72, lx + 0.05, 0.9 + if lx < 0.5 { step } else { -step }), 1.0, darken(color, 0.2));
            }
        }
        Pic::Bomb => {
            let c = pt(0.5, 0.58);
            p.circle_filled(c, s * 0.4, color);
            p.circle_stroke(c, s * 0.4, Stroke::new(thin, lighten(color, 0.3)));
            p.circle_filled(pos2(c.x - s * 0.12, c.y - s * 0.12), s * 0.08, lighten(color, 0.6));
            p.line_segment([pt(0.6, 0.22), pt(0.7, 0.08)], Stroke::new((s * 0.06).max(1.0), Color32::from_rgb(170, 130, 90)));
            let spark = 0.5 + (anim.time * 20.0).sin().abs() * 0.5;
            p.circle_filled(pt(0.72, 0.06), s * 0.07 * spark + 1.0, Color32::from_rgb(255, 200, 60));
        }
        Pic::Rock => {
            let pts = vec![pt(0.12, 0.8), pt(0.06, 0.5), pt(0.28, 0.2), pt(0.62, 0.12), pt(0.9, 0.38), pt(0.94, 0.78), pt(0.6, 0.94)];
            p.add(Shape::convex_polygon(pts, color, outline));
            p.add(Shape::convex_polygon(vec![pt(0.28, 0.22), pt(0.6, 0.15), pt(0.5, 0.4), pt(0.24, 0.45)], lighten(color, 0.3), Stroke::NONE));
        }
        Pic::Heart => draw_heart(p, pt(0.5, 0.5), s * 0.9, color),
        Pic::Plain => {
            p.rect_filled(fr(0.05, 0.05, 0.95, 0.95), s * 0.12, color);
            p.rect_stroke(fr(0.05, 0.05, 0.95, 0.95), s * 0.12, outline, StrokeKind::Inside);
        }
    }
}

/// Small arrows that show how an object moves. Only used in the editor.
pub fn draw_motion_badge(p: &Painter, r: Rect, behavior: Behavior, drift: Dir) {
    let (w, h) = (r.width(), r.height());
    let pt = |x: f32, y: f32| pos2(r.min.x + x * w, r.min.y + y * h);
    let stroke = Stroke::new((w.min(h) * 0.07).max(1.5), Color32::from_rgba_unmultiplied(255, 255, 255, 230));
    let shadow = Stroke::new((w.min(h) * 0.07).max(1.5) + 2.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
    let mut lines: Vec<Vec<Pos2>> = Vec::new();
    let horizontal = |lines: &mut Vec<Vec<Pos2>>| {
        lines.push(vec![pt(0.1, 0.5), pt(0.9, 0.5)]);
        lines.push(vec![pt(0.28, 0.36), pt(0.1, 0.5), pt(0.28, 0.64)]);
        lines.push(vec![pt(0.72, 0.36), pt(0.9, 0.5), pt(0.72, 0.64)]);
    };
    let vertical = |lines: &mut Vec<Vec<Pos2>>| {
        lines.push(vec![pt(0.5, 0.1), pt(0.5, 0.9)]);
        lines.push(vec![pt(0.36, 0.28), pt(0.5, 0.1), pt(0.64, 0.28)]);
        lines.push(vec![pt(0.36, 0.72), pt(0.5, 0.9), pt(0.64, 0.72)]);
    };
    match behavior {
        Behavior::PatrolH => horizontal(&mut lines),
        Behavior::PatrolV => vertical(&mut lines),
        Behavior::Drift => match drift {
            Dir::Left | Dir::Right => horizontal(&mut lines),
            _ => vertical(&mut lines),
        },
        _ => return,
    }
    for l in &lines {
        p.add(Shape::line(l.clone(), shadow));
    }
    for l in lines {
        p.add(Shape::line(l, stroke));
    }
}

pub fn draw_heart(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let r = size * 0.27;
    p.circle_filled(pos2(c.x - size * 0.23, c.y - size * 0.12), r, color);
    p.circle_filled(pos2(c.x + size * 0.23, c.y - size * 0.12), r, color);
    p.add(Shape::convex_polygon(
        vec![pos2(c.x - size * 0.47, c.y - size * 0.02), pos2(c.x + size * 0.47, c.y - size * 0.02), pos2(c.x, c.y + size * 0.5)],
        color,
        Stroke::NONE,
    ));
}

// ---------------------------------------------------------------------------
// The running game
// ---------------------------------------------------------------------------

/// Where the level is drawn on screen.
#[derive(Clone, Copy)]
pub struct View {
    pub origin: Pos2,
    pub cell: f32,
}

impl View {
    /// Fit a level of `w` by `h` cells inside `area`, centered.
    pub fn fit(area: Rect, w: i32, h: i32) -> View {
        let mut cell = (area.width() / w as f32).min(area.height() / h as f32).max(4.0);
        if cell > 12.0 {
            cell = cell.floor();
        }
        let size = vec2(cell * w as f32, cell * h as f32);
        View { origin: area.center() - size / 2.0, cell }
    }

    pub fn cell_rect(&self, x: f32, y: f32) -> Rect {
        Rect::from_min_size(self.origin + vec2(x, y) * self.cell, vec2(self.cell, self.cell))
    }

    /// The rectangle of something `w` by `h` squares big whose top-left is at (x, y).
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.origin + vec2(x, y) * self.cell, vec2(w, h) * self.cell)
    }

    pub fn level_rect(&self, w: i32, h: i32) -> Rect {
        Rect::from_min_size(self.origin, vec2(w as f32, h as f32) * self.cell)
    }
}

fn clouds(p: &Painter, v: &View, level: Rect, w: i32, h: i32, time: f32, sky: Color32) {
    let p = p.with_clip_rect(level);
    let white = lighten(sky, 0.55);
    let count = (w / 7).max(2);
    for i in 0..count {
        let seed = i as f32;
        let speed = 0.25 + (i % 3) as f32 * 0.1;
        let x = ((seed * 7.3 + 2.0 + time * speed) % (w as f32 + 6.0)) - 3.0;
        let y = 0.8 + ((i * 5) % 4) as f32 * (h as f32 * 0.12);
        let c = v.origin + vec2(x, y) * v.cell;
        let u = v.cell;
        p.circle_filled(c, u * 0.7, white);
        p.circle_filled(c + vec2(u * 0.8, u * 0.1), u * 0.9, white);
        p.circle_filled(c + vec2(u * 1.7, u * 0.2), u * 0.6, white);
        p.rect_filled(Rect::from_min_max(c + vec2(-0.3 * u, u * 0.3), c + vec2(u * 2.1, u * 1.0)), u * 0.45, white);
    }
}

/// Twinkling stars for dark (night) skies.
fn stars(p: &Painter, v: &View, level: Rect, w: i32, h: i32, time: f32) {
    let p = p.with_clip_rect(level);
    let count = (w * h / 5).clamp(20, 300);
    let mut seed = 0x9e37_79b9u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for i in 0..count {
        let (x, y, phase) = (next() * w as f32, next() * h as f32, next() * 6.0);
        let twinkle = 0.45 + 0.55 * (time * 1.5 + phase + i as f32).sin().abs();
        let c = Color32::from_rgba_unmultiplied(255, 255, 255, (twinkle * 200.0) as u8);
        p.circle_filled(v.origin + vec2(x, y) * v.cell, (v.cell * 0.05).max(1.0) * (0.7 + twinkle * 0.6), c);
    }
}

fn is_dark(c: Rgb) -> bool {
    (c[0] as u32 * 3 + c[1] as u32 * 6 + c[2] as u32) / 10 < 110
}

/// Draw the running game, including the score bar at the top.
/// `exit_hint` is shown at the right of the bar (for example "Esc = stop").
pub fn draw_game(p: &Painter, area: Rect, rt: &Runtime, exit_hint: &str) {
    p.rect_filled(area, 0.0, Color32::from_rgb(28, 26, 44));
    let hud_h = 40.0;
    let play_area = Rect::from_min_max(area.min + vec2(0.0, hud_h), area.max).shrink(8.0);
    let v = View::fit(play_area, rt.width, rt.height);
    let level_rect = v.level_rect(rt.width, rt.height);

    // Sky and clouds.
    p.rect_filled(level_rect, 4.0, rgb(rt.sky));
    if is_dark(rt.sky) {
        stars(p, &v, level_rect, rt.width, rt.height, rt.time);
    } else {
        clouds(p, &v, level_rect, rt.width, rt.height, rt.time, rgb(rt.sky));
    }
    let clip = p.with_clip_rect(level_rect);

    // Back to front: layer 0, then 1, then 2.
    for layer in 0..3u8 {
        for o in &rt.objs {
            let t = &rt.game.types[o.ty];
            if !o.alive || t.layer.min(2) != layer {
                continue;
            }
            if o.behavior.controlled() {
                if rt.state == State::GameOver {
                    continue;
                }
                if rt.invulnerable > 0.0 && ((rt.time * 14.0) as i32) % 2 == 0 {
                    continue;
                }
            }
            let facing = if o.behavior == Behavior::PatrolH { o.dir() } else { o.face.x };
            let anim = Anim { time: rt.time, facing, squash: o.squash, ghost: o.behavior == Behavior::Follow };
            let r = v.rect(o.pos.x, o.pos.y, o.size.x, o.size.y);
            draw_type(&clip, t, r, rgb(o.color), anim);
        }
    }

    for q in &rt.particles {
        let a = (q.life / q.max_life).clamp(0.0, 1.0);
        let col = Color32::from_rgba_unmultiplied(q.color[0], q.color[1], q.color[2], (a * 255.0) as u8);
        clip.circle_filled(v.origin + vec2(q.pos.x, q.pos.y) * v.cell, q.size * v.cell * (0.5 + a * 0.5), col);
    }

    // Speech bubble over the player (from signs).
    if let Some(text) = &rt.sign_text {
        if let Some(po) = rt.objs.iter().find(|o| o.alive && o.behavior.controlled()) {
            let font = FontId::proportional((v.cell * 0.55).clamp(14.0, 26.0));
            let galley = clip.layout(text.clone(), font, Color32::from_rgb(40, 30, 30), (v.cell * 9.0).max(160.0));
            let size = galley.size();
            let anchor = v.origin + vec2(po.pos.x + po.size.x / 2.0, po.pos.y) * v.cell;
            let mut min = pos2(anchor.x - size.x / 2.0 - 10.0, anchor.y - size.y - 26.0);
            min.x = min.x.clamp(level_rect.min.x + 4.0, (level_rect.max.x - size.x - 24.0).max(level_rect.min.x + 4.0));
            min.y = min.y.max(level_rect.min.y + 4.0);
            let bubble = Rect::from_min_size(min, size + vec2(20.0, 12.0));
            clip.rect_filled(bubble.translate(vec2(2.0, 3.0)), 10.0, Color32::from_rgba_unmultiplied(0, 0, 0, 50));
            clip.rect_filled(bubble, 10.0, Color32::from_rgb(255, 252, 240));
            clip.rect_stroke(bubble, 10.0, Stroke::new(2.0, Color32::from_rgb(120, 100, 80)), StrokeKind::Inside);
            clip.galley(bubble.min + vec2(10.0, 6.0), galley, Color32::BLACK);
        }
    }

    // Messages from "say something" actions.
    if let Some((text, _)) = &rt.banner {
        let galley = clip.layout(text.clone(), FontId::proportional(26.0), Color32::WHITE, level_rect.width() * 0.8);
        let size = galley.size();
        let b = Rect::from_center_size(pos2(level_rect.center().x, level_rect.min.y + 50.0 + size.y / 2.0), size + vec2(36.0, 20.0));
        clip.rect_filled(b, 14.0, Color32::from_rgba_unmultiplied(20, 16, 50, 220));
        clip.galley(b.min + vec2(18.0, 10.0), galley, Color32::WHITE);
    }

    draw_hud(p, area, hud_h, rt, exit_hint);

    match rt.state {
        State::LevelDone(_) => banner(p, level_rect, "Level complete!", "", Color32::from_rgb(90, 210, 120)),
        State::Won => banner(
            p,
            level_rect,
            "YOU WIN!",
            &format!("{}\nScore: {}\n\nPress R to play again", rt.game.win_message, rt.score),
            Color32::from_rgb(255, 214, 80),
        ),
        State::GameOver => banner(p, level_rect, "Game over", &format!("Score: {}\n\nPress R to try again", rt.score), Color32::from_rgb(255, 110, 120)),
        State::Playing => {}
    }
}

fn draw_hud(p: &Painter, area: Rect, hud_h: f32, rt: &Runtime, exit_hint: &str) {
    let bar = Rect::from_min_size(area.min, vec2(area.width(), hud_h));
    p.rect_filled(bar, 0.0, Color32::from_rgb(40, 36, 64));
    let mid = bar.center().y;
    let mut x = bar.min.x + 20.0;
    for i in 0..rt.game.lives.max(rt.lives).min(10) {
        let col = if i < rt.lives { Color32::from_rgb(255, 80, 100) } else { Color32::from_rgb(90, 84, 120) };
        draw_heart(p, pos2(x, mid), 22.0, col);
        x += 26.0;
    }
    x += 16.0;
    let font = FontId::proportional(20.0);
    let r = p.text(pos2(x, mid), Align2::LEFT_CENTER, format!("Score: {}", rt.score), font.clone(), Color32::WHITE);
    x = r.max.x + 22.0;
    for (i, v) in rt.game.vars.iter().enumerate() {
        if v.show {
            let r = p.text(
                pos2(x, mid),
                Align2::LEFT_CENTER,
                format!("{}: {}", v.name, rt.vars.get(i).copied().unwrap_or(0)),
                font.clone(),
                Color32::from_rgb(160, 230, 255),
            );
            x = r.max.x + 22.0;
        }
    }
    for t in rt.game.types.iter().filter(|t| t.show_count) {
        let n = rt.count_left(t.id);
        if n > 0 {
            let r = p.text(pos2(x, mid), Align2::LEFT_CENTER, format!("{} left: {n}", t.name), font.clone(), Color32::from_rgb(255, 214, 80));
            x = r.max.x + 22.0;
        }
    }
    let level_name = format!("Level {}/{}: {}", rt.level + 1, rt.game.levels.len(), rt.game.levels[rt.level].name);
    if x < bar.center().x - 60.0 {
        p.text(bar.center() + vec2(60.0, 0.0), Align2::CENTER_CENTER, level_name, FontId::proportional(17.0), Color32::from_rgb(200, 195, 230));
    }
    p.text(pos2(bar.max.x - 16.0, mid), Align2::RIGHT_CENTER, exit_hint, FontId::proportional(16.0), Color32::from_rgb(170, 165, 210));
}

fn banner(p: &Painter, level: Rect, title: &str, subtitle: &str, color: Color32) {
    let strip_h: f32 = if subtitle.is_empty() { 110.0 } else { 220.0 };
    let strip = Rect::from_center_size(level.center(), vec2(level.width(), strip_h.min(level.height())));
    p.rect_filled(level, 4.0, Color32::from_rgba_unmultiplied(10, 8, 30, 90));
    p.rect_filled(strip, 0.0, Color32::from_rgba_unmultiplied(20, 16, 50, 225));
    let title_y = if subtitle.is_empty() { strip.center().y } else { strip.min.y + 50.0 };
    p.text(pos2(strip.center().x, title_y), Align2::CENTER_CENTER, title, FontId::proportional(54.0), color);
    if !subtitle.is_empty() {
        p.text(pos2(strip.center().x, strip.min.y + 150.0), Align2::CENTER_CENTER, subtitle, FontId::proportional(22.0), Color32::WHITE);
    }
}
