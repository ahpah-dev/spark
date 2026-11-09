//! Tests for the engine: these run games with no window, to make sure they behave.

use super::*;
use crate::sound::ids as snd;

fn run(rt: &mut Runtime, seconds: f32, input: Input) {
    for _ in 0..(seconds * 60.0) as usize {
        rt.update(1.0 / 60.0, &input);
    }
}

fn game(rows: &[&str]) -> Game {
    Game::new("test").level("L1", rows)
}

fn right() -> Input {
    Input::hold(&[Key::Right])
}

fn player_index(rt: &Runtime) -> usize {
    rt.objs.iter().position(|o| o.behavior.controlled()).unwrap()
}

fn player(rt: &Runtime) -> &Obj {
    &rt.objs[player_index(rt)]
}

fn feet(rt: &Runtime) -> f32 {
    let p = rt.hitbox(player_index(rt));
    p.y + p.h
}

// A level with plenty of room (levels are at least 8 wide and 6 tall).
const FLAT: [&str; 6] = ["..........", "..........", "..........", "..........", ".P........", "##########"];

// ---- the basics ----

#[test]
fn player_falls_and_lands_on_the_floor() {
    let mut rt = Runtime::new(&game(&FLAT), 0);
    run(&mut rt, 1.0, Input::default());
    assert!(player(&rt).on_ground);
    assert!((feet(&rt) - 5.0).abs() < 0.01, "feet = {}", feet(&rt));
}

#[test]
fn player_walks_right_and_stops_at_wall() {
    let rows = ["..........", "..........", "..........", "..........", ".P.....#..", "##########"];
    let mut rt = Runtime::new(&game(&rows), 0);
    run(&mut rt, 3.0, right());
    let hb = rt.hitbox(player_index(&rt));
    assert!(hb.x + hb.w <= 7.0 + 0.001);
    assert!(hb.x > 5.5);
}

#[test]
fn jump_reaches_about_the_requested_height() {
    let mut g = game(&FLAT);
    g.ty_mut(ids::PLAYER).unwrap().jump_height = 3.0;
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 0.5, Input::default());
    let floor_y = feet(&rt);
    let mut highest = floor_y;
    let mut held = Input::tap(&[Key::Space]);
    for _ in 0..90 {
        rt.update(1.0 / 60.0, &held);
        held = Input::hold(&[Key::Space]);
        highest = highest.min(feet(&rt));
    }
    let height = floor_y - highest;
    assert!(height > 2.7 && height < 3.2, "jumped {height} squares");
}

#[test]
fn short_tap_makes_a_shorter_jump() {
    let mut rt = Runtime::new(&game(&FLAT), 0);
    run(&mut rt, 0.5, Input::default());
    let floor_y = feet(&rt);
    let mut highest = floor_y;
    rt.update(1.0 / 60.0, &Input::tap(&[Key::Space]));
    for _ in 0..90 {
        rt.update(1.0 / 60.0, &Input::default());
        highest = highest.min(feet(&rt));
    }
    assert!(floor_y - highest < 2.0);
}

#[test]
fn collecting_a_coin_gives_points_and_a_sound() {
    let rows = ["..........", "..........", "..........", "..........", ".PC.......", "##########"];
    let mut rt = Runtime::new(&game(&rows), 0);
    assert_eq!(rt.count_left(ids::COIN), 1);
    run(&mut rt, 1.0, right());
    assert_eq!(rt.score, 10);
    assert_eq!(rt.count_left(ids::COIN), 0);
    assert!(rt.take_events().contains(&Event::Sound(snd::COIN)));
}

#[test]
fn jumping_plays_the_jump_sound() {
    let mut rt = Runtime::new(&game(&FLAT), 0);
    run(&mut rt, 0.5, Input::default());
    rt.take_events();
    rt.update(1.0 / 60.0, &Input::tap(&[Key::Space]));
    assert!(rt.take_events().contains(&Event::Sound(snd::JUMP)));
}

