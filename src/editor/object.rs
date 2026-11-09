//! The Thing, Object and Game tabs, and the "pick a picture" window.

use super::*;
use egui::{ComboBox, DragValue, Slider};

/// Behaviors it makes sense to change on a single placed thing.
const PER_THING: [Behavior; 5] = [Behavior::Still, Behavior::PatrolH, Behavior::PatrolV, Behavior::Wander, Behavior::Follow];

fn hint_label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).color(Color32::from_rgb(110, 100, 150)));
}

impl Editor {
    // -----------------------------------------------------------------------
    // Thing: the one you selected in the level
    // -----------------------------------------------------------------------

    pub(super) fn thing_tab(&mut self, ui: &mut Ui) {
        let Some(idx) = self.selected_index() else {
            ui.label("Nothing is selected.");
            hint_label(ui, "Choose Select / Move on the left, then click a thing in your level to change its color, speed or message.");
            return;
        };
        let lvl = self.level;
        let original = self.game.levels[lvl].things[idx].clone();
        let Some(ty) = self.game.ty(original.ty).cloned() else {
            self.selected = None;
            return;
        };
        let mut t = original.clone();
        let mut delete = false;
        let mut edit_type = false;

        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(52.0, 52.0), Sense::hover());
            render::draw_type(ui.painter(), &ty, rect, render::rgb(t.color.unwrap_or(ty.color)), Anim::default());
            ui.vertical(|ui| {
                ui.label(RichText::new(&ty.name).heading());
                ui.label(RichText::new(format!("at column {}, row {}", t.x + 1, t.y + 1)).small());
            });
        });
        hint_label(ui, ty.behavior.label());
        ui.add_space(4.0);

        if matches!(ty.art, Art::Built(_)) {
            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut c = t.color.unwrap_or(ty.color);
                if ui.color_edit_button_srgb(&mut c).changed() {
                    t.color = Some(c);
                }
                if t.color.is_some() && ui.small_button("Reset").clicked() {
                    t.color = None;
                }
            });
        }

        let mut behavior = t.behavior.unwrap_or(ty.behavior);
        if matches!(ty.behavior, Behavior::Still | Behavior::PatrolH | Behavior::PatrolV | Behavior::Wander | Behavior::Follow) {
            ui.add_space(4.0);
            ui.label("How does this one move?");
            let same = t.behavior.is_none();
            let current = if same { format!("Same as {}: {}", ty.name, ty.behavior.label()) } else { behavior.label().to_string() };
            ComboBox::from_id_salt("thing_behavior").selected_text(current).width(250.0).show_ui(ui, |ui| {
                if ui.selectable_label(same, format!("Same as {}: {}", ty.name, ty.behavior.label())).clicked() {
                    t.behavior = None;
                }
                for b in PER_THING {
                    if ui.selectable_label(t.behavior == Some(b), b.label()).clicked() {
                        t.behavior = Some(b);
                    }
                }
            });
            behavior = t.behavior.unwrap_or(ty.behavior);
        }
        if behavior.uses_speed() && behavior != Behavior::Still && !behavior.controlled() {
            let mut sp = t.speed.unwrap_or(ty.speed);
            if ui.add(Slider::new(&mut sp, 0.5..=12.0).text("Speed")).changed() {
                t.speed = Some(sp);
            }
        }
        if ty.talks {
            ui.add_space(4.0);
            ui.label("What does it say?");
            ui.add(TextEdit::multiline(&mut t.text).desired_rows(4).desired_width(f32::INFINITY));
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Delete this thing").clicked() {
                delete = true;
            }
            if ui.button(format!("Change all {}s...", short(&ty.name, 10))).on_hover_text("Edit the object itself: its look, behavior and size").clicked() {
                edit_type = true;
            }
        });

        if delete {
            self.snapshot();
            let (x, y) = (t.x, t.y);
            self.game.erase(lvl, x, y);
            self.selected = None;
        } else if t != original {
            let now = ui.input(|i| i.time);
            self.snapshot_soft(now);
            self.game.levels[lvl].things[idx] = t;
        }
        if edit_type {
            self.tool = Tool::Place(original.ty);
            self.tab = Tab::Object;
        }
    }

    // -----------------------------------------------------------------------
    // Object: a kind of thing (its look and behavior)
    // -----------------------------------------------------------------------

    pub(super) fn object_tab(&mut self, ui: &mut Ui) {
        let Some(id) = self.focus_type() else {
            ui.label("No object picked.");
            hint_label(ui, "Click an object on the left, or add a new one with '+ New object'.");
            return;
        };
        let Some(original) = self.game.ty(id).cloned() else { return };
        let mut t = original.clone();
        let mut delete = false;
        let mut open_looks = false;
        let mut open_draw = false;
        let mut open_import = false;

        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
            ui.painter().rect_filled(rect, 8.0, Color32::from_rgb(150, 210, 255));
            let r = Rect::from_center_size(rect.center(), vec2(52.0, 52.0));
            render::draw_type(ui.painter(), &t, r, render::rgb(t.color), Anim::default());
            ui.vertical(|ui| {
                ui.add(TextEdit::singleline(&mut t.name).desired_width(190.0).font(FontId::proportional(20.0)));
                ui.label(RichText::new(t.behavior.label()).small().color(Color32::from_rgb(110, 100, 150)));
            });
        });

        // ---- look ----
        ui.add_space(6.0);
        ui.label(RichText::new("Look").heading());
        ui.horizontal_wrapped(|ui| {
            if ui.button("Pick a picture").clicked() {
                open_looks = true;
            }
            if ui.button(if matches!(t.art, Art::Pixels(_)) { "Edit my drawing" } else { "Draw my own" }).clicked() {
                open_draw = true;
            }
            if ui.button("Import PNG...").on_hover_text("Use a PNG picture from your computer (you can also drag one onto the window)").clicked() {
                open_import = true;
            }
        });
        if matches!(t.art, Art::Built(_)) {
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_srgb(&mut t.color);
            });
        }

        // ---- behavior ----
        ui.add_space(6.0);
        ui.label(RichText::new("Behavior").heading());
        ComboBox::from_id_salt("behavior").selected_text(t.behavior.label()).width(270.0).show_ui(ui, |ui| {
            for b in Behavior::ALL {
                if ui.selectable_label(t.behavior == b, b.label()).clicked() {
                    t.behavior = b;
                }
            }
        });
        hint_label(ui, behavior_help(t.behavior));
        if t.behavior != original.behavior && t.behavior.controlled() {
            t.unique = true;
        }
        if t.behavior.uses_speed() {
            ui.add(Slider::new(&mut t.speed, 0.5..=16.0).text("Speed"));
        }
        if t.behavior == Behavior::Platformer {
            ui.add(Slider::new(&mut t.jump_height, 1.0..=7.0).text("Jump height"));
        }
        if t.behavior == Behavior::Drift {
            ui.horizontal(|ui| {
                ui.label("Flies:");
                ComboBox::from_id_salt("drift").selected_text(t.drift.label()).show_ui(ui, |ui| {
                    for d in Dir::FLIGHT {
                        ui.selectable_value(&mut t.drift, d, d.label());
                    }
                });
            });
        }

        // ---- size ----
        ui.add_space(6.0);
        ui.label(RichText::new("Size and touching").heading());
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(DragValue::new(&mut t.size_w).range(0.25..=10.0).speed(0.05).suffix(" wide"));
            ui.add(DragValue::new(&mut t.size_h).range(0.25..=10.0).speed(0.05).suffix(" tall"));
        });
        ui.add(Slider::new(&mut t.hit_w, 0.2..=1.0).text("Touch width"));
        ui.add(Slider::new(&mut t.hit_h, 0.2..=1.0).text("Touch height"));
        ui.checkbox(&mut t.hit_bottom, "The touching part sits at the bottom");
        hint_label(ui, "Touching a bit less than the full picture feels fairer when you play.");

        // ---- extras ----
        ui.add_space(6.0);
        ui.label(RichText::new("Extras").heading());
        ui.checkbox(&mut t.talks, "Says its message when you are near (like a sign)");
        ui.checkbox(&mut t.show_count, "Show how many are left in the score bar");
        ui.checkbox(&mut t.unique, "Only one of these per level");
        ui.horizontal(|ui| {
            ui.label("Drawn:");
            let names = ["behind everything", "normally", "in front"];
            ComboBox::from_id_salt("layer").selected_text(names[t.layer.min(2) as usize]).show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut t.layer, i as u8, *n);
                }
            });
        });

        ui.add_space(10.0);
        if ui.button("Delete this object").on_hover_text("Also removes it from the levels and from any rules that use it. You can Undo.").clicked() {
            delete = true;
        }

        if delete {
            self.snapshot();
            self.game.remove_type(id);
            self.tool = Tool::Select;
            self.selected = None;
            self.sprite_ed = None;
        } else if t != original {
            let now = ui.input(|i| i.time);
            self.snapshot_soft(now);
            if let Some(slot) = self.game.types.iter_mut().find(|o| o.id == id) {
                *slot = t;
            }
        }
        if open_looks {
            self.show_looks = true;
        }
        if open_draw {
            self.open_sprite_editor(id);
        }
        if open_import {
            self.import_dialog(ui.ctx(), id, false);
        }
    }

    /// The "pick a picture" window.
    pub(super) fn looks_window(&mut self, ctx: &egui::Context) {
        if !self.show_looks {
            return;
        }
        let Some(id) = self.focus_type() else {
            self.show_looks = false;
            return;
        };
        let mut keep = true;
        let mut chosen: Option<Pic> = None;
        egui::Window::new(RichText::new("Pick a picture").size(20.0).strong())
            .open(&mut keep)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(380.0);
                ui.label("These come with Spark. Or close this and press 'Draw my own'!");
                ui.horizontal_wrapped(|ui| {
                    for pic in Pic::ALL {
                        let (rect, resp) = ui.allocate_exact_size(vec2(80.0, 86.0), Sense::click());
                        let p = ui.painter();
                        let fill = if resp.hovered() { Color32::from_rgb(238, 232, 255) } else { Color32::WHITE };
                        p.rect_filled(rect, 10.0, fill);
                        p.rect_stroke(rect, 10.0, Stroke::new(1.5, Color32::from_rgb(205, 198, 240)), StrokeKind::Inside);
                        render::draw_sprite(p, pic, Rect::from_min_size(rect.min + vec2(14.0, 8.0), vec2(52.0, 52.0)), render::rgb(pic.default_color()), Anim::default());
                        p.text(pos2(rect.center().x, rect.max.y - 14.0), Align2::CENTER_CENTER, pic.label(), FontId::proportional(13.0), style::INK);
                        if resp.clicked() {
                            chosen = Some(pic);
                        }
                    }
                });
            });
        if let Some(pic) = chosen {
            self.snapshot();
            if let Some(t) = self.game.ty_mut(id) {
                t.art = Art::Built(pic);
                t.color = pic.default_color();
            }
            self.show_looks = false;
        } else if !keep {
            self.show_looks = false;
        }
    }

    // -----------------------------------------------------------------------
    // Game: lives, counters, levels
    // -----------------------------------------------------------------------

    pub(super) fn game_tab(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("The whole game").heading());
        let mut lives = self.game.lives;
        let mut win = self.game.win_message.clone();
        ui.add(Slider::new(&mut lives, 1..=9).text("Lives"));
        ui.label("Message when you win:");
        ui.add(TextEdit::singleline(&mut win).desired_width(f32::INFINITY));
        if lives != self.game.lives || win != self.game.win_message {
            let now = ui.input(|i| i.time);
            self.snapshot_soft(now);
            self.game.lives = lives;
            self.game.win_message = win;
            self.dirty = true;
        }

        // ---- counters ----
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("Counters").heading());
        hint_label(ui, "Count things like keys or ammo. Rules can add to them and check them.");
        let mut vars = self.game.vars.clone();
        let mut remove: Option<usize> = None;
        for (i, v) in vars.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.add(TextEdit::singleline(&mut v.name).desired_width(100.0));
                ui.add(DragValue::new(&mut v.start).prefix("start "));
                ui.checkbox(&mut v.show, "show");
                if ui.small_button("X").on_hover_text("Delete this counter").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            let id = vars[i].id;
            self.snapshot();
            self.game.remove_var(id);
            vars = self.game.vars.clone();
        }
        if ui.button("+ Add counter").clicked() {
            self.snapshot();
            let n = self.game.vars.len() + 1;
            self.game.add_var(&format!("Counter {n}"), 0, true);
            vars = self.game.vars.clone();
        }
        if vars != self.game.vars {
            self.game.vars = vars;
            self.dirty = true;
        }

        // ---- this level ----
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("This level").heading());
        let lvl = self.level;
        let mut name = self.game.levels[lvl].name.clone();
        let mut sky = self.game.levels[lvl].sky;
        let (mut w, mut h) = (self.game.levels[lvl].width, self.game.levels[lvl].height);
        let mut snapshot_needed = false;

        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(TextEdit::singleline(&mut name).desired_width(170.0));
        });
        ui.horizontal(|ui| {
            ui.label("Sky color:");
            ui.color_edit_button_srgb(&mut sky);
        });
        ui.horizontal(|ui| {
            ui.label("Size:");
            let rw = ui.add(DragValue::new(&mut w).range(MIN_LEVEL_W..=MAX_LEVEL_W).suffix(" wide"));
            let rh = ui.add(DragValue::new(&mut h).range(MIN_LEVEL_H..=MAX_LEVEL_H).suffix(" tall"));
            if rw.drag_started() || rw.gained_focus() || rh.drag_started() || rh.gained_focus() {
                snapshot_needed = true;
            }
        });
        if snapshot_needed {
            self.snapshot();
        }
        {
            let l = &mut self.game.levels[lvl];
            if l.name != name || l.sky != sky {
                l.name = name;
                l.sky = sky;
                self.dirty = true;
            }
            if l.width != w || l.height != h {
                l.resize(w, h);
                self.dirty = true;
            }
        }

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Copy level").on_hover_text("Make a copy of this level").clicked() {
                self.snapshot();
                let mut copy = self.game.levels[lvl].clone();
                copy.name = format!("{} copy", copy.name);
                self.game.levels.insert(lvl + 1, copy);
                self.level = lvl + 1;
                self.selected = None;
            }
            let can_delete = self.game.levels.len() > 1;
            if ui.add_enabled(can_delete, Button::new("Delete level")).clicked() {
                self.snapshot();
                self.game.levels.remove(lvl);
                self.level = lvl.min(self.game.levels.len() - 1);
                self.selected = None;
            }
        });
    }
}

fn behavior_help(b: Behavior) -> &'static str {
    match b {
        Behavior::Still => "It stays where it is. Rules decide what happens when things touch it.",
        Behavior::Solid => "Things that move can't go through it. Walls, doors, floors.",
        Behavior::Platformer => "Arrow keys run, Space jumps. Gravity pulls it down.",
        Behavior::TopDown => "Arrow keys walk in all four directions.",
        Behavior::Sideways => "Left and right arrows slide it. Good for ships and paddles.",
        Behavior::Vertical => "Up and down arrows slide it.",
        Behavior::Grid => "Each key press hops one square. It pushes crates!",
        Behavior::PatrolH => "Walks left and right and turns around at walls.",
        Behavior::PatrolV => "Moves up and down and turns around at walls.",
        Behavior::Wander => "Wanders around in random directions.",
        Behavior::Follow => "Floats towards the player, even through walls.",
        Behavior::Fall => "Falls down and lands on solid things.",
        Behavior::Drift => "Flies in a straight line, then disappears when it leaves the level.",
        Behavior::Bounce => "Bounces off walls and the level edges, like a ball.",
        Behavior::Pushable => "A crate: the 'hop one square' player can push it.",
    }
}
