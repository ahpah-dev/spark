//! The picture drawer: make your own pixel art for any object, with animation frames.

use super::*;
use egui::ColorImage;

#[derive(Clone, Copy, PartialEq)]
enum PixelTool {
    Pen,
    Eraser,
    Fill,
    Pick,
}

pub struct SpriteEditor {
    pub ty: TypeId,
    frame: usize,
    color: [u8; 4],
    tool: PixelTool,
    mirror: bool,
    painting: bool,
    /// A blank drawing for an object that still uses a built-in picture. The object keeps its
    /// picture until the first pixel is drawn.
    draft: Option<Sprite>,
}

impl SpriteEditor {
    pub fn new(ty: TypeId, draft: Option<Sprite>) -> SpriteEditor {
        SpriteEditor { ty, frame: 0, color: [41, 173, 255, 255], tool: PixelTool::Pen, mirror: false, painting: false, draft }
    }
}

/// 16 friendly colors to draw with.
const PALETTE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [29, 43, 83],
    [126, 37, 83],
    [0, 135, 81],
    [171, 82, 54],
    [95, 87, 79],
    [194, 195, 199],
    [255, 241, 232],
    [255, 0, 77],
    [255, 163, 0],
    [255, 236, 39],
    [0, 228, 54],
    [41, 173, 255],
    [131, 118, 156],
    [255, 119, 168],
    [255, 204, 170],
];

// ---- pixel operations (these are plain functions so they are easy to test) ----

/// Paint every connected pixel of the same color with `color`.
pub fn flood_fill(s: &mut Sprite, frame: usize, x: usize, y: usize, color: [u8; 4]) {
    if x >= s.w || y >= s.h {
        return;
    }
    let target = s.get(frame, x, y);
    if target == color {
        return;
    }
    let mut stack = vec![(x, y)];
    while let Some((cx, cy)) = stack.pop() {
        if s.get(frame, cx, cy) != target {
            continue;
        }
        s.set(frame, cx, cy, color);
        if cx > 0 {
            stack.push((cx - 1, cy));
        }
        if cy > 0 {
            stack.push((cx, cy - 1));
        }
        if cx + 1 < s.w {
            stack.push((cx + 1, cy));
        }
        if cy + 1 < s.h {
            stack.push((cx, cy + 1));
        }
    }
}

/// Slide the whole picture (it wraps around the edges).
pub fn shift(s: &mut Sprite, frame: usize, dx: i32, dy: i32) {
    let old = s.frames[frame].clone();
    for y in 0..s.h {
        for x in 0..s.w {
            let nx = (x as i32 + dx).rem_euclid(s.w as i32) as usize;
            let ny = (y as i32 + dy).rem_euclid(s.h as i32) as usize;
            s.frames[frame][ny * s.w + nx] = old[y * s.w + x];
        }
    }
}

pub fn flip_horizontal(s: &mut Sprite, frame: usize) {
    for y in 0..s.h {
        s.frames[frame][y * s.w..(y + 1) * s.w].reverse();
    }
}

/// Make the picture a different size (every pixel is copied to the nearest new pixel).
pub fn resized(s: &Sprite, nw: usize, nh: usize) -> Sprite {
    let mut out = Sprite::new(nw, nh);
    out.fps = s.fps;
    out.frames = s
        .frames
        .iter()
        .map(|f| {
            let mut nf = vec![[0, 0, 0, 0]; nw * nh];
            for y in 0..nh {
                for x in 0..nw {
                    nf[y * nw + x] = f[(y * s.h / nh) * s.w + x * s.w / nw];
                }
            }
            nf
        })
        .collect();
    out.refresh();
    out
}


// ---- importing pictures ----

/// The biggest picture we keep (bigger pictures are shrunk to fit).
pub const MAX_IMPORT: u32 = 32;