#[test]
fn spikes_cost_lives_until_game_over() {
    let rows = ["..........", "..........", "..........", "..........", ".P.^......", "##########"];
    let mut rt = Runtime::new(&game(&rows).lives(2), 0);
    run(&mut rt, 0.35, right());
    assert_eq!(rt.lives, 1);
    assert_eq!(rt.state, State::Playing);
    assert!(player(&rt).pos.x < 2.0, "player should be back at the start");
    run(&mut rt, 2.0, Input::default());
    run(&mut rt, 1.0, right());
    assert_eq!(rt.lives, 0);
    assert_eq!(rt.state, State::GameOver);
    let restart = Input { restart_pressed: true, ..Default::default() };
    rt.update(1.0 / 60.0, &restart);
    assert_eq!(rt.state, State::Playing);
    assert_eq!(rt.lives, 2);
}

#[test]
fn reaching_the_goal_moves_to_next_level_then_wins() {
    let rows = ["..........", "..........", "..........", "..........", ".PG.......", "##########"];
    let g = Game::new("two").level("A", &rows).level("B", &rows);
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 0.6, right());
    assert!(matches!(rt.state, State::LevelDone(_)));
    run(&mut rt, 2.0, Input::default());
    assert_eq!(rt.level, 1);
    assert_eq!(rt.state, State::Playing);
    run(&mut rt, 0.6, right());
    run(&mut rt, 2.0, Input::default());
    assert_eq!(rt.state, State::Won);
}

fn still_enemies(g: &mut Game) {
    for t in &mut g.levels[0].things {
        if t.ty == ids::ENEMY {
            t.behavior = Some(Behavior::Still);
        }
    }
}

#[test]
fn jumping_on_an_enemy_squashes_it() {
    let rows = ["..........", "..........", ".P........", "..........", "...E......", "##########"];
    let mut g = game(&rows);
    still_enemies(&mut g);
    g.levels[0].things.retain(|t| t.ty != ids::PLAYER);
    g.place(0, Thing::new(ids::PLAYER, 3, 1));
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 1.0, Input::default());
    assert_eq!(rt.lives, 3, "should not lose a life");
    assert_eq!(rt.score, 20);
    assert_eq!(rt.count_left(ids::ENEMY), 0);
}

#[test]
fn walking_into_an_enemy_hurts() {
    let rows = ["..........", "..........", "..........", "..........", ".P.E......", "##########"];
    let mut g = game(&rows);
    still_enemies(&mut g);
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 1.0, right());
    assert_eq!(rt.lives, 2);
}

#[test]
fn spring_launches_the_player_high() {
    let rows = ["..........", "..........", "..........", "..........", ".PS.......", "##########"];
    let mut rt = Runtime::new(&game(&rows), 0);
    let mut highest = f32::MAX;
    for _ in 0..120 {
        rt.update(1.0 / 60.0, &right());
        highest = highest.min(player(&rt).pos.y);
    }
    assert!(highest < 5.0 - 3.2 - 1.0, "highest y = {highest}");
}

#[test]
fn falling_out_of_the_world_costs_a_life() {
    let rows = ["..........", "..........", "..........", "..........", ".P........", ".........."];
    let mut rt = Runtime::new(&game(&rows), 0);
    run(&mut rt, 2.0, Input::default());
    assert!(rt.lives < 3);
}

#[test]
fn enemies_turn_around_at_walls() {
    let rows = ["..........", "..........", "..........", "..........", "#.E.....#.", "##########"];
    let mut rt = Runtime::new(&game(&rows), 0);
    run(&mut rt, 6.0, Input::default());
    let i = rt.objs.iter().position(|o| o.tid == ids::ENEMY).unwrap();
    let hb = rt.hitbox(i);
    assert!(hb.x >= 0.99 && hb.x + hb.w <= 8.01, "enemy x = {}", rt.objs[i].pos.x);
}

#[test]
fn top_down_moves_in_all_directions() {
    let rows = ["##########", "#........#", "#........#", "#...P....#", "#........#", "##########"];
    let mut g = game(&rows);
    g.ty_mut(ids::PLAYER).unwrap().behavior = Behavior::TopDown;
    let mut rt = Runtime::new(&g, 0);
    let y0 = player(&rt).pos.y;
    run(&mut rt, 0.2, Input::hold(&[Key::Up]));
    assert!(player(&rt).pos.y < y0);
    run(&mut rt, 3.0, Input::hold(&[Key::Up, Key::Left]));
    let hb = rt.hitbox(player_index(&rt));
    assert!(hb.x >= 1.0 - 0.001 && hb.y >= 1.0 - 0.001, "box = {hb:?}");
}

