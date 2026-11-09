//! Games that come with Spark, one for each kind of game you can make. Open them from the
//! "Examples" button, play them, change them, and learn from them.
//!
//! Each level below is a picture made of letters. This is also how you can write a
//! game in code. See `examples/code_game.rs`.

use crate::data::*;
use crate::sound::{Music, ids as snd};

pub struct Example {
    pub name: &'static str,
    pub genre: &'static str,
    pub blurb: &'static str,
    pub build: fn() -> Game,
}

pub const ALL: [Example; 7] = [
    Example {
        name: "Sky Run",
        genre: "Jumping game",
        blurb: "Run and jump through 3 levels: coins, spikes, enemies to squash, springs and a ghost.",
        build: sky_run,
    },
    Example {
        name: "Maze Garden",
        genre: "Top-down maze",
        blurb: "Walk in every direction, find the flag, grab the coins, dodge the baddies!",
        build: maze_garden,
    },
    Example {
        name: "Space Blaster",
        genre: "Shooting game",
        blurb: "Slide your spaceship and press Space to shoot the aliens. Watch out for their bombs!",
        build: space_blaster,
    },
    Example {
        name: "Brick Breaker",
        genre: "Ball and paddle",
        blurb: "Keep the ball in the air with your paddle and smash every block.",
        build: brick_breaker,
    },
    Example {
        name: "Star Catcher",
        genre: "Catching game",
        blurb: "Catch the falling stars in your basket. Dodge the bombs! Get 150 points to win.",
        build: star_catcher,
    },
    Example {
        name: "Crate Pusher",
        genre: "Puzzle",
        blurb: "Hop one square at a time and push every crate onto a target.",
        build: crate_pusher,
    },
    Example { name: "Empty Game", genre: "Make your own", blurb: "A blank level with a floor. Start building your own game!", build: Game::blank },
];

/// Make a level picture of `w` by `h` dots and put letters in it: `(column, row, letter)`.
fn grid(w: usize, h: usize, items: &[(usize, usize, char)]) -> Vec<String> {
    let mut rows: Vec<Vec<char>> = vec![vec!['.'; w]; h];
    for &(x, y, c) in items {
        rows[y][x] = c;
    }
    rows.into_iter().map(|r| r.into_iter().collect()).collect()
}

fn as_strs(rows: &[String]) -> Vec<&str> {
    rows.iter().map(|s| s.as_str()).collect()
}

// ---------------------------------------------------------------------------
// 1. A jumping game
// ---------------------------------------------------------------------------

pub fn sky_run() -> Game {
    let mut game = Game::new("Sky Run")
        .lives(3)
        .win_message("You made it across the sky! You are a champion!")
        .music(Music::Happy)
        .level(
            "Meet the coins",
            &[
                "........................",
                "........................",
                "........................",
                "........................",
                "........................",
                "...................C.G.C",
                "...................#####",
                "...............CC.......",
                "..............####......",
                "..........CC............",
                ".........####...........",
                ".P?..^^.................",
                "########################",
            ],
        )
        .level(
            "Stomp and spring",
            &[
                "............................",
                "............................",
                "............................",
                "............................",
                "............................",
                "....................CCC...G.",
                "...................#########",
                "............................",
                "............................",
                ".........CCC................",
                ".P.....#..E.#..S............",
                "############################",
            ],
        )
        .level(
            "Ghost house",
            &[
                "........................",
                "........................",
                "........................",
                "....................H...",
                "........................",
                "........................",
                "...........#............",
                "........................",
                "...........U............",
                "..CC.......C.C.....CC...",
                ".P.................^^.G.",
                "########################",
            ],
        );
    game.levels[0].sky = [150, 210, 255];
    game.levels[1].sky = [255, 205, 170];
    game.levels[2].sky = [70, 60, 120];
    game
}

// ---------------------------------------------------------------------------
// 2. A top-down maze
// ---------------------------------------------------------------------------

