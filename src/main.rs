//! Spark Game Maker.
//!
//! * `spark`                 opens the editor
//! * `spark play my.spark`   plays a game
//! * `spark my.spark`        opens a game in the editor
//! * `spark examples`        saves the example games into the `games` folder
//!
//! If a file called `game.spark` sits next to the program, that game is played
//! straight away. That is how games made with the "Share" button work.

// In the finished program (release build on Windows) don't open a black console window behind the game.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use spark::data::Game;
use std::path::{Path, PathBuf};

fn usage() {
    println!(
        "Spark Game Maker\n\n\
         spark                  open the game maker\n\
         spark play FILE        play a .spark game\n\
         spark FILE             open a .spark game in the game maker\n\
         spark examples         save the example games into the games folder\n"
    );
}

fn load_or_exit(path: &Path) -> Game {
    match Game::load(path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn bundled_game() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let p = exe.parent()?.join("game.spark");
    p.exists().then_some(p)
}

/// A program without a console window can't print... unless it borrows the console of the terminal
/// that started it. We do that so `spark examples` and `spark help` still show their messages.
#[cfg(all(windows, not(debug_assertions)))]
fn attach_to_terminal() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    // u32::MAX means "the parent process".
    unsafe {
        AttachConsole(u32::MAX);
    }
}

#[cfg(not(all(windows, not(debug_assertions))))]
fn attach_to_terminal() {}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        attach_to_terminal();
    }
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        [] => match bundled_game() {
            Some(p) => load_or_exit(&p).play(),
            None => spark::run_editor(),
        },
        ["help" | "--help" | "-h"] => usage(),
        ["examples"] => {
            for ex in spark::examples::ALL.iter() {
                let game = (ex.build)();
                let path = spark::paths::games_dir().join(format!("{}.spark", spark::data::safe_file_name(ex.name)));
                match game.save(&path) {
                    Ok(()) => println!("Saved {}", path.display()),
                    Err(e) => eprintln!("{e}"),
                }
            }
        }
        ["play", file] => load_or_exit(Path::new(file)).play(),
        [file] if Path::new(file).exists() => load_or_exit(Path::new(file)).edit(),
        _ => usage(),
    }
}