#[test]
fn custom_rules_work() {
    let rows = ["..........", "..........", "..........", "..........", ".PC.......", "##########"];
    let g = game(&rows).when_touching(ids::PLAYER, ids::COIN, &[Action::AddScore(500), Action::Destroy(Target::Other)]);
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 1.0, right());
    assert_eq!(rt.score, 500);
}

#[test]
fn sign_shows_its_text() {
    let rows = ["..........", "..........", "..........", "..........", ".P?.......", "##########"];
    let mut rt = Runtime::new(&game(&rows), 0);
    run(&mut rt, 0.5, Input::default());
    assert!(rt.sign_text.is_some());
}

#[test]
fn games_survive_saving_and_loading() {
    let mut g = game(&FLAT).lives(5);
    let mut sprite = Sprite::new(8, 8);
    sprite.set(0, 1, 1, [255, 0, 0, 255]);
    sprite.refresh();
    let id = g.add_template("Blank").unwrap();
    g.ty_mut(id).unwrap().art = Art::Pixels(sprite);
    let back = Game::from_json(&g.to_json()).unwrap();
    assert_eq!(g, back);
}

#[test]
fn ascii_levels_are_read_correctly() {
    let g = Game::new("x");
    let l = g.draw_level("x", &["#P.C", "E^SG"], &[]);
    assert!(l.things.iter().any(|t| t.ty == ids::PLAYER && t.x == 1 && t.y == l.height - 2));
    assert!(l.things.iter().any(|t| t.ty == ids::GOAL));
    assert!(l.things.iter().any(|t| t.ty == ids::SPIKES));
}

#[test]
fn placing_a_second_player_moves_the_first() {
    let mut g = Game::new("x");
    g.levels.push(Level::new("x"));
    g.place(0, Thing::new(ids::PLAYER, 1, 1));
    g.place(0, Thing::new(ids::PLAYER, 5, 5));
    assert_eq!(g.levels[0].things.iter().filter(|t| t.ty == ids::PLAYER).count(), 1);
    assert_eq!(g.player_start(0), Some((5, 5)));
}

// ---- other kinds of games ----

#[test]
fn shooting_bullets_destroys_aliens() {
    let mut g = Game::new("shooter");
    let ship = g.add_template("Spaceship").unwrap();
    let alien = g.add_template("Alien").unwrap();
    let bullet = g.add_template("Bullet").unwrap();
    g.ty_mut(alien).unwrap().behavior = Behavior::Still;
    g.rules.clear();
    g.rules.push(Rule::new(
        Trigger::Key { key: Key::Space, mode: Press::Pressed, who: Some(ship) },
        &[Action::Spawn { ty: bullet, at: SpawnAt::Me, dir: Dir::Up }],
    ));
    g.rules.push(Rule::new(
        Trigger::Touch { a: bullet, b: alien },
        &[Action::AddScore(5), Action::Destroy(Target::Me), Action::Destroy(Target::Other)],
    ));
    let g = g.level_with(
        "L",
        &["..........", "....A.....", "..........", "..........", "..........", "....X....."],
        &[('A', alien), ('X', ship)],
    );
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 0.2, Input::default());
    rt.update(1.0 / 60.0, &Input::tap(&[Key::Space]));
    assert_eq!(rt.count_left(bullet), 1, "a bullet should appear");
    run(&mut rt, 1.0, Input::default());
    assert_eq!(rt.score, 5);
    assert_eq!(rt.count_left(alien), 0);
    assert_eq!(rt.count_left(bullet), 0);
}

#[test]
fn bullets_that_leave_the_level_disappear() {
    let mut g = Game::new("shooter");
    let ship = g.add_template("Spaceship").unwrap();
    let bullet = g.add_template("Bullet").unwrap();
    g.rules.clear();
    g.rules.push(Rule::new(
        Trigger::Key { key: Key::Space, mode: Press::Pressed, who: Some(ship) },
        &[Action::Spawn { ty: bullet, at: SpawnAt::Me, dir: Dir::Up }],
    ));
    let g = g.level_with("L", &["..........", "..........", "..........", "..........", "..........", "....X....."], &[('X', ship)]);
    let mut rt = Runtime::new(&g, 0);
    rt.update(1.0 / 60.0, &Input::tap(&[Key::Space]));
    run(&mut rt, 1.5, Input::default());
    assert_eq!(rt.count_left(bullet), 0);
}