/// Turn the bytes of a PNG picture into a sprite. Pictures bigger than 32 pixels are shrunk to fit;
/// small pixel art is kept exactly as it is.
pub fn sprite_from_image_bytes(bytes: &[u8]) -> Result<Sprite, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("That picture could not be opened ({e}). Spark can read PNG pictures."))?.to_rgba8();
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err("That picture is empty.".to_string());
    }
    let img = if w.max(h) > MAX_IMPORT {
        let scale = MAX_IMPORT as f32 / w.max(h) as f32;
        let (nw, nh) = (((w as f32 * scale).round() as u32).max(1), ((h as f32 * scale).round() as u32).max(1));
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let (w, h) = img.dimensions();
    let mut s = Sprite::new(w as usize, h as usize);
    s.frames[0] = img.pixels().map(|p| p.0).collect();
    s.refresh();
    Ok(s)
}

/// Fit the first frame of `src` into a `nw` by `nh` canvas (keeping its shape, sitting at the bottom middle).
pub fn fit_frame(src: &Sprite, nw: usize, nh: usize) -> Vec<[u8; 4]> {
    let scale = (nw as f32 / src.w as f32).min(nh as f32 / src.h as f32);
    let (tw, th) = (((src.w as f32 * scale).floor() as usize).clamp(1, nw), ((src.h as f32 * scale).floor() as usize).clamp(1, nh));
    let (ox, oy) = ((nw - tw) / 2, nh - th);
    let mut out = vec![[0, 0, 0, 0]; nw * nh];
    for y in 0..th {
        for x in 0..tw {
            out[(oy + y) * nw + ox + x] = src.frames[0][(y * src.h / th) * src.w + x * src.w / tw];
        }
    }
    out
}

impl Editor {
    /// Open the picture drawer for an object. If it still uses a built-in picture, we start a blank
    /// drawing, and the object keeps its picture until you draw the first pixel.
    pub(super) fn open_sprite_editor(&mut self, ty: TypeId) {
        let draft = match self.game.ty(ty).map(|t| &t.art) {
            Some(Art::Built(_)) => Some(Sprite::new(16, 16)),
            _ => None,
        };
        self.sprite_ed = Some(SpriteEditor::new(ty, draft));
    }

    /// Use an imported picture for an object: it replaces the drawing, or (`as_frame`) is added as a new animation frame.
    pub(super) fn import_sprite(&mut self, ty: TypeId, imported: Sprite, as_frame: bool) {
        let mut ed = self.sprite_ed.take().filter(|e| e.ty == ty).unwrap_or_else(|| SpriteEditor::new(ty, None));
        let current = match self.game.ty(ty).map(|t| &t.art) {
            Some(Art::Pixels(s)) => Some(s.clone()),
            _ => None,
        };
        let new = match (as_frame, current) {
            (true, Some(mut cur)) if cur.frames.len() < 8 => {
                let frame = fit_frame(&imported, cur.w, cur.h);
                cur.frames.push(frame);
                ed.frame = cur.frames.len() - 1;
                cur
            }
            _ => {
                ed.frame = 0;
                imported
            }
        };
        self.apply_sprite_change(&mut ed, new, true);
        self.sprite_ed = Some(ed);
    }

    /// Read a picture file and use it for an object.
    pub(super) fn import_png_file(&mut self, ctx: &egui::Context, ty: TypeId, path: &Path, as_frame: bool) {
        let result = fs::read(path).map_err(|e| format!("Could not open the file: {e}")).and_then(|bytes| sprite_from_image_bytes(&bytes));
        match result {
            Ok(s) => {
                let name = self.game.type_name(ty);
                let (w, h) = (s.w, s.h);
                self.import_sprite(ty, s, as_frame);
                self.say(ctx, format!("Imported the picture ({w} by {h} pixels) for '{name}'. You can keep drawing on it!"));
            }
            Err(e) => self.say(ctx, e),
        }
    }

    /// Ask for a picture file with the computer's own file window.
    pub(super) fn import_dialog(&mut self, ctx: &egui::Context, ty: TypeId, as_frame: bool) {
        let picked = rfd::FileDialog::new().set_title("Pick a PNG picture").add_filter("PNG pictures", &["png"]).pick_file();
        if let Some(path) = picked {
            self.import_png_file(ctx, ty, &path, as_frame);
        }
    }

