//! The Spark editor: where games are made.
//!
//! Layout:
//!   top bar       - game name, New / Open / Save / Examples / Share, and the big PLAY button
//!   left panel    - tools and the objects of your game: pick what to place
//!   middle        - your level. Click to place things!
//!   right panel   - Thing / Object / Rules / Sounds / Game tabs
//!   bottom bar    - a hint about what the current tool does

mod object;
mod rules;
mod sounds;
mod sprite;

use crate::audio::Audio;
use crate::data::*;
use crate::engine::Runtime;
use crate::render::{self, Anim, View};
use crate::sound::Sound;
use crate::{examples, play, style};
use egui::{
    Align, Align2, Button, Color32, FontId, Frame, Key, Layout, Margin, Modifiers, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke,
    StrokeKind, TextEdit, Ui, pos2, vec2,
};
use std::fs;
use std::path::{Path, PathBuf};

const GAME_EXT: &str = "spark";

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Select,
    Eraser,
    Place(TypeId),
}

#[derive(Clone, Copy, PartialEq)]
#[repr(usize)]
enum Tab {
    Thing,
    Object,
    Rules,
    Sounds,
    Game,
}

/// Things that replace the whole game (we ask first if there are unsaved changes).
enum Pending {
    New,
    Open(PathBuf),
    Example(usize),
}

/// Developer helper: save a picture of the editor to a file, then close (`SPARK_SCREENSHOT=out.png`).
struct Shot {
    path: String,
    frames: u32,
    start_play: bool,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// A mouse stroke on the canvas (press, move, release).
struct Drag {
    button: PointerButton,
    last: Option<(i32, i32)>,
    moving: bool,
}

pub struct Editor {
    game: Game,
    level: usize,
    tool: Tool,
    /// The thing selected in the level, by the square it started on.
    selected: Option<(i32, i32)>,
    tab: Tab,
    undo: Vec<Game>,
    redo: Vec<Game>,
    dirty: bool,
    path: Option<PathBuf>,
    playing: Option<Runtime>,
    toast: Option<(String, f64)>,
    show_welcome: bool,
    show_help: bool,
    show_open: bool,
    show_examples: bool,
    show_add_object: bool,
    show_looks: bool,
    pending: Option<Pending>,
    drag: Option<Drag>,
    audio: Option<Audio>,
    audio_tried: bool,
    sound_sel: Option<u32>,
    sprite_ed: Option<sprite::SpriteEditor>,
    last_soft_snapshot: f64,
    music_preview: bool,
    /// Start with every rule opened (for screenshots and tests).
    open_rules: bool,
    shot: Option<Shot>,
    /// Where the right panel was drawn last time (used by tests to make sure it stays on screen).
    side_rect: Rect,
}

impl Editor {
    pub fn new() -> Self {
        let first_run = !crate::paths::games_dir().join(".welcome_seen").exists();
        let game = Game::blank();
        let tool = game.ty(ids::WALL).map(|t| Tool::Place(t.id)).unwrap_or(Tool::Select);
        Editor {
            game,
            level: 0,
            tool,
            selected: None,
            tab: Tab::Thing,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            path: None,
            playing: None,
            toast: None,
            show_welcome: first_run,
            show_help: false,
            show_open: false,
            show_examples: false,
            show_add_object: false,
            show_looks: false,
            pending: None,
            drag: None,
            audio: None,
            audio_tried: false,
            sound_sel: None,
            sprite_ed: None,
            last_soft_snapshot: -10.0,
            music_preview: false,
            open_rules: false,
            shot: None,
            side_rect: Rect::NOTHING,
        }
    }

    pub fn with_game(game: Game) -> Self {
        let mut e = Editor::new();
        e.game = game;
        e.show_welcome = false;
        e.tool = e.game.types.first().map(|t| Tool::Place(t.id)).unwrap_or(Tool::Select);
        e.apply_dev_hooks();
        e
    }

    // -----------------------------------------------------------------------
    // Small helpers
    // -----------------------------------------------------------------------

    fn say(&mut self, ctx: &egui::Context, text: impl Into<String>) {
        let now = ctx.input(|i| i.time);
        self.toast = Some((text.into(), now));
    }

