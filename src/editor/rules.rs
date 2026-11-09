//! The Rules tab: "WHEN something happens, IF it is true, DO these things".

use super::*;
use crate::data::Key as GKey;
use crate::sound::SoundId;
use egui::{ComboBox, DragValue};
use std::hash::Hash;

/// The names and ids to choose from in the drop-down menus.
struct Lists {
    types: Vec<(TypeId, String)>,
    sounds: Vec<(SoundId, String)>,
    vars: Vec<(VarId, String)>,
    player: TypeId,
}

impl Lists {
    fn new(g: &Game) -> Lists {
        Lists {
            types: g.types.iter().map(|t| (t.id, t.name.clone())).collect(),
            sounds: g.sounds.iter().map(|s| (s.id, s.name.clone())).collect(),
            vars: g.vars.iter().map(|v| (v.id, v.name.clone())).collect(),
            player: g.player_type().or_else(|| g.types.first().map(|t| t.id)).unwrap_or(0),
        }
    }

    fn first_type(&self) -> TypeId {
        self.types.first().map(|t| t.0).unwrap_or(0)
    }

    /// A type that is not the player (a good default for "the other thing").
    fn other_type(&self) -> TypeId {
        self.types.iter().map(|t| t.0).find(|id| *id != self.player).unwrap_or(self.player)
    }

    fn first_sound(&self) -> SoundId {
        self.sounds.first().map(|t| t.0).unwrap_or(0)
    }

    fn first_var(&self) -> VarId {
        self.vars.first().map(|t| t.0).unwrap_or(0)
    }
}

// ---- the kinds of triggers, conditions and actions, for the menus ----

const TRIGGERS: [&str; 11] = [
    "The level starts",
    "Every few seconds",
    "A key is pressed",
    "Something touches something",
    "Something stops touching something",
    "None of something are left",
    "Some seconds after the level starts",
    "The score reaches a number",
    "A counter reaches a number",
    "Something jumps",
    "Something leaves the level",
];

fn trigger_kind(t: &Trigger) -> usize {
    match t {
        Trigger::Start => 0,
        Trigger::Every { .. } => 1,
        Trigger::Key { .. } => 2,
        Trigger::Touch { .. } => 3,
        Trigger::StopTouch { .. } => 4,
        Trigger::NoneLeft(_) => 5,
        Trigger::AfterSeconds(_) => 6,
        Trigger::Score(_) => 7,
        Trigger::Counter { .. } => 8,
        Trigger::Jumps(_) => 9,
        Trigger::LeavesLevel(_) => 10,
    }
}

fn make_trigger(i: usize, l: &Lists) -> Trigger {
    match i {
        0 => Trigger::Start,
        1 => Trigger::Every { secs: 2.0, who: None },
        2 => Trigger::Key { key: GKey::Space, mode: Press::Pressed, who: Some(l.player) },
        3 => Trigger::Touch { a: l.player, b: l.other_type() },
        4 => Trigger::StopTouch { a: l.player, b: l.other_type() },
        5 => Trigger::NoneLeft(l.other_type()),
        6 => Trigger::AfterSeconds(30.0),
        7 => Trigger::Score(100),
        8 => Trigger::Counter { var: l.first_var(), n: 3 },
        9 => Trigger::Jumps(l.player),
        _ => Trigger::LeavesLevel(l.other_type()),
    }
}

const CONDITIONS: [&str; 7] = [
    "It lands on top (stomp)",
    "It does NOT land on top",
    "None of something are left",
    "Some of something are left",
    "The score is at least...",
    "A counter is at least...",
    "Only some of the time",
];

fn condition_kind(c: &Condition) -> usize {
    match c {
        Condition::FromAbove => 0,
        Condition::NotFromAbove => 1,
        Condition::NoneLeft(_) => 2,
        Condition::AnyLeft(_) => 3,
        Condition::ScoreAtLeast(_) => 4,
        Condition::CounterAtLeast { .. } => 5,
        Condition::Chance(_) => 6,
    }
}