    /// Something was dragged onto the window: a PNG becomes the picture of the object you are working on,
    /// and a .spark file is opened as a game.
    pub(super) fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<egui::DroppedFileHandle> = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let path = f.path().to_path_buf();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            match ext.as_str() {
                "png" => match self.sprite_ed.as_ref().map(|e| e.ty).or_else(|| self.focus_type()) {
                    Some(ty) => {
                        self.tab = Tab::Object;
                        self.import_png_file(ctx, ty, &path, false);
                    }
                    None => self.say(ctx, "Pick an object on the left first, then drop the picture on the window."),
                },
                "spark" => self.request(ctx, Pending::Open(path)),
                _ => self.say(ctx, "Spark can use PNG pictures and .spark games."),
            }
        }
    }

    /// A hint while a file is being dragged over the window.
    pub(super) fn drop_hint(&self, ctx: &egui::Context) {
        if ctx.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let name = self.sprite_ed.as_ref().map(|e| e.ty).or_else(|| self.focus_type()).map(|t| self.game.type_name(t));
        let text = match name {
            Some(n) => format!("Drop it here to use it as the picture for '{n}'"),
            None => "Pick an object first, then drop the picture".to_string(),
        };
        let screen = ctx.content_rect();
        let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("drop_hint")));
        p.rect_filled(screen, 0.0, Color32::from_rgba_unmultiplied(110, 76, 230, 150));
        p.text(screen.center(), Align2::CENTER_CENTER, text, FontId::proportional(32.0), Color32::WHITE);
    }

    /// Save the drawing into the object. A drawing for a built-in picture only replaces it once it has some pixels.
    fn apply_sprite_change(&mut self, ed: &mut SpriteEditor, mut sprite: Sprite, snapshot_needed: bool) {
        sprite.refresh();
        let built = self.game.ty(ed.ty).is_some_and(|t| matches!(t.art, Art::Built(_)));
        let has_pixels = sprite.frames.iter().any(|f| f.iter().any(|p| p[3] > 0));
        if built && !has_pixels {
            ed.draft = Some(sprite);
            return;
        }
        if snapshot_needed || built {
            self.snapshot();
        }
        if let Some(t) = self.game.ty_mut(ed.ty) {
            t.art = Art::Pixels(sprite);
        }
        ed.draft = None;
        self.dirty = true;
    }

    pub(super) fn sprite_window(&mut self, ctx: &egui::Context) {
        let Some(mut ed) = self.sprite_ed.take() else { return };
        let Some(t) = self.game.ty(ed.ty) else { return };
        let name = t.name.clone();
        let built_type = if matches!(t.art, Art::Built(_)) { Some(t.clone()) } else { None };
        let mut sprite = match &t.art {
            Art::Pixels(s) => s.clone(),
            Art::Built(_) => ed.draft.clone().unwrap_or_else(|| Sprite::new(16, 16)),
        };
        let original = sprite.clone();
        ed.frame = ed.frame.min(sprite.frames.len() - 1);

        let mut open = true;
        let mut done = false;
        let mut snapshot_needed = false;
        let mut import_request: Option<bool> = None;
        let ty = ed.ty;
        let time = ctx.input(|i| i.time) as f32;

        egui::Window::new(RichText::new(format!("Draw: {name}")).size(20.0).strong())
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_pos(pos2(260.0, 90.0))
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    // ---- the drawing area ----
                    ui.vertical(|ui| {
                        let cell = (400.0 / sprite.w.max(sprite.h) as f32).floor().max(6.0);
                        let size = vec2(sprite.w as f32 * cell, sprite.h as f32 * cell);
                        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
                        let rect = resp.rect;
                        for y in 0..sprite.h {
                            for x in 0..sprite.w {
                                let r = Rect::from_min_size(rect.min + vec2(x as f32, y as f32) * cell, vec2(cell, cell));
                                let check = if (x + y) % 2 == 0 { Color32::from_rgb(214, 210, 232) } else { Color32::from_rgb(236, 233, 247) };
                                painter.rect_filled(r, 0.0, check);
                                let p = sprite.get(ed.frame, x, y);
                                if p[3] > 0 {
                                    painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]));
                                }
                            }
                        }
                        if let Some(bt) = &built_type {
                            // The picture it has now, faintly, to trace over.
                            let mut ghost = painter.clone();
                            ghost.set_opacity(0.35);
                            render::draw_type(&ghost, bt, rect, render::rgb(bt.color), Anim::default());
                        }
                        let grid = Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 60, 30));
                        for i in 0..=sprite.w {
                            let x = rect.min.x + i as f32 * cell;
                            painter.line_segment([pos2(x, rect.min.y), pos2(x, rect.max.y)], grid);
                        }
                        for i in 0..=sprite.h {
                            let y = rect.min.y + i as f32 * cell;
                            painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], grid);
                        }
                        painter.rect_stroke(rect, 0.0, Stroke::new(2.0, style::INK), StrokeKind::Outside);

                        let down = ui.input(|i| i.pointer.primary_down());
                        let pressed = ui.input(|i| i.pointer.primary_pressed());
                        let pos = ui.input(|i| i.pointer.latest_pos());
                        if !down {
                            ed.painting = false;
                        }
                        if pressed && resp.hovered() {
                            ed.painting = true;
                            snapshot_needed = true;
                        }
                        if ed.painting {
                            if let Some(pos) = pos.filter(|p| rect.contains(*p)) {
                                let c = (pos - rect.min) / cell;
                                let (x, y) = ((c.x as usize).min(sprite.w - 1), (c.y as usize).min(sprite.h - 1));
                                let mut spots = vec![(x, y)];
                                if ed.mirror {
                                    spots.push((sprite.w - 1 - x, y));
                                }
                                match ed.tool {
                                    PixelTool::Pen => {
                                        for (sx, sy) in spots {
                                            sprite.set(ed.frame, sx, sy, ed.color);
                                        }
                                    }
                                    PixelTool::Eraser => {
                                        for (sx, sy) in spots {
                                            sprite.set(ed.frame, sx, sy, [0, 0, 0, 0]);
                                        }
                                    }
                                    PixelTool::Fill => {
                                        if pressed {
                                            flood_fill(&mut sprite, ed.frame, x, y, ed.color);
                                        }
                                    }
                                    PixelTool::Pick => {
                                        let p = sprite.get(ed.frame, x, y);
                                        if p[3] > 0 {
                                            ed.color = p;
                                        }
                                        ed.tool = PixelTool::Pen;
                                    }
                                }
                            }
                        }
                        if resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new("Click and drag to draw. Transparent pixels show what is behind.").small().color(Color32::from_rgb(110, 100, 150)));
                    });

                    // ---- the tools ----
                    ui.vertical(|ui| {
                        ui.set_width(210.0);
                        ui.label(RichText::new("Tool").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.selectable_value(&mut ed.tool, PixelTool::Pen, "Pen");
                            ui.selectable_value(&mut ed.tool, PixelTool::Eraser, "Eraser");
                            ui.selectable_value(&mut ed.tool, PixelTool::Fill, "Fill");
                            ui.selectable_value(&mut ed.tool, PixelTool::Pick, "Pick color");
                        });
                        ui.checkbox(&mut ed.mirror, "Mirror (draw both sides)");

                        ui.add_space(4.0);
                        ui.label(RichText::new("Color").strong());
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                            for c in PALETTE {
                                let (r, resp) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::click());
                                ui.painter().rect_filled(r, 4.0, render::rgb(c));
                                let selected = ed.color[..3] == c;
                                ui.painter().rect_stroke(r, 4.0, Stroke::new(if selected { 3.0 } else { 1.0 }, if selected { Color32::WHITE } else { Color32::from_gray(90) }), StrokeKind::Inside);
                                if selected {
                                    ui.painter().rect_stroke(r.expand(1.0), 4.0, Stroke::new(1.5, style::PURPLE), StrokeKind::Outside);
                                }
                                if resp.clicked() {
                                    ed.color = [c[0], c[1], c[2], 255];
                                    if ed.tool == PixelTool::Eraser || ed.tool == PixelTool::Pick {
                                        ed.tool = PixelTool::Pen;
                                    }
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Any color:");
                            let mut rgb = [ed.color[0], ed.color[1], ed.color[2]];
                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                ed.color = [rgb[0], rgb[1], rgb[2], 255];
                                ed.tool = PixelTool::Pen;
                            }
                        });

                        ui.add_space(4.0);
                        ui.label(RichText::new("Animation frames").strong());
                        ui.horizontal_wrapped(|ui| {
                            for i in 0..sprite.frames.len() {
                                ui.selectable_value(&mut ed.frame, i, format!("{}", i + 1));
                            }
                            if sprite.frames.len() < 8 && ui.small_button("+ copy").on_hover_text("Add a frame that starts as a copy of this one").clicked() {
                                snapshot_needed = true;
                                let copy = sprite.frames[ed.frame].clone();
                                sprite.frames.insert(ed.frame + 1, copy);
                                ed.frame += 1;
                            }
                            if sprite.frames.len() > 1 && ui.small_button("delete").clicked() {
                                snapshot_needed = true;
                                sprite.frames.remove(ed.frame);
                                ed.frame = ed.frame.min(sprite.frames.len() - 1);
                            }
                        });
                        if sprite.frames.len() > 1 {
                            ui.add(egui::Slider::new(&mut sprite.fps, 1.0..=20.0).text("frames/sec"));
                            ctx.request_repaint();
                        }

                        ui.add_space(4.0);
                        ui.label(RichText::new("Change the picture").strong());
                        ui.horizontal_wrapped(|ui| {
                            let f = ed.frame;
                            let mut op = |ui: &mut Ui, label: &str, apply: &mut dyn FnMut(&mut Sprite)| {
                                if ui.small_button(label).clicked() {
                                    snapshot_needed = true;
                                    apply(&mut sprite);
                                }
                            };
                            op(ui, "Left", &mut |s| shift(s, f, -1, 0));
                            op(ui, "Right", &mut |s| shift(s, f, 1, 0));
                            op(ui, "Up", &mut |s| shift(s, f, 0, -1));
                            op(ui, "Down", &mut |s| shift(s, f, 0, 1));
                            op(ui, "Flip", &mut |s| flip_horizontal(s, f));
                            op(ui, "Clear", &mut |s| s.frames[f].iter_mut().for_each(|p| *p = [0, 0, 0, 0]));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Size:");
                            for n in [8usize, 16, 24, 32] {
                                if ui.selectable_label(sprite.w == n && sprite.h == n, format!("{n}")).clicked() && !(sprite.w == n && sprite.h == n) {
                                    snapshot_needed = true;
                                    sprite = resized(&sprite, n, n);
                                }
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Import PNG...").on_hover_text("Use a PNG picture from your computer (you can also drag one onto this window)").clicked() {
                                import_request = Some(false);
                            }
                            if ui.button("Add as a frame").on_hover_text("Add a PNG as another animation frame").clicked() {
                                import_request = Some(true);
                            }
                        });

                        ui.add_space(6.0);
                        ui.label(RichText::new("Preview").strong());
                        let (r, _) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
                        ui.painter().rect_filled(r, 8.0, Color32::from_rgb(150, 210, 255));
                        render::draw_pixels(ui.painter(), &sprite, r.shrink(8.0), Anim { time, ..Anim::default() });

                        ui.add_space(6.0);
                        if ui.add(Button::new(RichText::new("Done").size(18.0).strong().color(Color32::WHITE)).fill(style::PURPLE).min_size(vec2(120.0, 36.0))).clicked() {
                            done = true;
                        }
                    });
                });
            });

        if sprite != original {
            self.apply_sprite_change(&mut ed, sprite, snapshot_needed);
        }
        if open && !done {
            self.sprite_ed = Some(ed);
        }
        if let Some(as_frame) = import_request {
            self.import_dialog(ctx, ty, as_frame);
        }
    }
}