    /// Remember the game so that Undo can bring it back. Call this BEFORE changing something.
    fn snapshot(&mut self) {
        self.undo.push(self.game.clone());
        if self.undo.len() > 80 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    /// Like `snapshot`, but only once in a while. Used when a slider or number is being dragged,
    /// so one drag becomes one Undo step instead of hundreds.
    fn snapshot_soft(&mut self, now: f64) {
        if now - self.last_soft_snapshot > 0.8 {
            self.snapshot();
        }
        self.last_soft_snapshot = now;
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.game, prev));
            self.fix_after_history();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.game, next));
            self.fix_after_history();
        }
    }

    fn fix_after_history(&mut self) {
        self.dirty = true;
        self.level = self.level.min(self.game.levels.len() - 1);
        self.selected = None;
        if let Tool::Place(t) = self.tool {
            if self.game.ty(t).is_none() {
                self.tool = Tool::Select;
            }
        }
    }

    fn replace_game(&mut self, game: Game, path: Option<PathBuf>) {
        self.game = game;
        self.path = path;
        self.level = 0;
        self.selected = None;
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
        self.drag = None;
        self.sound_sel = None;
        self.sprite_ed = None;
        self.tool = self.game.types.first().map(|t| Tool::Place(t.id)).unwrap_or(Tool::Select);
    }

    fn ensure_audio(&mut self) {
        if !self.audio_tried {
            self.audio_tried = true;
            self.audio = Audio::new();
        }
    }

    fn preview_sound(&mut self, s: &Sound) {
        self.ensure_audio();
        if let Some(a) = &mut self.audio {
            a.sfx_volume = self.game.sfx_volume;
            a.play(s);
        }
    }

    /// The object type shown in the Object tab: the one you are placing, or the one you selected.
    fn focus_type(&self) -> Option<TypeId> {
        if let Tool::Place(t) = self.tool {
            if self.game.ty(t).is_some() {
                return Some(t);
            }
        }
        let (x, y) = self.selected?;
        let i = self.game.levels[self.level].things.iter().position(|t| t.x == x && t.y == y)?;
        Some(self.game.levels[self.level].things[i].ty)
    }

    fn selected_index(&self) -> Option<usize> {
        let (x, y) = self.selected?;
        self.game.levels[self.level].things.iter().position(|t| t.x == x && t.y == y)
    }

    // -----------------------------------------------------------------------
    // Files
    // -----------------------------------------------------------------------

    fn save_path(&self) -> PathBuf {
        if let Some(p) = &self.path {
            return p.clone();
        }
        let base = safe_file_name(&self.game.title);
        let mut candidate = crate::paths::games_dir().join(format!("{base}.{GAME_EXT}"));
        let mut n = 2;
        while candidate.exists() {
            candidate = crate::paths::games_dir().join(format!("{base} {n}.{GAME_EXT}"));
            n += 1;
        }
        candidate
    }

    fn save(&mut self, ctx: &egui::Context) -> bool {
        let path = self.save_path();
        match self.game.save(&path) {
            Ok(()) => {
                self.say(ctx, format!("Saved! ({})", path.display()));
                self.path = Some(path);
                self.dirty = false;
                true
            }
            Err(e) => {
                self.say(ctx, e);
                false
            }
        }
    }

    fn list_saved_games() -> Vec<(String, PathBuf)> {
        let mut out = Vec::new();
        if let Ok(rd) = fs::read_dir(crate::paths::games_dir()) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some(GAME_EXT) {
                    let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    out.push((name, p));
                }
            }
        }
        out.sort_by_key(|(n, _)| n.to_lowercase());
        out
    }

    fn request(&mut self, ctx: &egui::Context, what: Pending) {
        if self.dirty {
            self.pending = Some(what);
        } else {
            self.do_pending(ctx, what);
        }
    }

    fn do_pending(&mut self, ctx: &egui::Context, what: Pending) {
        match what {
            Pending::New => {
                self.replace_game(Game::blank(), None);
                self.say(ctx, "A new game! Put things in the level, then press PLAY.");
            }
            Pending::Open(path) => match Game::load(&path) {
                Ok(g) => {
                    let name = g.title.clone();
                    self.replace_game(g, Some(path));
                    self.say(ctx, format!("Opened {name}"));
                }
                Err(e) => self.say(ctx, e),
            },
            Pending::Example(i) => {
                let g = (examples::ALL[i].build)();
                let name = g.title.clone();
                self.replace_game(g, None);
                self.say(ctx, format!("Loaded the example '{name}'. Press PLAY, then change anything you like!"));
            }
        }
    }

    /// Make a folder you can give to a friend: it has a copy of Spark that opens straight into your game.
    fn export(&mut self, ctx: &egui::Context) {
        let folder = crate::paths::exports_dir().join(safe_file_name(&self.game.title));
        let result = (|| -> Result<PathBuf, String> {
            fs::create_dir_all(&folder).map_err(|e| format!("Could not make the folder: {e}"))?;
            let exe = std::env::current_exe().map_err(|e| format!("Could not find Spark: {e}"))?;
            let exe_name = format!("{}{}", safe_file_name(&self.game.title), std::env::consts::EXE_SUFFIX);
            fs::copy(&exe, folder.join(&exe_name)).map_err(|e| format!("Could not copy Spark: {e}"))?;
            self.game.save(&folder.join("game.spark"))?;
            Ok(folder.clone())
        })();
        match result {
            Ok(f) => {
                self.say(ctx, format!("Your game is ready to share! Look in the folder: {}", f.display()));
                let opener = if cfg!(windows) {
                    "explorer"
                } else if cfg!(target_os = "macos") {
                    "open"
                } else {
                    "xdg-open"
                };
                let _ = std::process::Command::new(opener).arg(&f).spawn();
            }
            Err(e) => self.say(ctx, e),
        }
    }

    // -----------------------------------------------------------------------
    // Playing inside the editor
    // -----------------------------------------------------------------------

    fn start_play(&mut self, ctx: &egui::Context, from_level: usize) {
        for (i, _) in self.game.levels.iter().enumerate().skip(from_level) {
            if self.game.player_start(i).is_none() {
                self.level = i;
                let name = self.game.levels[i].name.clone();
                match self.game.player_type() {
                    Some(p) => {
                        self.tool = Tool::Place(p);
                        self.say(ctx, format!("'{name}' has no player! Click the level to place one."));
                    }
                    None => {
                        self.tab = Tab::Object;
                        self.say(ctx, "Your game has no player object. Add one (try 'Hero') or give an object a 'You: ...' behavior.");
                    }
                }
                return;
            }
        }
        self.ensure_audio();
        if let Some(a) = &mut self.audio {
            a.music_volume = self.game.music_volume;
            a.set_music(self.game.music);
        }
        self.sprite_ed = None;
        self.playing = Some(Runtime::new(&self.game, from_level));
    }

    fn stop_play(&mut self) {
        self.playing = None;
        if let Some(a) = &mut self.audio {
            a.stop_music();
        }
    }

    fn play_screen(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.stop_play();
            return;
        }
        let input = play::read_input(&ctx);
        let dt = ctx.input(|i| i.unstable_dt);
        if let Some(rt) = &mut self.playing {
            rt.update(dt, &input);
            play::play_events(rt, &mut self.audio);
        }
        let mut stop = false;
        egui::Panel::top("play_bar").frame(Frame::new().fill(style::PURPLE_DARK).inner_margin(Margin::symmetric(12, 6))).show(ui, |ui| {
            ui.horizontal(|ui| {
                let b = Button::new(RichText::new("Stop (Esc)").size(17.0).strong().color(Color32::WHITE))
                    .fill(Color32::from_rgb(220, 70, 90))
                    .min_size(vec2(120.0, 32.0));
                if ui.add(b).clicked() {
                    stop = true;
                }
                ui.label(RichText::new("Testing your game. Arrow keys / WASD to move, Space to jump, R to restart.").color(Color32::WHITE));
            });
        });
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter_at(rect);
            if let Some(rt) = &self.playing {
                render::draw_game(&painter, rect, rt, "Esc = stop");
            }
        });
        if stop {
            self.stop_play();
        }
        ctx.request_repaint();
    }

    // -----------------------------------------------------------------------
    // Keyboard shortcuts
    // -----------------------------------------------------------------------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (undo, redo, save, delete) = ctx.input_mut(|i| {
            let redo = i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || i.consume_key(Modifiers::COMMAND, Key::Y);
            let undo = i.consume_key(Modifiers::COMMAND, Key::Z);
            let save = i.consume_key(Modifiers::COMMAND, Key::S);
            let delete = i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace);
            (undo, redo, save, delete)
        });
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if save {
            self.save(ctx);
        }
        if delete {
            if let Some((x, y)) = self.selected {
                self.snapshot();
                let lvl = self.level;
                self.game.erase(lvl, x, y);
                self.selected = None;
            }
        }
        let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9];
        for (k, t) in keys.iter().zip(self.game.types.iter()) {
            if ctx.input(|i| i.key_pressed(*k)) {
                self.tool = Tool::Place(t.id);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Q)) {
            self.tool = Tool::Select;
        }
        if ctx.input(|i| i.key_pressed(Key::E)) {
            self.tool = Tool::Eraser;
        }
    }

    // -----------------------------------------------------------------------
    // Top bar
    // -----------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        egui::Panel::top("top_bar")
            .frame(Frame::new().fill(Color32::WHITE).inner_margin(Margin::symmetric(12, 8)).stroke(Stroke::new(1.0, Color32::from_rgb(210, 202, 245))))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Spark").size(28.0).strong().color(style::PURPLE));
                    ui.add_space(10.0);
                    ui.label("Game name:");
                    let r = ui.add(TextEdit::singleline(&mut self.game.title).desired_width(170.0));
                    if r.changed() {
                        self.dirty = true;
                    }
                    ui.add_space(10.0);
                    if ui.add(top_button("New")).on_hover_text("Start a fresh game").clicked() {
                        self.request(&ctx, Pending::New);
                    }
                    if ui.add(top_button("Open")).on_hover_text("Open a game you saved").clicked() {
                        self.show_open = true;
                    }
                    let save_label = if self.dirty { "Save *" } else { "Save" };
                    if ui.add(top_button(save_label)).on_hover_text("Save your game (Ctrl+S)").clicked() {
                        self.save(&ctx);
                    }
                    if ui.add(top_button("Examples")).on_hover_text("Open games that come with Spark").clicked() {
                        self.show_examples = true;
                    }
                    if ui.add(top_button("Share")).on_hover_text("Make a folder with a copy of your game that your friends can play").clicked() {
                        self.export(&ctx);
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let play = Button::new(RichText::new("PLAY").size(22.0).strong().color(Color32::WHITE))
                            .fill(style::GREEN)
                            .stroke(Stroke::new(2.0, style::GREEN_DARK))
                            .min_size(vec2(120.0, 42.0));
                        if ui.add(play).on_hover_text("Play your whole game from level 1").clicked() {
                            self.start_play(&ctx, 0);
                        }
                        let test = Button::new(RichText::new("Test level").size(16.0).strong().color(style::GREEN_DARK)).min_size(vec2(110.0, 42.0));
                        if ui.add(test).on_hover_text("Play just the level you are looking at").clicked() {
                            let l = self.level;
                            self.start_play(&ctx, l);
                        }
                        if ui.button(RichText::new("Help").size(16.0)).clicked() {
                            self.show_help = true;
                        }
                    });
                });
            });
    }

    // -----------------------------------------------------------------------
    // Left panel: tools and objects
    // -----------------------------------------------------------------------

    fn tools_panel(&mut self, ui: &mut Ui) {
        egui::Panel::left("tools").exact_size(214.0).resizable(false).frame(Frame::new().fill(style::PANEL).inner_margin(Margin::same(10))).show(ui, |ui| {
            ui.label(RichText::new("Tools").heading());
            if tool_button(ui, self.tool == Tool::Select, "Select / Move", "Q", "Click a thing to change it. Drag it to move it.", |p, r| draw_cursor_icon(p, r)).clicked() {
                self.tool = Tool::Select;
            }
            if tool_button(ui, self.tool == Tool::Eraser, "Eraser", "E", "Click or drag to rub things out. (Right-click always erases.)", |p, r| draw_eraser_icon(p, r)).clicked() {
                self.tool = Tool::Eraser;
            }

            ui.add_space(4.0);
            ui.label(RichText::new("Objects").heading());
            let h = (ui.available_height() - 90.0).max(120.0);
            egui::ScrollArea::vertical().max_height(h).auto_shrink([false, false]).show(ui, |ui| {
                for (n, t) in self.game.types.iter().enumerate() {
                    let shortcut = if n < 9 { (n + 1).to_string() } else { String::new() };
                    let color = render::rgb(t.color);
                    let anim = Anim { ghost: t.behavior == Behavior::Follow, ..Anim::default() };
                    let hint = format!("{} ({})", t.name, t.behavior.label());
                    if tool_button(ui, self.tool == Tool::Place(t.id), &t.name, &shortcut, &hint, |p, r| render::draw_type(p, t, r, color, anim)).clicked() {
                        self.tool = Tool::Place(t.id);
                        self.tab = Tab::Object;
                    }
                }
                if ui.add(Button::new(RichText::new("+ New object").size(16.0).strong().color(style::PURPLE_DARK)).min_size(vec2(ui.available_width(), 36.0))).clicked() {
                    self.show_add_object = true;
                }
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.undo.is_empty(), Button::new("Undo")).on_hover_text("Ctrl+Z").clicked() {
                    self.undo();
                }
                if ui.add_enabled(!self.redo.is_empty(), Button::new("Redo")).on_hover_text("Ctrl+Y").clicked() {
                    self.redo();
                }
            });
        });
    }

    // -----------------------------------------------------------------------
    // Right panel
    // -----------------------------------------------------------------------

    fn side_panel(&mut self, ui: &mut Ui) {
        if self.tab != Tab::Sounds && self.music_preview {
            self.music_preview = false;
            if let Some(a) = &mut self.audio {
                a.stop_music();
            }
        }
        let shown = egui::Panel::right("side").exact_size(360.0).resizable(false).frame(Frame::new().fill(style::PANEL).inner_margin(Margin::same(10))).show(ui, |ui| {
            // The tabs may wrap onto a second line. If they were wider than the panel, the panel would be pushed off the screen.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.spacing_mut().button_padding = vec2(8.0, 5.0);
                for (tab, name) in [(Tab::Thing, "Thing"), (Tab::Object, "Object"), (Tab::Rules, "Rules"), (Tab::Sounds, "Sounds"), (Tab::Game, "Game")] {
                    ui.selectable_value(&mut self.tab, tab, RichText::new(name).size(16.0));
                }
            });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
                Tab::Thing => self.thing_tab(ui),
                Tab::Object => self.object_tab(ui),
                Tab::Rules => self.rules_tab(ui),
                Tab::Sounds => self.sounds_tab(ui),
                Tab::Game => self.game_tab(ui),
            });
        });
        self.side_rect = shown.response.rect;
        if std::env::var("SPARK_DEBUG_LAYOUT").is_ok() {
            eprintln!("right panel rect = {:?}", shown.response.rect);
        }
    }

    // -----------------------------------------------------------------------
    // Bottom bar
    // -----------------------------------------------------------------------

    fn bottom_bar(&mut self, ui: &mut Ui) {
        let hint = match self.tool {
            Tool::Place(t) => format!("Click or drag on the level to add: {}.  Right-click to erase.", self.game.type_name(t)),
            Tool::Select => "Click a thing to select it. Drag it to move it. Press Delete to remove it.".to_string(),
            Tool::Eraser => "Click or drag on the level to erase things.".to_string(),
        };
        let now = ui.input(|i| i.time);
        if let Some((_, t)) = &self.toast {
            if now - *t > 5.0 {
                self.toast = None;
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
            }
        }
        egui::Panel::bottom("hint_bar")
            .frame(Frame::new().fill(Color32::WHITE).inner_margin(Margin::symmetric(14, 8)).stroke(Stroke::new(1.0, Color32::from_rgb(210, 202, 245))))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some((msg, _)) = &self.toast {
                        ui.label(RichText::new(msg).strong().color(style::PURPLE_DARK));
                    } else {
                        ui.label(RichText::new(hint).color(Color32::from_rgb(90, 80, 140)));
                    }
                });
            });
    }

    // -----------------------------------------------------------------------
    // The level canvas
    // -----------------------------------------------------------------------

    fn canvas(&mut self, ui: &mut Ui) {
        egui::CentralPanel::default().frame(Frame::new().fill(Color32::from_rgb(60, 54, 92)).inner_margin(Margin::same(10))).show(ui, |ui| {
            // Level tabs.
            ui.horizontal_wrapped(|ui| {
                let mut switch_to = None;
                for (i, l) in self.game.levels.iter().enumerate() {
                    let label = RichText::new(format!("{}. {}", i + 1, l.name))
                        .size(16.0)
                        .color(if i == self.level { style::INK } else { Color32::from_rgb(220, 215, 250) });
                    if ui.selectable_label(i == self.level, label).clicked() {
                        switch_to = Some(i);
                    }
                }
                if let Some(i) = switch_to {
                    self.level = i;
                    self.selected = None;
                    self.drag = None;
                }
                if ui.button("+ Add level").clicked() {
                    self.snapshot();
                    let n = self.game.levels.len() + 1;
                    let l = self.game.starter_level(&format!("Level {n}"));
                    self.game.levels.push(l);
                    self.level = self.game.levels.len() - 1;
                    self.selected = None;
                }
            });

            let size = ui.available_size();
            let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
            self.canvas_interaction(ui, &resp, &painter);
        });
    }

    fn canvas_interaction(&mut self, ui: &mut Ui, resp: &egui::Response, painter: &Painter) {
        let rect = resp.rect;
        let (lw, lh, sky) = {
            let l = &self.game.levels[self.level];
            (l.width, l.height, l.sky)
        };
        let view = View::fit(rect.shrink(4.0), lw, lh);
        let level_rect = view.level_rect(lw, lh);

        let cell_at = |pos: Pos2| -> Option<(i32, i32)> {
            if !level_rect.contains(pos) {
                return None;
            }
            let c = (pos - view.origin) / view.cell;
            Some((c.x.floor() as i32, c.y.floor() as i32))
        };
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let covered_by_window = self.sprite_ed.is_some() && !resp.hovered();
        let hover_cell = if (resp.hovered() || self.drag.is_some()) && !covered_by_window { pointer.and_then(cell_at) } else { None };

        // --- mouse ---
        if self.drag.is_none() && resp.hovered() {
            for button in [PointerButton::Primary, PointerButton::Secondary] {
                if ui.input(|i| i.pointer.button_pressed(button)) {
                    self.drag = Some(Drag { button, last: None, moving: false });
                }
            }
        }
        if let Some(d) = &self.drag {
            let button = d.button;
            if ui.input(|i| i.pointer.button_down(button)) {
                if let Some(cell) = hover_cell {
                    let first = d.last.is_none();
                    if self.drag.as_ref().and_then(|d| d.last) != Some(cell) {
                        self.paint_at(button, cell, first);
                        if let Some(d) = &mut self.drag {
                            d.last = Some(cell);
                        }
                    }
                }
            } else {
                self.drag = None;
            }
        }
        if resp.hovered() || self.drag.is_some() {
            let icon = match (&self.drag, self.tool) {
                (Some(d), _) if d.moving => egui::CursorIcon::Grabbing,
                (_, Tool::Select) => egui::CursorIcon::PointingHand,
                _ => egui::CursorIcon::Crosshair,
            };
            ui.ctx().set_cursor_icon(icon);
        }

        // --- drawing ---
        let game = &self.game;
        let level = &game.levels[self.level];
        painter.rect_filled(level_rect, 4.0, render::rgb(sky));
        let grid = Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 40, 28));
        for x in 0..=lw {
            let px = view.origin.x + x as f32 * view.cell;
            painter.line_segment([pos2(px, level_rect.min.y), pos2(px, level_rect.max.y)], grid);
        }
        for y in 0..=lh {
            let py = view.origin.y + y as f32 * view.cell;
            painter.line_segment([pos2(level_rect.min.x, py), pos2(level_rect.max.x, py)], grid);
        }
        let clip = painter.with_clip_rect(level_rect);
        let time = ui.input(|i| i.time) as f32;
        let any_animated = level.things.iter().any(|t| game.ty(t.ty).is_some_and(|o| matches!(&o.art, Art::Pixels(s) if s.frames.len() > 1)));
        for layer in 0..3u8 {
            for t in &level.things {
                let Some(ty) = game.ty(t.ty) else { continue };
                if ty.layer.min(2) != layer {
                    continue;
                }
                let r = view.rect(t.x as f32, t.y as f32, ty.size_w, ty.size_h);
                let behavior = t.behavior.unwrap_or(ty.behavior);
                let anim = Anim { time: if any_animated { time } else { 0.0 }, ghost: behavior == Behavior::Follow, ..Anim::default() };
                render::draw_type(&clip, ty, r, render::rgb(t.color.unwrap_or(ty.color)), anim);
                render::draw_motion_badge(&clip, r, behavior, ty.drift);
            }
        }
        if any_animated {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        painter.rect_stroke(level_rect, 4.0, Stroke::new(2.0, Color32::from_rgb(40, 34, 70)), StrokeKind::Outside);

        if let Some(i) = self.selected_index() {
            let t = &level.things[i];
            let (w, h) = game.footprint(t);
            let r = view.rect(t.x as f32, t.y as f32, w, h).expand(1.0);
            painter.rect_stroke(r, 4.0, Stroke::new(4.0, Color32::WHITE), StrokeKind::Outside);
            painter.rect_stroke(r, 4.0, Stroke::new(2.0, style::PURPLE), StrokeKind::Outside);
        }
        if let Some((hx, hy)) = hover_cell {
            let r1 = view.cell_rect(hx as f32, hy as f32);
            match self.tool {
                Tool::Place(id) => {
                    if let Some(ty) = game.ty(id) {
                        let r = view.rect(hx as f32, hy as f32, ty.size_w, ty.size_h);
                        let mut ghost = painter.clone();
                        ghost.set_opacity(0.55);
                        render::draw_type(&ghost, ty, r, render::rgb(ty.color), Anim::default());
                        painter.rect_stroke(r, 2.0, Stroke::new(2.0, Color32::WHITE), StrokeKind::Inside);
                    }
                }
                Tool::Eraser => {
                    painter.rect_filled(r1, 2.0, Color32::from_rgba_unmultiplied(255, 60, 90, 90));
                    painter.rect_stroke(r1, 2.0, Stroke::new(2.0, Color32::from_rgb(255, 60, 90)), StrokeKind::Inside);
                }
                Tool::Select => {
                    painter.rect_stroke(r1, 2.0, Stroke::new(2.0, style::PURPLE), StrokeKind::Inside);
                }
            }
        }
    }

    /// Do what the current tool does at one grid square.
    fn paint_at(&mut self, button: PointerButton, cell: (i32, i32), first_cell_of_stroke: bool) {
        let (cx, cy) = cell;
        let lvl = self.level;
        let existing = self.game.thing_covering(lvl, cx, cy);
        let erase = button == PointerButton::Secondary || self.tool == Tool::Eraser;

        if erase {
            if existing.is_some() {
                self.snapshot();
                self.game.erase(lvl, cx, cy);
                self.selected = None;
            }
            return;
        }
        match self.tool {
            Tool::Place(ty) => {
                let same = self.game.levels[lvl].things.iter().any(|t| t.x == cx && t.y == cy && t.ty == ty);
                if !same {
                    self.snapshot();
                    let thing = self.game.new_thing(ty, cx, cy);
                    self.game.place(lvl, thing);
                    self.selected = Some(cell);
                } else if first_cell_of_stroke {
                    self.selected = Some(cell);
                }
            }
            Tool::Select => {
                if first_cell_of_stroke {
                    self.selected = existing.map(|i| {
                        let t = &self.game.levels[lvl].things[i];
                        (t.x, t.y)
                    });
                    if let Some(d) = &mut self.drag {
                        d.moving = existing.is_some();
                    }
                    if existing.is_some() {
                        self.tab = Tab::Thing;
                    }
                } else if self.drag.as_ref().is_some_and(|d| d.moving) {
                    if let Some((sx, sy)) = self.selected {
                        if (sx, sy) != cell && self.selected_index().is_some() {
                            self.snapshot();
                            let level = &mut self.game.levels[lvl];
                            level.things.retain(|t| !(t.x == cx && t.y == cy));
                            if let Some(i) = level.things.iter().position(|t| t.x == sx && t.y == sy) {
                                level.things[i].x = cx;
                                level.things[i].y = cy;
                                self.selected = Some(cell);
                            }
                        }
                    }
                }
            }
            Tool::Eraser => {}
        }
    }

    // -----------------------------------------------------------------------
    // Pop-up windows
    // -----------------------------------------------------------------------

    fn windows(&mut self, ctx: &egui::Context) {
        // Welcome / help.
        if self.show_welcome || self.show_help {
            let mut keep = true;
            let mut let_go = false;
            egui::Window::new(RichText::new("Welcome to Spark!").size(22.0).strong())
                .open(&mut keep)
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_width(500.0);
                    ui.label(RichText::new("Make ANY kind of game in 3 easy steps:").size(18.0));
                    ui.add_space(6.0);
                    step(ui, "1", "Pick an object on the left (or add a new one with + New object).");
                    step(ui, "2", "Click or drag on the level to put it there. Right-click erases.");
                    step(ui, "3", "Press the green PLAY button!");
                    ui.add_space(8.0);
                    ui.separator();
                    ui.label(RichText::new("While you play").strong());
                    ui.label("Arrow keys or W A S D to move.  Space (or Up) to jump.  Z, X and Enter can be used by your rules.");
                    ui.label("R starts again after a win or game over.  Esc stops playing.");
                    ui.add_space(4.0);
                    ui.label(RichText::new("Make it yours").strong());
                    ui.label("Object tab: draw your own picture, choose how it moves (player, enemy, bullet, ball...).");
                    ui.label("Rules tab: WHEN something happens, DO something. Shoot, spawn, bounce, count keys, open doors!");
                    ui.label("Sounds tab: make beeps and boings, and pick the music.");
                    ui.label("Examples: finished games of many kinds to play and change.  Share: give your game to friends.");
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        let b = Button::new(RichText::new("Let's go!").size(20.0).strong().color(Color32::WHITE)).fill(style::PURPLE).min_size(vec2(160.0, 40.0));
                        if ui.add(b).clicked() {
                            let_go = true;
                        }
                    });
                });
            if !keep || let_go {
                if self.show_welcome {
                    let _ = fs::create_dir_all(crate::paths::games_dir());
                    let _ = fs::write(crate::paths::games_dir().join(".welcome_seen"), "1");
                }
                self.show_welcome = false;
                self.show_help = false;
            }
        }

        // Open a saved game.
        if self.show_open {
            let mut keep = true;
            let mut chosen: Option<PathBuf> = None;
            egui::Window::new(RichText::new("Open a game").size(20.0).strong())
                .open(&mut keep)
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_width(380.0);
                    let games = Self::list_saved_games();
                    if games.is_empty() {
                        ui.label("You have no saved games yet. Press Save to keep your game!");
                    }
                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        for (name, path) in games {
                            if ui.add(Button::new(RichText::new(&name).size(18.0)).min_size(vec2(360.0, 36.0))).clicked() {
                                chosen = Some(path);
                            }
                        }
                    });
                });
            if let Some(p) = chosen {
                self.show_open = false;
                self.request(ctx, Pending::Open(p));
            } else if !keep {
                self.show_open = false;
            }
        }

        // Examples.
        if self.show_examples {
            let mut keep = true;
            let mut chosen: Option<usize> = None;
            egui::Window::new(RichText::new("Example games").size(20.0).strong())
                .open(&mut keep)
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_width(480.0);
                    egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                        for (i, ex) in examples::ALL.iter().enumerate() {
                            Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.5, Color32::from_rgb(205, 198, 240))).corner_radius(10.0).inner_margin(Margin::same(10)).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(ex.name).size(19.0).strong());
                                        ui.label(RichText::new(ex.genre).small().color(style::PURPLE));
                                        ui.add_sized([340.0, 20.0], egui::Label::new(RichText::new(ex.blurb).small()).wrap());
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.add(Button::new(RichText::new("Open").size(17.0).strong()).min_size(vec2(70.0, 34.0))).clicked() {
                                            chosen = Some(i);
                                        }
                                    });
                                });
                            });
                        }
                    });
                });
            if let Some(i) = chosen {
                self.show_examples = false;
                self.request(ctx, Pending::Example(i));
            } else if !keep {
                self.show_examples = false;
            }
        }

        // Add an object.
        if self.show_add_object {
            let mut keep = true;
            let mut chosen: Option<usize> = None;
            egui::Window::new(RichText::new("Add an object").size(20.0).strong())
                .open(&mut keep)
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.set_width(560.0);
                    ui.label("Pick one to add to your game. You can change its look and behavior afterwards.");
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        ui.columns(2, |cols| {
                            for (i, tpl) in TEMPLATES.iter().enumerate() {
                                let ui = &mut cols[i % 2];
                                let sample = (tpl.make)(0);
                                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::click());
                                let p = ui.painter();
                                let fill = if resp.hovered() { Color32::from_rgb(238, 232, 255) } else { Color32::WHITE };
                                p.rect_filled(rect, 10.0, fill);
                                p.rect_stroke(rect, 10.0, Stroke::new(1.5, Color32::from_rgb(205, 198, 240)), StrokeKind::Inside);
                                render::draw_type(p, &sample, Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2(42.0, 42.0)), render::rgb(sample.color), Anim::default());
                                p.text(pos2(rect.min.x + 60.0, rect.min.y + 18.0), Align2::LEFT_CENTER, tpl.name, FontId::proportional(15.0), style::INK);
                                p.text(pos2(rect.min.x + 60.0, rect.min.y + 40.0), Align2::LEFT_CENTER, short(tpl.blurb, 34), FontId::proportional(12.0), Color32::from_rgb(120, 110, 160));
                                if resp.on_hover_text(tpl.blurb).clicked() {
                                    chosen = Some(i);
                                }
                            }
                        });
                    });
                });
            if let Some(i) = chosen {
                self.snapshot();
                let id = self.game.add_template_index(i);
                self.tool = Tool::Place(id);
                self.tab = Tab::Object;
                self.show_add_object = false;
                let name = self.game.type_name(id);
                self.say(ctx, format!("Added '{name}'. Click the level to place it, or change it in the Object tab."));
            } else if !keep {
                self.show_add_object = false;
            }
        }

        // "Save first?" question.
        if self.pending.is_some() {
            let mut action: Option<u8> = None; // 0 save+continue, 1 discard+continue, 2 cancel
            egui::Window::new(RichText::new("Save your game first?").size(20.0).strong())
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("You changed your game since you last saved it.");
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.add(Button::new(RichText::new("Save, then continue").strong()).fill(style::GREEN).min_size(vec2(100.0, 34.0))).clicked() {
                            action = Some(0);
                        }
                        if ui.add(Button::new("Don't save").min_size(vec2(90.0, 34.0))).clicked() {
                            action = Some(1);
                        }
                        if ui.add(Button::new("Cancel").min_size(vec2(80.0, 34.0))).clicked() {
                            action = Some(2);
                        }
                    });
                });
            if let Some(a) = action {
                let what = self.pending.take();
                match (a, what) {
                    (0, Some(w)) => {
                        if self.save(ctx) {
                            self.do_pending(ctx, w);
                        }
                    }
                    (1, Some(w)) => self.do_pending(ctx, w),
                    _ => {}
                }
            }
        }

        self.sprite_window(ctx);
        self.looks_window(ctx);
        self.drop_hint(ctx);
    }
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.dev_screenshot(&ctx);

        // Closing the window saves the game, so nothing is ever lost.
        if ctx.input(|i| i.viewport().close_requested()) && self.dirty {
            self.save(&ctx);
        }

        if self.playing.is_some() {
            self.play_screen(ui);
            return;
        }

        self.draw_editor(ui);
    }
}