#[test]
fn a_ball_bounces_inside_a_box() {
    let mut g = Game::new("ball");
    let ball = g.add_template("Ball").unwrap();
    g.rules.clear();
    let g = g.level_with("L", &["#........#", "#........#", "#........#", "#........#", "#...B....#", "##########"], &[('B', ball)]);
    let mut rt = Runtime::new(&g, 0);
    for _ in 0..600 {
        rt.update(1.0 / 60.0, &Input::default());
        let b = rt.objs.iter().find(|o| o.tid == ball).expect("ball must stay alive");
        assert!(b.pos.x >= 0.99 && b.pos.x <= 9.0 && b.pos.y >= -0.01 && b.pos.y <= 5.0, "ball at {:?}", b.pos);
        let speed = (b.vel.x * b.vel.x + b.vel.y * b.vel.y).sqrt();
        assert!((speed - b.speed).abs() < 0.1);
    }
}

#[test]
fn the_ball_bounces_off_the_paddle() {
    let mut g = Game::new("breakout");
    let ball = g.add_template("Ball").unwrap();
    let paddle = g.add_template("Paddle").unwrap();
    g.rules.clear();
    g.rules.push(Rule::new(Trigger::Touch { a: ball, b: paddle }, &[Action::Bounce]));
    let g = g.level_with(
        "L",
        &["..........", "....B.....", "..........", "..........", "...M......", ".........."],
        &[('B', ball), ('M', paddle)],
    );
    let mut rt = Runtime::new(&g, 0);
    let bi = rt.objs.iter().position(|o| o.tid == ball).unwrap();
    rt.objs[bi].vel = Vec2::new(0.0, 8.0);
    // Fall, bounce, and look just after the bounce (later it would bounce off the ceiling too).
    run(&mut rt, 0.5, Input::default());
    let b = rt.objs.iter().find(|o| o.tid == ball).expect("ball alive");
    let m = rt.objs.iter().find(|o| o.tid == paddle).unwrap();
    assert!(b.pos.y < m.pos.y, "ball should be above the paddle after bouncing: ball {:?} paddle {:?}", b.pos, m.pos);
    assert!(b.vel.y < 0.0);
}

#[test]
fn finishing_when_none_are_left() {
    let mut g = Game::new("collect");
    g.rules.push(Rule::new(Trigger::NoneLeft(ids::COIN), &[Action::NextLevel]));
    let rows = ["..........", "..........", "..........", "..........", ".PC.......", "##########"];
    let g = g.level("A", &rows).level("B", &rows);
    let mut rt = Runtime::new(&g, 0);
    assert_eq!(rt.level, 0);
    run(&mut rt, 1.0, right());
    run(&mut rt, 2.0, Input::default());
    assert_eq!(rt.level, 1);
}

#[test]
fn a_key_opens_a_door() {
    let mut g = Game::new("keys");
    let key = g.add_template("Key").unwrap();
    let door = g.add_template("Door").unwrap();
    let keys = g.add_var("Keys", 0, true);
    g.rules.clear();
    g.rules.push(Rule::new(
        Trigger::Touch { a: ids::PLAYER, b: key },
        &[Action::AddCounter { var: keys, n: 1 }, Action::Destroy(Target::Other)],
    ));
    g.rules.push(
        Rule::new(Trigger::Touch { a: ids::PLAYER, b: door }, &[Action::Destroy(Target::Other)]).only_if(Condition::CounterAtLeast { var: keys, n: 1 }),
    );
    g.rules.push(Rule::new(Trigger::Touch { a: ids::PLAYER, b: ids::GOAL }, &[Action::NextLevel]));
    let with_key = g.clone().level_with(
        "A",
        &["..........", "..........", "..........", "..........", ".PKD...G..", "##########"],
        &[('K', key), ('D', door)],
    );
    let mut rt = Runtime::new(&with_key, 0);
    run(&mut rt, 2.0, right());
    assert!(matches!(rt.state, State::LevelDone(_)), "with a key the player gets through");

    let no_key = g.level_with("A", &["..........", "..........", "..........", "..........", ".P.D...G..", "##########"], &[('K', key), ('D', door)]);
    let mut rt = Runtime::new(&no_key, 0);
    run(&mut rt, 2.0, right());
    assert_eq!(rt.state, State::Playing);
    assert!(player(&rt).pos.x < 3.0, "the door must block the way");
}

