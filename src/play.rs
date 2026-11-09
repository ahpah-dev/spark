//! Playing a game in its own window (the "player", without the editor).

use crate::audio::Audio;
use crate::data::{Game, Key as GameKey};
use crate::engine::{Event, Input, Runtime};
use crate::render;
use egui::Key;

/// Read the keyboard: arrow keys or WASD to move, Space / Up / W to jump, Z X Enter for extras.
pub fn read_input(ctx: &egui::Context) -> Input {
    ctx.input(|i| {
        let mut input = Input::default();
        let map: [(GameKey, &[Key]); 8] = [
            (GameKey::Left, &[Key::ArrowLeft, Key::A]),
            (GameKey::Right, &[Key::ArrowRight, Key::D]),
            (GameKey::Up, &[Key::ArrowUp, Key::W]),
            (GameKey::Down, &[Key::ArrowDown, Key::S]),
            (GameKey::Space, &[Key::Space]),
            (GameKey::Z, &[Key::Z]),
            (GameKey::X, &[Key::X]),
            (GameKey::Enter, &[Key::Enter]),
        ];
        for (gk, keys) in map {
            input.set_held(gk, keys.iter().any(|k| i.key_down(*k)));
            input.set_pressed(gk, keys.iter().any(|k| i.key_pressed(*k)));
        }
        input.restart_pressed = i.key_pressed(Key::R) || i.key_pressed(Key::Enter);
        input
    })
}

/// Send the sounds a game made this frame to the speakers.
pub fn play_events(rt: &mut Runtime, audio: &mut Option<Audio>) {
    let events = rt.take_events();
    if let Some(a) = audio {
        a.sfx_volume = rt.game.sfx_volume;
        for e in events {
            let Event::Sound(id) = e;
            if let Some(s) = rt.game.sound(id) {
                a.play(s);
            }
        }
    }
}

/// A window that only plays a game.
pub struct PlayerApp {
    runtime: Runtime,
    audio: Option<Audio>,
}

impl PlayerApp {
    pub fn new(game: &Game) -> Self {
        let mut audio = Audio::new();
        if let Some(a) = &mut audio {
            a.music_volume = game.music_volume;
            a.set_music(game.music);
        }
        PlayerApp { runtime: Runtime::new(game, 0), audio }
    }
}

impl eframe::App for PlayerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let input = read_input(&ctx);
        let dt = ctx.input(|i| i.unstable_dt);
        self.runtime.update(dt, &input);
        play_events(&mut self.runtime, &mut self.audio);
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter_at(rect);
            render::draw_game(&painter, rect, &self.runtime, "Esc = quit");
        });
        ctx.request_repaint();
    }
}

/// Open a window and play `game`. Returns when the window is closed.
pub fn run_game(game: Game) -> eframe::Result {
    let title = game.title.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]).with_min_inner_size([480.0, 360.0]).with_title(title.clone()).with_icon(crate::style::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            crate::style::apply(&cc.egui_ctx);
            Ok(Box::new(PlayerApp::new(&game)))
        }),
    )
}