impl Editor {
    /// Takes the picture for `SPARK_SCREENSHOT`.
    fn dev_screenshot(&mut self, ctx: &egui::Context) {
        let Some(shot) = &mut self.shot else { return };
        shot.frames += 1;
        if shot.frames == 3 && shot.start_play {
            shot.start_play = false;
            self.start_play(ctx, 0);
        }
        let Some(shot) = &mut self.shot else { return };
        if shot.frames == 25 {
            let (path, done) = (shot.path.clone(), shot.done.clone());
            ctx.request_screenshot(move |img| {
                let bytes: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Err(e) = image::save_buffer(&path, &bytes, img.size[0] as u32, img.size[1] as u32, image::ColorType::Rgba8) {
                    eprintln!("could not save the picture: {e}");
                }
                done.store(true, std::sync::atomic::Ordering::SeqCst);
            });
        }
        if shot.done.load(std::sync::atomic::Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }

    /// One frame of the editor (everything except playing).
    fn draw_editor(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let debug = std::env::var("SPARK_DEBUG_LAYOUT").is_ok();
        let log = |what: &str, ui: &Ui| {
            if debug {
                eprintln!("{what}: free space = {:?}, screen = {:?}, ppp = {}", ui.available_rect_before_wrap(), ui.ctx().content_rect(), ui.ctx().pixels_per_point());
            }
        };
        self.handle_dropped_files(&ctx);
        self.shortcuts(&ctx);
        log("start", ui);
        self.top_bar(ui);
        log("after top", ui);
        self.bottom_bar(ui);
        log("after bottom", ui);
        self.tools_panel(ui);
        log("after left", ui);
        self.side_panel(ui);
        log("after right", ui);
        self.canvas(ui);
        self.windows(&ctx);
    }

    /// Hidden helpers for developers: `SPARK_TAB=rules` starts on that tab, `SPARK_SPRITE=1` opens the picture drawer.
    fn apply_dev_hooks(&mut self) {
        if let Ok(t) = std::env::var("SPARK_TAB") {
            self.tab = match t.as_str() {
                "object" => Tab::Object,
                "rules" => Tab::Rules,
                "sounds" => Tab::Sounds,
                "game" => Tab::Game,
                _ => Tab::Thing,
            };
        }
        if std::env::var("SPARK_SPRITE").is_ok() {
            if let Some(id) = self.focus_type() {
                self.open_sprite_editor(id);
            }
        }
        if let Ok(path) = std::env::var("SPARK_SCREENSHOT") {
            self.shot = Some(Shot { path, frames: 0, start_play: std::env::var("SPARK_PLAY").is_ok(), done: Default::default() });
        }
        if std::env::var("SPARK_OPEN_RULES").is_ok() {
            self.open_rules = true;
        }
        if std::env::var("SPARK_WINDOW").as_deref() == Ok("examples") {
            self.show_examples = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Little helpers for the buttons
// ---------------------------------------------------------------------------

fn short(text: &str, max: usize) -> String {
    if text.chars().count() <= max { text.to_string() } else { format!("{}...", text.chars().take(max.saturating_sub(3)).collect::<String>()) }
}

fn top_button(label: &str) -> Button<'_> {
    Button::new(RichText::new(label).size(17.0)).min_size(vec2(0.0, 36.0))
}

fn step(ui: &mut Ui, number: &str, text: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 15.0, style::PURPLE);
        ui.painter().text(rect.center(), Align2::CENTER_CENTER, number, FontId::proportional(18.0), Color32::WHITE);
        ui.label(RichText::new(text).size(17.0));
    });
}