pub fn maze_garden() -> Game {
    let mut g = Game::new("Maze Garden").lives(3).win_message("You found your way out of the maze!").music(Music::Adventure);
    {
        let p = g.ty_mut(ids::PLAYER).unwrap();
        p.behavior = Behavior::TopDown;
        p.speed = 5.5;
        p.hit_bottom = false;
        p.hit_h = 0.8;
        p.name = "Explorer".into();
    }
    // Squashing monsters from above only makes sense in a jumping game.
    g.rules.retain(|r| !r.conditions.contains(&Condition::FromAbove));
    for r in &mut g.rules {
        r.conditions.retain(|c| *c != Condition::NotFromAbove);
    }
    let mut g = g.level(
        "The maze",
        &[
            "#######################",
            "#P......#...#H........#",
            "#######.#.#.#.#.#####.#",
            "#.......#.#.#...E.#...#",
            "#.#######.#.#####.#.###",
            "#.#C....#.#...#...#...#",
            "#.###.#.#.###.#.#####.#",
            "#.........#C#...#..C#.#",
            "###########.#####.###.#",
            "#C#...#.E......C#.....#",
            "#.#.#.#.###.#####.#####",
            "#...#....C#..........G#",
            "#######################",
        ],
    );
    g.levels[0].sky = [196, 232, 170];
    for t in &mut g.levels[0].things {
        if t.ty == ids::WALL {
            t.color = Some([70, 130, 80]);
        }
    }
    g
}

// ---------------------------------------------------------------------------
// 3. A shooting game
// ---------------------------------------------------------------------------

pub fn space_blaster() -> Game {
    let mut g = Game::new("Space Blaster").lives(3).win_message("You saved the galaxy!").music(Music::Action);
    g.types.clear();
    g.rules.clear();
    let ship = g.add_template("Spaceship").unwrap();
    let alien = g.add_template("Alien").unwrap();
    let bullet = g.add_template("Bullet").unwrap();
    let bomb = g.add_template("Bomb").unwrap();
    g.ty_mut(alien).unwrap().show_count = true;

    // Press Space to shoot.
    g.rules.push(Rule::new(
        Trigger::Key { key: Key::Space, mode: Press::Pressed, who: Some(ship) },
        &[Action::Spawn { ty: bullet, at: SpawnAt::Me, dir: Dir::Up }, Action::PlaySound(snd::SHOOT)],
    ));
    // Bullets destroy aliens.
    g.rules.push(Rule::new(
        Trigger::Touch { a: bullet, b: alien },
        &[Action::AddScore(10), Action::Destroy(Target::Me), Action::Destroy(Target::Other), Action::PlaySound(snd::BOOM)],
    ));
    // Every second, each alien has a small chance to drop a bomb.
    g.rules.push(Rule::new(Trigger::Every { secs: 1.0, who: Some(alien) }, &[Action::Spawn { ty: bomb, at: SpawnAt::Me, dir: Dir::Down }]).only_if(Condition::Chance(18)));
    // Bombs hurt the ship.
    g.rules.push(Rule::new(Trigger::Touch { a: bomb, b: ship }, &[Action::LoseLife, Action::Destroy(Target::Me), Action::PlaySound(snd::HURT)]));
    // Win when no aliens are left.
    g.rules.push(Rule::new(Trigger::NoneLeft(alien), &[Action::NextLevel, Action::PlaySound(snd::WIN)]));

    let legend = [('A', alien), ('X', ship)];
    let mut level1 = Vec::new();
    for row in [1usize, 3] {
        for x in [2usize, 5, 8, 11, 14, 17] {
            level1.push((x, row, 'A'));
        }
    }
    level1.push((10, 11, 'X'));
    let mut level2 = Vec::new();
    for row in [1usize, 3, 5] {
        for x in [1usize, 4, 7, 10, 13, 16] {
            level2.push((x, row, 'A'));
        }
    }
    level2.push((10, 11, 'X'));

    let (r1, r2) = (grid(20, 12, &level1), grid(20, 12, &level2));
    let mut g = g.level_with("Wave 1", &as_strs(&r1), &legend).level_with("Wave 2", &as_strs(&r2), &legend);
    // Aliens in different rows move at different speeds and have different colors.
    let colors = [[120, 230, 120], [255, 170, 80], [200, 130, 255]];
    for (li, level) in g.levels.iter_mut().enumerate() {
        level.sky = [14, 12, 44];
        for t in &mut level.things {
            if t.ty == alien {
                let row = (t.y / 2) as usize;
                t.speed = Some(1.6 + row as f32 * 0.7 + li as f32 * 0.8);
                t.color = Some(colors[row.min(2)]);
            }
        }
    }
    g
}

