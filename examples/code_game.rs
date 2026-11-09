//! Make a game with a few lines of Rust!
//!
//! Run it with:   cargo run --example code_game
//!
//! Each level is a picture made of letters:
//!   #  wall        P  you          C  coin        G  goal flag
//!   ^  spikes      S  spring       E  enemy that walks left and right
//!   U  enemy that moves up and down     H  ghost that follows you
//!   ?  sign        .  empty space
//!
//! You can also add your own kinds of objects (like a star) and give them a letter.
//! Try changing the letters, then run it again!

use spark::prelude::*;

fn main() {
    let mut game = Game::new("My First Game").lives(3).win_message("You did it! You made a game with Rust!").music(Music::Happy);

    // A new kind of object: a star that is worth 50 points.
    let star = game.add_template("Star").unwrap();
    game = game.rule(Rule::new(
        Trigger::Touch { a: PLAYER, b: star },
        &[Action::AddScore(50), Action::Destroy(Target::Other), Action::PlaySound(sounds::POWERUP)],
    ));

    game.level_with(
        "Level 1",
        &[
            "........................",
            "........................",
            "........................",
            "........................",
            "........................",
            ".........C.C.C.......G..",
            "......#######.......###.",
            "...........*............",
            "..?.....................",
            ".P..........E.....^^....",
            "########################",
        ],
        &[('*', star)],
    )
    .play();
    // Want to change the game with the mouse instead? Use .edit() in place of .play()
}
