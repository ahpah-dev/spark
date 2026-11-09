//! A tiny shooting game, made only from rules!
//!
//! Run it with:   cargo run --example code_shooter
//!
//! Left and right arrows move the ship. Space shoots.
//! Read the rules: each one says WHEN something happens, and what to DO.

use spark::prelude::*;

fn main() {
    let mut game = Game::new("Tiny Shooter").lives(3).music(Music::Action);

    // Start with an empty game, then add only the objects we need.
    game.types.clear();
    game.rules.clear();
    let ship = game.add_template("Spaceship").unwrap();
    let alien = game.add_template("Alien").unwrap();
    let bullet = game.add_template("Bullet").unwrap();

    // WHEN you press Space, DO: a bullet appears on the ship, flying up.
    game = game.rule(Rule::new(
        Trigger::Key { key: Key::Space, mode: Press::Pressed, who: Some(ship) },
        &[Action::Spawn { ty: bullet, at: SpawnAt::Me, dir: Dir::Up }, Action::PlaySound(sounds::SHOOT)],
    ));
    // WHEN a bullet touches an alien, DO: score, and both disappear.
    game = game.rule(Rule::new(
        Trigger::Touch { a: bullet, b: alien },
        &[Action::AddScore(10), Action::Destroy(Target::Me), Action::Destroy(Target::Other), Action::PlaySound(sounds::BOOM)],
    ));
    // WHEN there are no aliens left, DO: win!
    game = game.rule(Rule::new(Trigger::NoneLeft(alien), &[Action::WinGame, Action::PlaySound(sounds::WIN)]));

    game.level_with(
        "Space",
        &[
            "....................",
            "..A..A..A..A..A.....",
            "....................",
            "..A..A..A..A..A.....",
            "....................",
            "....................",
            "....................",
            "....................",
            "....................",
            "....................",
            "....................",
            "..........X.........",
        ],
        &[('A', alien), ('X', ship)],
    )
    .play();
}