fn make_condition(i: usize, l: &Lists) -> Condition {
    match i {
        0 => Condition::FromAbove,
        1 => Condition::NotFromAbove,
        2 => Condition::NoneLeft(l.other_type()),
        3 => Condition::AnyLeft(l.other_type()),
        4 => Condition::ScoreAtLeast(50),
        5 => Condition::CounterAtLeast { var: l.first_var(), n: 1 },
        _ => Condition::Chance(25),
    }
}

fn make_action(i: usize, l: &Lists) -> Action {
    match i {
        0 => Action::AddScore(10),
        1 => Action::AddLives(1),
        2 => Action::AddCounter { var: l.first_var(), n: 1 },
        3 => Action::SetCounter { var: l.first_var(), n: 0 },
        4 => Action::LoseLife,
        5 => Action::NextLevel,
        6 => Action::RestartLevel,
        7 => Action::GameOver,
        8 => Action::WinGame,
        9 => Action::Destroy(Target::Other),
        10 => Action::Spawn { ty: l.first_type(), at: SpawnAt::Me, dir: Dir::Default },
        11 => Action::GoToStart,
        12 => Action::Launch(3.0),
        13 => Action::Bounce,
        14 => Action::PlaySound(l.first_sound()),
        _ => Action::Say("Hello!".into()),
    }
}

const ACTION_COUNT: usize = 16;

fn action_kind(a: &Action) -> usize {
    (0..ACTION_COUNT).find(|i| make_action(*i, &Lists { types: vec![], sounds: vec![], vars: vec![], player: 0 }).same_type(a)).unwrap_or(0)
}

// ---- small widgets ----

fn pick<T: Copy + PartialEq>(ui: &mut Ui, salt: impl Hash + std::fmt::Debug, value: &mut T, options: &[(T, String)], width: f32) {
    let text = options.iter().find(|(v, _)| v == value).map(|(_, n)| n.clone()).unwrap_or_else(|| "(pick one)".to_string());
    ComboBox::from_id_salt(salt).selected_text(text).width(width).show_ui(ui, |ui| {
        for (v, n) in options {
            ui.selectable_value(value, *v, n);
        }
    });
}

fn pick_opt(ui: &mut Ui, salt: impl Hash + std::fmt::Debug, value: &mut Option<TypeId>, options: &[(TypeId, String)], none: &str) {
    let text = match value {
        None => none.to_string(),
        Some(v) => options.iter().find(|(id, _)| id == v).map(|(_, n)| n.clone()).unwrap_or_else(|| "(pick one)".into()),
    };
    ComboBox::from_id_salt(salt).selected_text(text).width(170.0).show_ui(ui, |ui| {
        ui.selectable_value(value, None, none);
        for (id, n) in options {
            ui.selectable_value(value, Some(*id), n);
        }
    });
}

fn kind_combo(ui: &mut Ui, salt: impl Hash + std::fmt::Debug, current: usize, names: &[&str]) -> Option<usize> {
    let mut chosen = None;
    ComboBox::from_id_salt(salt).selected_text(names[current]).width(200.0).show_ui(ui, |ui| {
        for (i, n) in names.iter().enumerate() {
            if ui.selectable_label(i == current, *n).clicked() && i != current {
                chosen = Some(i);
            }
        }
    });
    chosen
}