fn tool_button(ui: &mut Ui, selected: bool, label: &str, shortcut: &str, hint: &str, icon: impl FnOnce(&Painter, Rect)) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
    let fill = if selected {
        Color32::from_rgb(222, 212, 255)
    } else if resp.hovered() {
        Color32::from_rgb(238, 232, 255)
    } else {
        Color32::WHITE
    };
    let stroke = if selected { Stroke::new(2.5, style::PURPLE) } else { Stroke::new(1.5, Color32::from_rgb(205, 198, 240)) };
    let p = ui.painter();
    p.rect_filled(rect, 10.0, fill);
    p.rect_stroke(rect, 10.0, stroke, StrokeKind::Inside);
    icon(&p.with_clip_rect(rect), Rect::from_min_size(rect.min + vec2(7.0, 5.0), vec2(36.0, 36.0)));
    p.text(pos2(rect.min.x + 54.0, rect.center().y), Align2::LEFT_CENTER, short(label, 14), FontId::proportional(17.0), style::INK);
    p.text(pos2(rect.max.x - 10.0, rect.center().y), Align2::RIGHT_CENTER, shortcut, FontId::proportional(13.0), Color32::from_rgb(150, 140, 190));
    resp.on_hover_text(hint)
}

fn draw_cursor_icon(p: &Painter, r: Rect) {
    let pt = |x: f32, y: f32| pos2(r.min.x + x * r.width(), r.min.y + y * r.height());
    p.add(Shape::convex_polygon(vec![pt(0.22, 0.08), pt(0.22, 0.8), pt(0.4, 0.64), pt(0.76, 0.64)], Color32::WHITE, Stroke::new(2.0, style::INK)));
    p.add(Shape::line(vec![pt(0.46, 0.66), pt(0.6, 0.92)], Stroke::new(4.0, style::INK)));
}

