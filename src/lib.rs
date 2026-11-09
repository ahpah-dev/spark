//! # Spark: a game maker for young creators
//!
//! Spark has two ways to make a game:
//!
//! 1. **The editor** (just run the program!). Click to place things, draw your own pictures,
//!    set up rules, make sounds, then press PLAY.
//! 2. **A few lines of Rust**, using letters to draw your level:
//!
//! ```no_run
//! use spark::prelude::*;
//!
//! fn main() {
//!     Game::new("My First Game")
//!         .level("Level 1", &[
//!             "........................",
//!             "........................",
//!             "........................",
//!             "........................",
//!             "........................",
//!             ".....C.C.C.....G........",
//!             "....#######....####.....",
//!             ".P......................",
//!             "########################",
//!         ])
//!         .play();
//! }
//! ```
//!
//! The pieces:
//! * [`data`]: what a game is made of (object types, levels, rules, sounds). Games are saved as `.spark` files.
//! * [`engine`]: movement, touching, rules and winning. No window needed.
//! * [`sound`] and [`audio`]: sound effects and music, made from numbers, and playing them.
//! * [`render`]: how everything is drawn.
//! * [`editor`]: the visual game maker.

pub mod audio;
pub mod data;
pub mod editor;
pub mod engine;
pub mod examples;
pub mod legacy;
pub mod paths;
pub mod play;
pub mod render;
pub mod sound;
pub mod style;

pub use data::{Action, Behavior, Condition, Dir, Game, Key, Level, ObjectType, Press, Rule, SpawnAt, Target, Thing, Trigger, TypeId};

/// Everything you need for a small game. Use it with `use spark::prelude::*;`.
pub mod prelude {
    pub use crate::data::ids::*;
    pub use crate::data::{Action, Behavior, Condition, Dir, Game, Key, Level, ObjectType, Press, Rule, SpawnAt, Target, Thing, Trigger, TypeId};
    pub use crate::sound::{Music, Preset, ids as sounds};
    pub use crate::{play_game as play, run_editor};
}

/// Open the visual game maker.
pub fn run_editor() {
    if let Err(e) = editor::run_editor() {
        eprintln!("Spark could not open its window: {e}");
    }
}

/// Open a window and play a game. (Also available as `game.play()`.)
pub fn play_game(game: Game) {
    if let Err(e) = play::run_game(game) {
        eprintln!("Spark could not open its window: {e}");
    }
}

impl Game {
    /// Open a window and play this game!
    pub fn play(self) {
        play_game(self);
    }

    /// Open this game in the editor so you can change it.
    pub fn edit(self) {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1420.0, 840.0]).with_title("Spark Game Maker").with_icon(style::window_icon()),
            ..Default::default()
        };
        let result = eframe::run_native(
            "Spark Game Maker",
            options,
            Box::new(move |cc| {
                style::apply(&cc.egui_ctx);
                Ok(Box::new(editor::Editor::with_game(self)))
            }),
        );
        if let Err(e) = result {
            eprintln!("Spark could not open its window: {e}");
        }
    }
}