// ---------------------------------------------------------------------------
// 4. Ball and paddle
// ---------------------------------------------------------------------------

pub fn brick_breaker() -> Game {
    let mut g = Game::new("Brick Breaker").lives(3).win_message("Every block is smashed!").music(Music::Happy);
    g.types.clear();
    g.rules.clear();
    let paddle = g.add_template("Paddle").unwrap();
    let ball = g.add_template("Ball").unwrap();
    let block = g.add_template("Block").unwrap();
    g.ty_mut(block).unwrap().show_count = true;

    g.rules.push(Rule::new(Trigger::Touch { a: ball, b: paddle }, &[Action::Bounce, Action::PlaySound(snd::BLIP)]));
    g.rules.push(Rule::new(
        Trigger::Touch { a: ball, b: block },
        &[Action::Bounce, Action::Destroy(Target::Other), Action::AddScore(10), Action::PlaySound(snd::COIN)],
    ));
    g.rules.push(Rule::new(
        Trigger::LeavesLevel(ball),
        &[Action::LoseLife, Action::Spawn { ty: ball, at: SpawnAt::MyStart, dir: Dir::Default }, Action::PlaySound(snd::HURT)],
    ));
    g.rules.push(Rule::new(Trigger::NoneLeft(block), &[Action::NextLevel, Action::PlaySound(snd::WIN)]));

    let mut items = vec![(8, 10, 'B'), (6, 12, 'M')];
    for y in 2..6 {
        for x in 1..15 {
            items.push((x, y, 'K'));
        }
    }
    let rows = grid(16, 14, &items);
    let mut g = g.level_with("Smash them all", &as_strs(&rows), &[('B', ball), ('M', paddle), ('K', block)]);
    g.levels[0].sky = [26, 24, 66];
    let colors = [[255, 90, 100], [255, 170, 60], [250, 225, 70], [90, 210, 120]];
    for t in &mut g.levels[0].things {
        if t.ty == block {
            t.color = Some(colors[(t.y as usize - 2) % 4]);
        }
    }
    g
}

// ---------------------------------------------------------------------------
// 5. A catching game
// ---------------------------------------------------------------------------

pub fn star_catcher() -> Game {
    let mut g = Game::new("Star Catcher").lives(3).win_message("What a catch! You are a star catcher!").music(Music::Chill);
    g.types.clear();
    g.rules.clear();
    let basket = g.add_template("Paddle").unwrap();
    {
        let b = g.ty_mut(basket).unwrap();
        b.name = "Basket".into();
        b.size_w = 2.0;
        b.speed = 10.0;
        b.art = Art::Built(Pic::Crate);
        b.color = [210, 150, 80];
        b.hit_h = 0.8;
    }
    let star = g.add_template("Falling star").unwrap();
    let bomb = g.add_template("Bomb").unwrap();

    g.rules.push(Rule::new(Trigger::Every { secs: 0.7, who: None }, &[Action::Spawn { ty: star, at: SpawnAt::RandomTop, dir: Dir::Default }]));
    g.rules.push(Rule::new(Trigger::Every { secs: 1.6, who: None }, &[Action::Spawn { ty: bomb, at: SpawnAt::RandomTop, dir: Dir::Default }]));
    g.rules.push(Rule::new(Trigger::Touch { a: star, b: basket }, &[Action::AddScore(10), Action::Destroy(Target::Me), Action::PlaySound(snd::COIN)]));
    g.rules.push(Rule::new(Trigger::Touch { a: bomb, b: basket }, &[Action::LoseLife, Action::Destroy(Target::Me), Action::PlaySound(snd::HURT)]));
    g.rules.push(Rule::new(Trigger::Score(150), &[Action::WinGame, Action::PlaySound(snd::WIN)]));

    let rows = grid(14, 12, &[(6, 11, 'U')]);
    let mut g = g.level_with("Night sky", &as_strs(&rows), &[('U', basket)]);
    g.levels[0].sky = [30, 28, 92];
    g
}