fn trigger_ui(ui: &mut Ui, ri: usize, t: &mut Trigger, l: &Lists) {
    let keys: Vec<(GKey, String)> = GKey::ALL.iter().map(|k| (*k, k.label().to_string())).collect();
    let touches = matches!(t, Trigger::Touch { .. });
    match t {
        Trigger::Start => {}
        Trigger::Every { secs, who } => {
            ui.horizontal(|ui| {
                ui.label("every");
                ui.add(DragValue::new(secs).range(0.1..=120.0).speed(0.05).suffix(" seconds"));
            });
            ui.horizontal(|ui| {
                ui.label("for");
                pick_opt(ui, (ri, "every_who"), who, &l.types, "the whole game");
            });
        }
        Trigger::Key { key, mode, who } => {
            ui.horizontal(|ui| {
                pick(ui, (ri, "key"), key, &keys, 120.0);
                ui.selectable_value(mode, Press::Pressed, "pressed");
                ui.selectable_value(mode, Press::Held, "held");
            });
            ui.horizontal(|ui| {
                ui.label("for");
                pick_opt(ui, (ri, "key_who"), who, &l.types, "the whole game");
            });
        }
        Trigger::Touch { a, .. } | Trigger::StopTouch { a, .. } => {
            ui.horizontal(|ui| {
                pick(ui, (ri, "ta"), a, &l.types, 110.0);
                ui.label(if touches { "touches" } else { "stops touching" });
            });
        }
        Trigger::NoneLeft(x) | Trigger::Jumps(x) | Trigger::LeavesLevel(x) => {
            pick(ui, (ri, "tx"), x, &l.types, 170.0);
        }
        Trigger::AfterSeconds(s) => {
            ui.add(DragValue::new(s).range(1.0..=3600.0).speed(0.5).suffix(" seconds"));
        }
        Trigger::Score(n) => {
            ui.add(DragValue::new(n).range(1..=100000).speed(1.0).suffix(" points"));
        }
        Trigger::Counter { var, n } => {
            ui.horizontal(|ui| {
                pick(ui, (ri, "tv"), var, &l.vars, 120.0);
                ui.label("reaches");
                ui.add(DragValue::new(n).range(-1000..=1000));
            });
        }
    }
    if let Trigger::Touch { b, .. } | Trigger::StopTouch { b, .. } = t {
        pick(ui, (ri, "tb"), b, &l.types, 170.0);
    }
}

fn condition_ui(ui: &mut Ui, salt: (usize, usize), c: &mut Condition, l: &Lists) {
    match c {
        Condition::FromAbove | Condition::NotFromAbove => {}
        Condition::NoneLeft(t) | Condition::AnyLeft(t) => pick(ui, (salt, "ct"), t, &l.types, 130.0),
        Condition::ScoreAtLeast(n) => {
            ui.add(DragValue::new(n).range(0..=100000));
        }
        Condition::CounterAtLeast { var, n } => {
            pick(ui, (salt, "cv"), var, &l.vars, 100.0);
            ui.add(DragValue::new(n).range(-1000..=1000));
        }
        Condition::Chance(p) => {
            ui.add(DragValue::new(p).range(1..=100).suffix(" %"));
        }
    }
}