fn draw_eraser_icon(p: &Painter, r: Rect) {
    let pt = |x: f32, y: f32| pos2(r.min.x + x * r.width(), r.min.y + y * r.height());
    let body = vec![pt(0.1, 0.62), pt(0.5, 0.14), pt(0.92, 0.42), pt(0.52, 0.9)];
    p.add(Shape::convex_polygon(body, Color32::from_rgb(255, 150, 175), Stroke::new(2.0, style::INK)));
    p.add(Shape::convex_polygon(vec![pt(0.1, 0.62), pt(0.3, 0.38), pt(0.74, 0.66), pt(0.52, 0.9)], Color32::WHITE, Stroke::new(2.0, style::INK)));
}

/// Open the editor window.
pub fn run_editor() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1420.0, 840.0]).with_min_inner_size([1080.0, 640.0]).with_title("Spark Game Maker").with_icon(style::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Spark Game Maker",
        options,
        Box::new(|cc| {
            style::apply(&cc.egui_ctx);
            let mut editor = Editor::new();
            editor.apply_dev_hooks();
            Ok(Box::new(editor))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, ed: &mut Editor, t: f64) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1500.0, 900.0))),
            time: Some(t),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| ed.draw_editor(ui));
        out.textures_delta.clear();
    }

    /// Draw every tab, window and tool for every example game. Nothing may panic.
    #[test]
    fn every_screen_of_the_editor_draws() {
        let ctx = egui::Context::default();
        style::apply(&ctx);
        let mut t = 0.0;
        for ex in examples::ALL.iter() {
            let mut ed = Editor::with_game((ex.build)());
            ed.show_welcome = false;
            ed.open_rules = true; // draw all the rule widgets too
            for tab in [Tab::Thing, Tab::Object, Tab::Rules, Tab::Sounds, Tab::Game] {
                for tool in [Tool::Select, Tool::Eraser, Tool::Place(ed.game.types[0].id)] {
                    ed.tab = tab;
                    ed.tool = tool;
                    // Pretend to have clicked on the first thing in the level.
                    ed.selected = ed.game.levels[0].things.first().map(|th| (th.x, th.y));
                    for _ in 0..2 {
                        t += 0.1;
                        frame(&ctx, &mut ed, t);
                    }
                }
            }
            ed.show_help = true;
            ed.show_open = true;
            ed.show_examples = true;
            ed.show_add_object = true;
            ed.show_looks = true;
            for ty in ed.game.types.iter().map(|t| t.id).collect::<Vec<_>>() {
                ed.tool = Tool::Place(ty);
                t += 0.1;
                frame(&ctx, &mut ed, t);
            }
            ed.open_sprite_editor(ed.game.types[0].id);
            for _ in 0..3 {
                t += 0.1;
                frame(&ctx, &mut ed, t);
            }
        }
    }

    /// The right panel must sit inside the window, whatever tab is open. (It used to be pushed off the
    /// screen when the row of tabs was wider than the panel.)
    #[test]
    fn the_side_panel_stays_inside_the_window() {
        for (w, h) in [(1420.0, 840.0), (1280.0, 720.0), (1100.0, 650.0)] {
            let ctx = egui::Context::default();
            style::apply(&ctx);
            for ex in examples::ALL.iter() {
                let mut ed = Editor::with_game((ex.build)());
                ed.open_rules = true;
                for tab in [Tab::Thing, Tab::Object, Tab::Rules, Tab::Sounds, Tab::Game] {
                    ed.tab = tab;
                    ed.selected = ed.game.levels[0].things.first().map(|t| (t.x, t.y));
                    for i in 0..3 {
                        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))), time: Some(i as f64 * 0.1), ..Default::default() };
                        let mut out = ctx.run_ui(input, |ui| ed.draw_editor(ui));
                        out.textures_delta.clear();
                    }
                    let r = ed.side_rect;
                    assert!(r.max.x <= w + 0.5 && r.min.x >= w - 440.0, "{} / tab {}: the right panel is at {r:?} but the window is {w} wide", ex.name, ["Thing", "Object", "Rules", "Sounds", "Game"][tab as usize]);
                    assert!(r.width() <= 440.0, "{}: the panel grew to {} wide", ex.name, r.width());
                }
            }
        }
    }

    #[test]
    fn playing_inside_the_editor_works() {
        let ctx = egui::Context::default();
        style::apply(&ctx);
        for ex in examples::ALL.iter() {
            let mut ed = Editor::with_game((ex.build)());
            ed.start_play(&ctx, 0);
            assert!(ed.playing.is_some(), "{} should start", ex.name);
            for i in 0..30 {
                let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))), time: Some(i as f64 * 0.016), ..Default::default() };
                let mut out = ctx.run_ui(input, |ui| ed.play_screen(ui));
                out.textures_delta.clear();
            }
            ed.stop_play();
            assert!(ed.playing.is_none());
        }
    }

    #[test]
    fn painting_erasing_and_undoing_work() {
        let mut ed = Editor::with_game(Game::blank());
        let ty = ids::COIN;
        ed.tool = Tool::Place(ty);
        let before = ed.game.levels[0].things.len();
        ed.paint_at(PointerButton::Primary, (5, 3), true);
        assert_eq!(ed.game.levels[0].things.len(), before + 1);
        ed.paint_at(PointerButton::Secondary, (5, 3), true);
        assert_eq!(ed.game.levels[0].things.len(), before);
        ed.undo();
        assert_eq!(ed.game.levels[0].things.len(), before + 1);
        ed.redo();
        assert_eq!(ed.game.levels[0].things.len(), before);
    }

    #[test]
    fn the_big_paddle_can_be_selected_by_any_square_it_covers() {
        let mut g = Game::new("t");
        g.levels.push(Level::new("l"));
        let paddle = g.add_template("Paddle").unwrap();
        g.place(0, Thing::new(paddle, 4, 10));
        assert!(g.thing_covering(0, 4, 10).is_some());
        assert!(g.thing_covering(0, 6, 10).is_some(), "the paddle is 3 squares wide");
        assert!(g.thing_covering(0, 7, 10).is_none());
        assert!(g.erase(0, 6, 10));
    }
}