// ---------------------------------------------------------------------------
// 6. A puzzle
// ---------------------------------------------------------------------------

pub fn crate_pusher() -> Game {
    let mut g = Game::new("Crate Pusher").lives(1).win_message("Every crate is in place. Puzzle solved!").music(Music::Chill);
    g.types.clear();
    g.rules.clear();
    let hopper = g.add_template("Hopper").unwrap();
    let wall = g.add_template("Wall").unwrap();
    let crate_ = g.add_template("Crate").unwrap();
    let target = g.add_template("Target").unwrap();
    let done = g.add_var("On target", 0, true);

    g.rules.push(Rule::new(Trigger::Start, &[Action::SetCounter { var: done, n: 0 }]));
    g.rules.push(Rule::new(Trigger::Touch { a: crate_, b: target }, &[Action::AddCounter { var: done, n: 1 }, Action::PlaySound(snd::POWERUP)]));
    g.rules.push(Rule::new(Trigger::StopTouch { a: crate_, b: target }, &[Action::AddCounter { var: done, n: -1 }]));
    g.rules.push(Rule::new(Trigger::Counter { var: done, n: 3 }, &[Action::NextLevel, Action::PlaySound(snd::WIN)]));

    let legend = [('H', hopper), ('#', wall), ('C', crate_), ('T', target)];
    let level1 = [
        "##########",
        "#........#",
        "#.H.C..T.#",
        "#........#",
        "#.C....T.#",
        "#........#",
        "#.C....T.#",
        "##########",
    ];
    let level2 = [
        "##########",
        "#........#",
        "#.H.C..T.#",
        "#.#....#.#",
        "#.C....T.#",
        "#..#.....#",
        "#.C....T.#",
        "##########",
    ];
    let mut g = g.level_with("Warm up", &level1, &legend).level_with("Pillars", &level2, &legend);
    for l in &mut g.levels {
        l.sky = [214, 200, 170];
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::*;
    use std::collections::VecDeque;

    fn run(rt: &mut Runtime, secs: f32, input: Input) {
        for _ in 0..(secs * 60.0) as usize {
            rt.update(1.0 / 60.0, &input);
        }
    }

    #[test]
    fn every_example_level_has_a_player() {
        for ex in ALL.iter() {
            let g = (ex.build)();
            assert!(!g.levels.is_empty(), "{} has no levels", ex.name);
            for (i, l) in g.levels.iter().enumerate() {
                assert!(g.player_start(i).is_some(), "{} / {} has no player", ex.name, l.name);
            }
        }
    }

    #[test]
    fn example_games_can_be_saved_and_loaded() {
        for ex in ALL.iter() {
            let g = (ex.build)();
            assert_eq!(Game::from_json(&g.to_json()).unwrap(), g, "{}", ex.name);
        }
    }

    #[test]
    fn every_rule_only_mentions_things_that_exist() {
        for ex in ALL.iter() {
            let g = (ex.build)();
            let ok_ty = |id: TypeId| g.ty(id).is_some();
            for r in &g.rules {
                let trig_ok = match &r.trigger {
                    Trigger::Touch { a, b } | Trigger::StopTouch { a, b } => ok_ty(*a) && ok_ty(*b),
                    Trigger::NoneLeft(t) | Trigger::Jumps(t) | Trigger::LeavesLevel(t) => ok_ty(*t),
                    Trigger::Key { who: Some(t), .. } | Trigger::Every { who: Some(t), .. } => ok_ty(*t),
                    Trigger::Counter { var, .. } => g.vars.iter().any(|v| v.id == *var),
                    _ => true,
                };
                assert!(trig_ok, "{}: bad trigger {:?}", ex.name, r.trigger);
                for a in &r.actions {
                    match a {
                        Action::Spawn { ty, .. } => assert!(ok_ty(*ty), "{}: spawn of a missing type", ex.name),
                        Action::PlaySound(s) => assert!(g.sound(*s).is_some(), "{}: missing sound", ex.name),
                        _ => {}
                    }
                }
            }
            for l in &g.levels {
                for t in &l.things {
                    assert!(ok_ty(t.ty), "{}: a thing has a type that does not exist", ex.name);
                }
            }
        }
    }

    /// Nothing should ever crash or go wrong with numbers, whatever buttons are pressed.
    #[test]
    fn mashing_buttons_never_breaks_any_example() {
        for ex in ALL.iter() {
            let g = (ex.build)();
            let mut rt = Runtime::new(&g, 0);
            let mut seed = 12345u32;
            let mut input = Input::default();
            for frame in 0..60 * 25 {
                if frame % 9 == 0 {
                    seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    input = Input::default();
                    for (i, k) in Key::ALL.iter().enumerate() {
                        let on = (seed >> (i + 8)) & 1 == 1;
                        input.set_held(*k, on);
                        input.set_pressed(*k, on && (seed >> 3) & 1 == 1);
                    }
                    input.restart_pressed = (seed >> 20) & 7 == 0;
                }
                rt.update(1.0 / 60.0, &input);
                rt.take_events();
                for o in &rt.objs {
                    assert!(o.pos.x.is_finite() && o.pos.y.is_finite(), "{}: {:?} went to {:?}", ex.name, o.tid, o.pos);
                }
            }
        }
    }

    #[test]
    fn the_maze_can_be_solved() {
        let g = maze_garden();
        let l = &g.levels[0];
        let (sx, sy) = g.player_start(0).unwrap();
        let goal = l.things.iter().find(|t| t.ty == ids::GOAL).unwrap();
        let wall = |x: i32, y: i32| l.things.iter().any(|t| t.ty == ids::WALL && t.x == x && t.y == y);
        let mut seen = vec![vec![false; l.width as usize]; l.height as usize];
        let mut q = VecDeque::new();
        q.push_back((sx, sy));
        seen[sy as usize][sx as usize] = true;
        while let Some((x, y)) = q.pop_front() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if l.in_bounds(nx, ny) && !seen[ny as usize][nx as usize] && !wall(nx, ny) {
                    seen[ny as usize][nx as usize] = true;
                    q.push_back((nx, ny));
                }
            }
        }
        assert!(seen[goal.y as usize][goal.x as usize]);
    }

    /// The spring in level 2 must be able to get you onto the high ledge with the goal.
    #[test]
    fn the_spring_in_sky_run_reaches_the_ledge() {
        let g = sky_run();
        let mut rt = Runtime::new(&g, 1);
        rt.lives = 99;
        let p = rt.objs.iter().position(|o| o.behavior.controlled()).unwrap();
        rt.objs[p].pos = Vec2::new(13.15, 10.0);
        let mut landed = false;
        for _ in 0..180 {
            rt.update(1.0 / 60.0, &Input::hold(&[Key::Right]));
            // (Objects that disappear shift the list, so look for the player every time.)
            let p = rt.objs.iter().position(|o| o.behavior.controlled()).unwrap();
            let hb = rt.hitbox(p);
            // Ledge top is row 6, so standing on it means feet at y = 6.
            if rt.objs[p].on_ground && (hb.y + hb.h - 6.0).abs() < 0.05 {
                landed = true;
            }
        }
        assert!(landed, "player never landed on the ledge");
    }

    /// Check that the first level's step platforms are all reachable with a normal jump.
    #[test]
    fn level_one_steps_are_reachable() {
        let g = sky_run();
        let l = &g.levels[0];
        let mut tops: Vec<i32> = l.things.iter().filter(|t| t.ty == ids::WALL).map(|t| t.y).collect();
        tops.sort();
        tops.dedup();
        for pair in tops.windows(2) {
            assert!(pair[1] - pair[0] <= 3, "gap between platform rows {pair:?} is too tall");
        }
    }

    #[test]
    fn a_simple_bot_can_shoot_aliens_in_space_blaster() {
        let g = space_blaster();
        let mut rt = Runtime::new(&g, 0);
        let alien = g.type_named("Alien").unwrap();
        for frame in 0..60 * 30 {
            let ship = rt.objs.iter().position(|o| o.behavior.controlled());
            let mut input = Input::default();
            if let Some(s) = ship {
                let sx = rt.objs[s].pos.x + 0.5;
                // Aim at the lowest alien.
                let target = rt.objs.iter().filter(|o| o.tid == alien).max_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).unwrap());
                if let Some(t) = target {
                    let tx = t.pos.x + 0.5;
                    input.set_held(Key::Left, tx < sx - 0.3);
                    input.set_held(Key::Right, tx > sx + 0.3);
                }
                if frame % 20 == 0 {
                    input.set_pressed(Key::Space, true);
                    input.set_held(Key::Space, true);
                }
            }
            rt.update(1.0 / 60.0, &input);
        }
        assert!(rt.score >= 30, "the bot should have hit a few aliens, score = {}", rt.score);
    }

    #[test]
    fn a_bot_can_break_bricks() {
        let g = brick_breaker();
        let mut rt = Runtime::new(&g, 0);
        let ball = g.type_named("Ball").unwrap();
        for _ in 0..60 * 40 {
            let mut input = Input::default();
            let b = rt.objs.iter().find(|o| o.tid == ball).map(|o| o.pos.x + o.size.x / 2.0);
            let p = rt.objs.iter().find(|o| o.behavior.controlled()).map(|o| o.pos.x + o.size.x / 2.0);
            if let (Some(b), Some(p)) = (b, p) {
                input.set_held(Key::Left, b < p - 0.2);
                input.set_held(Key::Right, b > p + 0.2);
            }
            rt.update(1.0 / 60.0, &input);
        }
        assert!(rt.score >= 100, "score = {}", rt.score);
        assert!(rt.lives >= 1);
    }

    #[test]
    fn a_bot_can_catch_stars() {
        let g = star_catcher();
        let mut rt = Runtime::new(&g, 0);
        let star = g.type_named("Falling star").unwrap();
        for _ in 0..60 * 40 {
            let mut input = Input::default();
            let p = rt.objs.iter().find(|o| o.behavior.controlled()).map(|o| o.pos.x + o.size.x / 2.0);
            let s = rt.objs.iter().filter(|o| o.tid == star).max_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).unwrap()).map(|o| o.pos.x + o.size.x / 2.0);
            if let (Some(p), Some(s)) = (p, s) {
                input.set_held(Key::Left, s < p - 0.2);
                input.set_held(Key::Right, s > p + 0.2);
            }
            rt.update(1.0 / 60.0, &input);
            if rt.state == State::Won {
                break;
            }
        }
        assert!(rt.score >= 100 || rt.state == State::Won, "score = {}", rt.score);
    }

    /// Solve the first Crate Pusher level with a list of key presses.
    #[test]
    fn crate_pusher_level_one_can_be_solved() {
        let g = crate_pusher();
        let mut rt = Runtime::new(&g, 0);
        let mut moves = Vec::new();
        let mut add = |k: Key, n: usize| moves.extend(std::iter::repeat_n(k, n));
        // First crate: hop right once, then push it three times onto the target.
        add(Key::Right, 4);
        // Second crate.
        add(Key::Left, 5);
        add(Key::Down, 2);
        add(Key::Right, 5);
        // Third crate.
        add(Key::Left, 5);
        add(Key::Down, 2);
        add(Key::Right, 5);
        for k in moves {
            rt.update(1.0 / 60.0, &Input::tap(&[k]));
            run(&mut rt, 0.22, Input::default());
        }
        assert!(matches!(rt.state, State::LevelDone(_)), "the puzzle should be solved (counter = {:?}, state = {:?})", rt.vars, rt.state);
    }

    #[test]
    fn crate_pusher_levels_have_as_many_targets_as_crates() {
        let g = crate_pusher();
        for l in &g.levels {
            let crates = l.things.iter().filter(|t| g.ty(t.ty).is_some_and(|o| o.behavior == Behavior::Pushable)).count();
            let targets = l.things.iter().filter(|t| g.ty(t.ty).is_some_and(|o| o.name == "Target")).count();
            assert_eq!((crates, targets), (3, 3));
        }
    }
}