/// `preview` is set when the user presses the little "Play" button next to a sound.
fn action_ui(ui: &mut Ui, salt: (usize, usize), a: &mut Action, l: &Lists, preview: &mut Option<SoundId>) {
    match a {
        Action::AddScore(n) => {
            ui.add(DragValue::new(n).range(-1000..=1000).speed(1.0).prefix("by "));
        }
        Action::AddLives(n) => {
            ui.add(DragValue::new(n).range(-9..=9).prefix("by "));
        }
        Action::AddCounter { var, n } => {
            pick(ui, (salt, "av"), var, &l.vars, 100.0);
            ui.add(DragValue::new(n).range(-1000..=1000).prefix("by "));
        }
        Action::SetCounter { var, n } => {
            pick(ui, (salt, "sv"), var, &l.vars, 100.0);
            ui.add(DragValue::new(n).range(-1000..=1000).prefix("to "));
        }
        Action::Destroy(target) => {
            let kind = match target {
                Target::Me => 0,
                Target::Other => 1,
                Target::All(_) => 2,
            };
            let names = ["me", "the other thing", "every one of a type"];
            if let Some(k) = kind_combo(ui, (salt, "dk"), kind, &names) {
                *target = match k {
                    0 => Target::Me,
                    1 => Target::Other,
                    _ => Target::All(l.other_type()),
                };
            }
            if let Target::All(t) = target {
                pick(ui, (salt, "dt"), t, &l.types, 130.0);
            }
        }
        Action::Spawn { ty, at, dir } => {
            pick(ui, (salt, "st"), ty, &l.types, 150.0);
            ui.end_row();
            let ats: Vec<(SpawnAt, String)> = SpawnAt::ALL.iter().map(|x| (*x, x.label().to_string())).collect();
            ui.label("appears");
            pick(ui, (salt, "sa"), at, &ats, 130.0);
            ui.end_row();
            let dirs: Vec<(Dir, String)> = Dir::ALL.iter().map(|x| (*x, x.label().to_string())).collect();
            ui.label("goes");
            pick(ui, (salt, "sd"), dir, &dirs, 150.0);
        }
        Action::Launch(h) => {
            ui.add(DragValue::new(h).range(0.5..=14.0).speed(0.1).suffix(" squares"));
        }
        Action::PlaySound(id) => {
            pick(ui, (salt, "ps"), id, &l.sounds, 120.0);
            if ui.small_button("Play").on_hover_text("Hear it").clicked() {
                *preview = Some(*id);
            }
        }
        Action::Say(text) => {
            ui.add(TextEdit::singleline(text).desired_width(150.0));
        }
        _ => {}
    }
}

fn who_text(t: &Trigger, g: &Game) -> Option<String> {
    match t {
        Trigger::Touch { a, b } | Trigger::StopTouch { a, b } => Some(format!("'me' is the {}, 'the other thing' is the {}.", g.type_name(*a), g.type_name(*b))),
        Trigger::Key { who: Some(w), .. } | Trigger::Every { who: Some(w), .. } => Some(format!("'me' is each {}.", g.type_name(*w))),
        Trigger::Jumps(w) | Trigger::LeavesLevel(w) => Some(format!("'me' is the {}.", g.type_name(*w))),
        _ => None,
    }
}