/// Used by tests: turn a sprite frame into a picture the graphics card can use.
#[allow(dead_code)]
pub fn frame_image(s: &Sprite, frame: usize) -> ColorImage {
    let bytes: Vec<u8> = s.frames[frame].iter().flat_map(|p| p.iter().copied()).collect();
    ColorImage::from_rgba_unmultiplied([s.w, s.h], &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_stays_inside_the_shape() {
        let mut s = Sprite::new(8, 8);
        let red = [255, 0, 0, 255];
        for i in 0..8 {
            s.set(0, 4, i, [0, 0, 0, 255]); // a wall down the middle
        }
        flood_fill(&mut s, 0, 0, 0, red);
        assert_eq!(s.get(0, 3, 7), red);
        assert_eq!(s.get(0, 5, 0), [0, 0, 0, 0], "the other side must not be filled");
        assert_eq!(s.get(0, 4, 3), [0, 0, 0, 255]);
    }

    #[test]
    fn shifting_wraps_around() {
        let mut s = Sprite::new(8, 8);
        s.set(0, 7, 0, [1, 2, 3, 255]);
        shift(&mut s, 0, 1, 0);
        assert_eq!(s.get(0, 0, 0), [1, 2, 3, 255]);
        assert_eq!(s.get(0, 7, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn flipping_mirrors_the_picture() {
        let mut s = Sprite::new(8, 8);
        s.set(0, 0, 3, [9, 9, 9, 255]);
        flip_horizontal(&mut s, 0);
        assert_eq!(s.get(0, 7, 3), [9, 9, 9, 255]);
    }

    #[test]
    fn resizing_keeps_the_look_and_frames() {
        let mut s = Sprite::new(8, 8);
        s.frames.push(vec![[5, 5, 5, 255]; 64]);
        s.set(0, 0, 0, [7, 7, 7, 255]);
        let big = resized(&s, 16, 16);
        assert_eq!((big.w, big.h, big.frames.len()), (16, 16, 2));
        assert_eq!(big.get(0, 1, 1), [7, 7, 7, 255]);
        assert_eq!(big.get(1, 15, 15), [5, 5, 5, 255]);
    }

    #[test]
    fn a_drawing_only_replaces_the_built_in_picture_after_the_first_pixel() {
        let mut ed = Editor::with_game(crate::examples::space_blaster());
        let ship = ed.game.types[0].id;
        assert!(matches!(ed.game.ty(ship).unwrap().art, Art::Built(_)));
        ed.open_sprite_editor(ship);
        let mut state = ed.sprite_ed.take().unwrap();
        assert!(state.draft.is_some());

        // Changing the size of the blank drawing must not make the ship disappear.
        let blank = resized(state.draft.as_ref().unwrap(), 24, 24);
        ed.apply_sprite_change(&mut state, blank, true);
        assert!(matches!(ed.game.ty(ship).unwrap().art, Art::Built(_)), "still the built-in picture");
        assert_eq!(state.draft.as_ref().unwrap().w, 24);

        // The first pixel turns it into your own picture.
        let mut painted = state.draft.clone().unwrap();
        painted.set(0, 3, 3, [255, 0, 0, 255]);
        ed.apply_sprite_change(&mut state, painted, true);
        assert!(matches!(ed.game.ty(ship).unwrap().art, Art::Pixels(_)));
        assert!(state.draft.is_none());
        // ... and Undo brings the old picture back.
        ed.undo();
        assert!(matches!(ed.game.ty(ship).unwrap().art, Art::Built(_)));
    }

    fn png_bytes(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut img = image::RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(x, y, image::Rgba(f(x, y)));
            }
        }
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn small_pixel_art_is_imported_exactly() {
        let bytes = png_bytes(16, 16, |x, y| if x == 3 && y == 5 { [10, 20, 30, 255] } else { [0, 0, 0, 0] });
        let s = sprite_from_image_bytes(&bytes).unwrap();
        assert_eq!((s.w, s.h), (16, 16));
        assert_eq!(s.get(0, 3, 5), [10, 20, 30, 255]);
        assert_eq!(s.get(0, 0, 0)[3], 0, "see-through pixels stay see-through");
    }

    #[test]
    fn big_pictures_are_shrunk_and_keep_their_shape() {
        let bytes = png_bytes(200, 100, |_, _| [255, 0, 0, 255]);
        let s = sprite_from_image_bytes(&bytes).unwrap();
        assert_eq!((s.w, s.h), (32, 16));
        assert_eq!(s.get(0, 10, 8), [255, 0, 0, 255]);
    }

    #[test]
    fn things_that_are_not_pictures_give_a_friendly_error() {
        let e = sprite_from_image_bytes(b"this is not a picture").unwrap_err();
        assert!(e.contains("could not be opened"), "{e}");
    }

    #[test]
    fn a_picture_fits_into_a_frame_without_stretching() {
        let wide = sprite_from_image_bytes(&png_bytes(8, 4, |_, _| [1, 2, 3, 255])).unwrap();
        let frame = fit_frame(&wide, 16, 16);
        // 8x4 scaled up to 16x8, sitting at the bottom.
        assert_eq!(frame[15 * 16 + 5], [1, 2, 3, 255]);
        assert_eq!(frame[0], [0, 0, 0, 0]);
        assert_eq!(frame[7 * 16 + 5][3], 0);
    }

    #[test]
    fn importing_replaces_the_picture_or_adds_a_frame() {
        let mut ed = Editor::with_game(crate::examples::space_blaster());
        let ship = ed.game.types[0].id;
        let first = sprite_from_image_bytes(&png_bytes(16, 16, |_, _| [200, 0, 0, 255])).unwrap();
        ed.import_sprite(ship, first, false);
        let Art::Pixels(s) = &ed.game.ty(ship).unwrap().art else { panic!("should be your own picture now") };
        assert_eq!(s.frames.len(), 1);

        let second = sprite_from_image_bytes(&png_bytes(16, 16, |_, _| [0, 200, 0, 255])).unwrap();
        ed.import_sprite(ship, second, true);
        let Art::Pixels(s) = &ed.game.ty(ship).unwrap().art else { panic!() };
        assert_eq!(s.frames.len(), 2, "the new picture is an animation frame");
        assert_eq!(s.get(1, 8, 8), [0, 200, 0, 255]);
        assert_eq!(s.get(0, 8, 8), [200, 0, 0, 255]);
        ed.undo();
        let Art::Pixels(s) = &ed.game.ty(ship).unwrap().art else { panic!() };
        assert_eq!(s.frames.len(), 1, "Undo removes the frame again");
    }

    /// A pretend file that was dragged onto the window.
    #[derive(Debug)]
    struct FakeDrop(std::path::PathBuf);

    impl egui::DroppedFile for FakeDrop {
        fn path(&self) -> &Path {
            &self.0
        }
        fn bytes(&self) -> Result<Vec<u8>, String> {
            std::fs::read(&self.0).map_err(|e| e.to_string())
        }
    }

    #[test]
    fn a_png_dropped_on_the_window_becomes_the_picture() {
        let path = std::env::temp_dir().join("spark_drop_test.png");
        std::fs::write(&path, png_bytes(16, 16, |_, _| [0, 0, 255, 255])).unwrap();
        let mut ed = Editor::with_game(crate::examples::space_blaster());
        let ship = ed.game.types[0].id;
        ed.tool = Tool::Place(ship);
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 800.0))),
            dropped_files: vec![std::sync::Arc::new(FakeDrop(path.clone()))],
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            ed.handle_dropped_files(ui.ctx());
        });
        out.textures_delta.clear();
        assert!(matches!(ed.game.ty(ship).unwrap().art, Art::Pixels(_)));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn changing_pixels_changes_the_revision() {
        let mut s = Sprite::new(8, 8);
        let a = s.rev;
        s.set(0, 1, 1, [1, 1, 1, 255]);
        s.refresh();
        assert_ne!(a, s.rev);
    }
}
