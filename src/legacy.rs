//! Opening games saved by the first version of Spark.
//!
//! The first version had 8 fixed kinds of things and simple "touch X, do Y" rules.
//! We read that old format and turn it into the new one, so nobody loses their games.

use crate::data::*;
use crate::sound::ids as snd;
use serde::Deserialize;

#[derive(Deserialize, Clone, Copy, PartialEq)]
enum LKind {
    Player,
    Wall,
    Coin,
    Enemy,
    Goal,
    Spikes,
    Spring,
    Sign,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Default)]
enum LMotion {
    #[default]
    Still,
    LeftRight,
    UpDown,
    Follow,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Default)]
enum LMode {
    #[default]
    Platformer,
    TopDown,
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
enum LAction {
    AddScore(i32),
    LoseLife,
    FinishLevel,
    Disappear,
    GoToStart,
    Launch,
}

#[derive(Deserialize, Clone, PartialEq)]
struct LRule {
    touches: LKind,
    actions: Vec<LAction>,
}

#[derive(Deserialize)]
struct LThing {
    kind: LKind,
    x: i32,
    y: i32,
    #[serde(default)]
    color: Option<Rgb>,
    #[serde(default)]
    motion: LMotion,
    #[serde(default)]
    speed: Option<f32>,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct LLevel {
    name: String,
    width: i32,
    height: i32,
    sky: Rgb,
    things: Vec<LThing>,
}

#[derive(Deserialize)]
struct LGame {
    title: String,
    #[serde(default)]
    mode: LMode,
    #[serde(default)]
    lives: Option<i32>,
    #[serde(default)]
    speed: Option<f32>,
    #[serde(default)]
    jump_height: Option<f32>,
    #[serde(default = "yes")]
    stomp: bool,
    #[serde(default)]
    win_message: Option<String>,
    #[serde(default)]
    rules: Option<Vec<LRule>>,
    levels: Vec<LLevel>,
}

fn yes() -> bool {
    true
}

fn type_id(k: LKind) -> TypeId {
    match k {
        LKind::Player => ids::PLAYER,
        LKind::Wall => ids::WALL,
        LKind::Coin => ids::COIN,
        LKind::Enemy => ids::ENEMY,
        LKind::Goal => ids::GOAL,
        LKind::Spikes => ids::SPIKES,
        LKind::Spring => ids::SPRING,
        LKind::Sign => ids::SIGN,
    }
}

fn old_default_rules() -> Vec<LRule> {
    vec![
        LRule { touches: LKind::Coin, actions: vec![LAction::AddScore(10), LAction::Disappear] },
        LRule { touches: LKind::Enemy, actions: vec![LAction::LoseLife] },
        LRule { touches: LKind::Spikes, actions: vec![LAction::LoseLife] },
        LRule { touches: LKind::Goal, actions: vec![LAction::FinishLevel] },
        LRule { touches: LKind::Spring, actions: vec![LAction::Launch] },
    ]
}

pub fn convert(value: serde_json::Value) -> Result<Game, String> {
    let old: LGame = serde_json::from_value(value).map_err(|e| format!("That file is not a Spark game ({e})"))?;
    let mut g = Game::new(&old.title);
    g.lives = old.lives.unwrap_or(3).max(1);
    if let Some(m) = old.win_message {
        g.win_message = m;
    }
    if let Some(p) = g.ty_mut(ids::PLAYER) {
        p.speed = old.speed.unwrap_or(6.0);
        p.jump_height = old.jump_height.unwrap_or(3.2);
        if old.mode == LMode::TopDown {
            p.behavior = Behavior::TopDown;
            p.hit_bottom = false;
            p.hit_h = 0.8;
        }
    }

    // Rules.
    let old_rules = old.rules.unwrap_or_else(old_default_rules);
    if old_rules == old_default_rules() {
        // Keep the nicer new default rules (with sounds). Squashing enemies depends on the old "stomp" setting.
        if !old.stomp {
            g.rules.retain(|r| !r.conditions.contains(&Condition::FromAbove));
            for r in &mut g.rules {
                r.conditions.retain(|c| *c != Condition::NotFromAbove);
            }
        }
        if old.mode == LMode::TopDown {
            g.rules.retain(|r| !r.conditions.contains(&Condition::FromAbove));
            for r in &mut g.rules {
                r.conditions.retain(|c| *c != Condition::NotFromAbove);
            }
        }
    } else {
        g.rules.clear();
        for r in &old_rules {
            let b = type_id(r.touches);
            let actions: Vec<Action> = r
                .actions
                .iter()
                .map(|a| match a {
                    LAction::AddScore(n) => Action::AddScore(*n),
                    LAction::LoseLife => Action::LoseLife,
                    LAction::FinishLevel => Action::NextLevel,
                    LAction::Disappear => Action::Destroy(Target::Other),
                    LAction::GoToStart => Action::GoToStart,
                    LAction::Launch => Action::Launch(7.7),
                })
                .collect();
            if r.touches == LKind::Enemy && old.stomp && old.mode == LMode::Platformer {
                g.rules.push(
                    Rule::new(
                        Trigger::Touch { a: ids::PLAYER, b },
                        &[Action::Destroy(Target::Other), Action::AddScore(20), Action::Launch(1.6), Action::PlaySound(snd::BLIP)],
                    )
                    .only_if(Condition::FromAbove),
                );
                g.rules.push(Rule { trigger: Trigger::Touch { a: ids::PLAYER, b }, conditions: vec![Condition::NotFromAbove], actions });
            } else {
                g.rules.push(Rule::new(Trigger::Touch { a: ids::PLAYER, b }, &actions));
            }
        }
        g.rules.push(Rule::new(Trigger::Jumps(ids::PLAYER), &[Action::PlaySound(snd::JUMP)]));
        g.rules.push(Rule::new(Trigger::LeavesLevel(ids::PLAYER), &[Action::LoseLife, Action::PlaySound(snd::HURT)]));
    }

    // Levels.
    for l in old.levels {
        let mut level = Level::new(&l.name);
        level.width = l.width.clamp(MIN_LEVEL_W, MAX_LEVEL_W);
        level.height = l.height.clamp(MIN_LEVEL_H, MAX_LEVEL_H);
        level.sky = l.sky;
        for t in l.things {
            let mut nt = g.new_thing(type_id(t.kind), t.x, t.y);
            nt.color = t.color;
            if !t.text.is_empty() {
                nt.text = t.text;
            }
            if t.kind == LKind::Enemy {
                let b = match t.motion {
                    LMotion::Still => Behavior::Still,
                    LMotion::LeftRight => Behavior::PatrolH,
                    LMotion::UpDown => Behavior::PatrolV,
                    LMotion::Follow => Behavior::Follow,
                };
                if b != Behavior::PatrolH {
                    nt.behavior = Some(b);
                }
                if let Some(s) = t.speed {
                    if (s - 2.0).abs() > 1e-3 {
                        nt.speed = Some(s);
                    }
                }
            }
            if level.in_bounds(nt.x, nt.y) {
                level.things.retain(|o| !(o.x == nt.x && o.y == nt.y));
                level.things.push(nt);
            }
        }
        g.levels.push(level);
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD_GAME: &str = r#"{
      "title": "Old Game", "mode": "Platformer", "lives": 4, "speed": 7.0, "jump_height": 3.5, "stomp": true,
      "win_message": "Yay",
      "rules": [
        {"touches":"Coin","actions":[{"AddScore":10},"Disappear"]},
        {"touches":"Enemy","actions":["LoseLife"]},
        {"touches":"Spikes","actions":["LoseLife"]},
        {"touches":"Goal","actions":["FinishLevel"]},
        {"touches":"Spring","actions":["Launch"]}
      ],
      "levels": [{"name":"One","width":10,"height":8,"sky":[1,2,3],"things":[
        {"kind":"Player","x":1,"y":6,"motion":"Still","speed":2.0,"text":""},
        {"kind":"Enemy","x":4,"y":6,"motion":"UpDown","speed":3.0,"text":""},
        {"kind":"Sign","x":3,"y":6,"motion":"Still","speed":2.0,"text":"Hi"}
      ]}]
    }"#;

    #[test]
    fn old_games_open_in_the_new_format() {
        let g = Game::from_json(OLD_GAME).unwrap();
        assert_eq!(g.title, "Old Game");
        assert_eq!(g.lives, 4);
        assert_eq!(g.ty(ids::PLAYER).unwrap().speed, 7.0);
        assert_eq!(g.levels[0].things.len(), 3);
        let enemy = g.levels[0].things.iter().find(|t| t.ty == ids::ENEMY).unwrap();
        assert_eq!(enemy.behavior, Some(Behavior::PatrolV));
        assert_eq!(enemy.speed, Some(3.0));
        let sign = g.levels[0].things.iter().find(|t| t.ty == ids::SIGN).unwrap();
        assert_eq!(sign.text, "Hi");
        // Default old rules become the new default rules (with sounds and squashing).
        assert_eq!(g.rules, default_rules());
    }

    #[test]
    fn custom_old_rules_are_translated() {
        let text = OLD_GAME.replace("[{\"AddScore\":10},\"Disappear\"]", "[{\"AddScore\":99},\"Disappear\"]");
        let g = Game::from_json(&text).unwrap();
        let coin = g
            .rules
            .iter()
            .find(|r| r.trigger == (Trigger::Touch { a: ids::PLAYER, b: ids::COIN }))
            .unwrap();
        assert!(coin.actions.contains(&Action::AddScore(99)));
    }
}