impl Editor {
    pub(super) fn rules_tab(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Rules").heading());
        ui.label(RichText::new("WHEN something happens, (IF something is true), DO things. Open a rule to change it, or add your own!").color(Color32::from_rgb(110, 100, 150)));
        ui.add_space(4.0);

        let lists = Lists::new(&self.game);
        let mut rules = self.game.rules.clone();
        let mut remove: Option<usize> = None;
        let mut swap: Option<(usize, usize)> = None;
        let mut preview: Option<SoundId> = None;
        let count = rules.len();

        for (ri, rule) in rules.iter_mut().enumerate() {
            let actions: Vec<String> = rule.actions.iter().map(|a| a.describe(&self.game)).collect();
            let mut when = format!("WHEN {}", rule.trigger.describe(&self.game));
            if !rule.conditions.is_empty() {
                when += &format!(" IF {}", rule.conditions.iter().map(|c| c.describe(&self.game)).collect::<Vec<_>>().join(" and "));
            }
            // Two short lines: a header that is too wide would push the whole panel off the screen.
            let header = RichText::new(format!("{}
DO {}", short(&when, 38), short(&actions.join(", "), 36))).size(14.5);
            egui::CollapsingHeader::new(header).id_salt(("rule", ri)).default_open(self.open_rules).show(ui, |ui| {
                Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.5, Color32::from_rgb(205, 198, 240))).corner_radius(10.0).inner_margin(Margin::same(8)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(format!("{} DO {}", when, actions.join(", "))).small().color(Color32::from_rgb(120, 110, 160)));
                    // WHEN
                    ui.label(RichText::new("WHEN").strong().color(style::PURPLE));
                    if let Some(k) = kind_combo(ui, (ri, "tk"), trigger_kind(&rule.trigger), &TRIGGERS) {
                        rule.trigger = make_trigger(k, &lists);
                    }
                    trigger_ui(ui, ri, &mut rule.trigger, &lists);
                    if let Some(w) = who_text(&rule.trigger, &self.game) {
                        ui.label(RichText::new(w).small().color(Color32::from_rgb(120, 110, 160)));
                    }

                    // IF
                    ui.add_space(4.0);
                    ui.label(RichText::new("IF (all of these)").strong().color(style::PURPLE));
                    let mut remove_c: Option<usize> = None;
                    for (ci, c) in rule.conditions.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if let Some(k) = kind_combo(ui, (ri, ci, "ck"), condition_kind(c), &CONDITIONS) {
                                *c = make_condition(k, &lists);
                            }
                            if ui.small_button("x").on_hover_text("Remove this check").clicked() {
                                remove_c = Some(ci);
                            }
                        });
                        ui.horizontal_wrapped(|ui| condition_ui(ui, (ri, ci), c, &lists));
                    }
                    if let Some(ci) = remove_c {
                        rule.conditions.remove(ci);
                    }
                    ui.menu_button("+ Add a check", |ui| {
                        for (i, n) in CONDITIONS.iter().enumerate() {
                            if ui.button(*n).clicked() {
                                rule.conditions.push(make_condition(i, &lists));
                                ui.close();
                            }
                        }
                    });

                    // DO
                    ui.add_space(4.0);
                    ui.label(RichText::new("DO (in order)").strong().color(style::PURPLE));
                    let names: Vec<&'static str> = (0..ACTION_COUNT).map(|i| make_action(i, &lists).label()).collect();
                    let mut remove_a: Option<usize> = None;
                    for (ai, a) in rule.actions.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            if let Some(k) = kind_combo(ui, (ri, ai, "ak"), action_kind(a), &names) {
                                *a = make_action(k, &lists);
                            }
                            if ui.small_button("x").on_hover_text("Remove this step").clicked() {
                                remove_a = Some(ai);
                            }
                        });
                        ui.horizontal_wrapped(|ui| action_ui(ui, (ri, ai), a, &lists, &mut preview));
                    }
                    if let Some(ai) = remove_a {
                        rule.actions.remove(ai);
                    }
                    ui.menu_button("+ Add an action", |ui| {
                        for (i, n) in names.iter().enumerate() {
                            if ui.button(*n).clicked() {
                                rule.actions.push(make_action(i, &lists));
                                ui.close();
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(ri > 0, Button::new("Up")).on_hover_text("Run this rule earlier").clicked() {
                            swap = Some((ri, ri - 1));
                        }
                        if ui.add_enabled(ri + 1 < count, Button::new("Down")).on_hover_text("Run this rule later").clicked() {
                            swap = Some((ri, ri + 1));
                        }
                        if ui.button("Delete rule").clicked() {
                            remove = Some(ri);
                        }
                    });
                });
            });
        }
        if let Some(ri) = remove {
            rules.remove(ri);
        }
        if let Some((a, b)) = swap {
            rules.swap(a, b);
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("+ New rule").clicked() {
                let l = Lists::new(&self.game);
                rules.push(Rule::new(Trigger::Touch { a: l.player, b: l.other_type() }, &[Action::AddScore(10)]));
            }
            let has_defaults = [ids::PLAYER, ids::COIN, ids::ENEMY, ids::GOAL, ids::SPIKES, ids::SPRING].iter().all(|id| self.game.ty(*id).is_some());
            if has_defaults && ui.button("Reset rules").on_hover_text("Go back to the normal platformer rules").clicked() {
                rules = default_rules();
            }
        });

        if rules != self.game.rules {
            let now = ui.input(|i| i.time);
            self.snapshot_soft(now);
            self.game.rules = rules;
            self.dirty = true;
        }
        if let Some(id) = preview {
            if let Some(s) = self.game.sound(id).cloned() {
                self.preview_sound(&s);
            }
        }
    }
}