#[test]
fn timers_spawn_things() {
    let mut g = Game::new("catch");
    let star = g.add_template("Falling star").unwrap();
    g.rules.clear();
    g.rules.push(Rule::new(Trigger::Every { secs: 0.5, who: None }, &[Action::Spawn { ty: star, at: SpawnAt::RandomTop, dir: Dir::Default }]));
    let g = g.level("L", &FLAT);
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 1.6, Input::default());
    let n = rt.count_left(star);
    assert!((1..=4).contains(&n), "{n} stars");
}

#[test]
fn the_hopper_pushes_crates_but_not_into_walls() {
    let mut g = Game::new("sokoban");
    let hopper = g.add_template("Hopper").unwrap();
    let crate_ = g.add_template("Crate").unwrap();
    g.rules.clear();
    let g = g.level_with("L", &["..........", "..........", "..........", "..........", ".HC.....#.", "##########"], &[('H', hopper), ('C', crate_)]);
    let mut rt = Runtime::new(&g, 0);
    for _ in 0..2 {
        rt.update(1.0 / 60.0, &Input::tap(&[Key::Right]));
        run(&mut rt, 0.4, Input::default());
    }
    let c = rt.objs.iter().find(|o| o.tid == crate_).unwrap();
    let h = rt.objs.iter().find(|o| o.tid == hopper).unwrap();
    assert!((c.pos.x - 4.0).abs() < 0.01 && (h.pos.x - 3.0).abs() < 0.01, "crate {:?} hopper {:?}", c.pos, h.pos);
    // Keep pushing: the crate stops just before the wall (column 7).
    for _ in 0..8 {
        rt.update(1.0 / 60.0, &Input::tap(&[Key::Right]));
        run(&mut rt, 0.4, Input::default());
    }
    let c = rt.objs.iter().find(|o| o.tid == crate_).unwrap();
    assert!((c.pos.x - 7.0).abs() < 0.01, "crate {:?}", c.pos);
}

#[test]
fn a_ball_that_falls_out_can_be_respawned() {
    let mut g = Game::new("lost ball");
    let ball = g.add_template("Ball").unwrap();
    g.rules.clear();
    g.rules.push(Rule::new(
        Trigger::LeavesLevel(ball),
        &[Action::LoseLife, Action::Spawn { ty: ball, at: SpawnAt::MyStart, dir: Dir::Default }],
    ));
    let g = g.level_with("L", &["#........#", "#........#", "#........#", "#........#", "#...B....#", ".........."], &[('B', ball)]);
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 4.0, Input::default());
    assert!(rt.lives < 3, "the ball should have fallen out at least once");
}

#[test]
fn score_and_counter_triggers_fire_once_when_reached() {
    let mut g = Game::new("goals");
    let c = g.add_var("Stars", 0, true);
    g.rules.clear();
    g.rules.push(Rule::new(Trigger::Start, &[Action::AddScore(100)]));
    g.rules.push(Rule::new(Trigger::Score(100), &[Action::AddCounter { var: c, n: 1 }]));
    let g = g.level("L", &FLAT);
    let mut rt = Runtime::new(&g, 0);
    run(&mut rt, 1.0, Input::default());
    assert_eq!(rt.vars[0], 1);
}

#[test]
fn removing_a_type_cleans_up_rules_and_things() {
    let mut g = game(&FLAT);
    g.remove_type(ids::COIN);
    assert!(g.rules.iter().all(|r| !r.trigger.mentions(ids::COIN)));
    let mut g2 = Game::new("x");
    let alien = g2.add_template("Alien").unwrap();
    g2.rules.push(Rule::new(Trigger::Start, &[Action::Spawn { ty: alien, at: SpawnAt::RandomTop, dir: Dir::Default }]));
    g2.remove_type(alien);
    assert!(g2.rules.iter().all(|r| r.actions.iter().all(|a| !a.mentions(alien))));
}
