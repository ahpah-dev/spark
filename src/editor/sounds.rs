//! The Sounds tab: make beeps and boings, and pick the music.

use super::*;
use crate::sound::{Music, Preset, Wave, surprise};
use egui::{ComboBox, Slider};

impl Editor {
    pub(super) fn sounds_tab(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Sound effects").heading());
        ui.label(RichText::new("Make sounds, then use them in rules (the 'Play a sound' action).").color(Color32::from_rgb(110, 100, 150)));
        ui.add_space(4.0);

        // ---- the list ----
        let mut play: Option<Sound> = None;
        let mut select: Option<u32> = None;
        let sel = self.sound_sel.filter(|id| self.game.sound(*id).is_some()).or_else(|| self.game.sounds.first().map(|s| s.id));
        for s in &self.game.sounds {
            ui.horizontal(|ui| {
                if ui.add_sized([46.0, 26.0], Button::new("Play")).clicked() {
                    play = Some(s.clone());
                }
                if ui.selectable_label(Some(s.id) == sel, RichText::new(&s.name).size(16.0)).clicked() {
                    select = Some(s.id);
                }
            });
        }
        if let Some(id) = select {
            self.sound_sel = Some(id);
        }
        let sel = select.or(sel);

        ui.add_space(4.0);
        let mut new_from: Option<Preset> = None;
        ui.menu_button("+ New sound", |ui| {
            for p in Preset::ALL {
                if ui.button(p.name()).clicked() {
                    new_from = Some(p);
                    ui.close();
                }
            }
        });
        if let Some(p) = new_from {
            self.snapshot();
            let s = p.sound(0);
            let id = self.game.add_sound(s.clone());
            self.sound_sel = Some(id);
            play = self.game.sound(id).cloned();
        }

        // ---- editing the selected one ----
        if let Some(id) = sel {
            if let Some(original) = self.game.sound(id).cloned() {
                ui.add_space(8.0);
                ui.separator();
                let mut s = original.clone();
                let mut delete = false;
                let mut dragged_off = false;
                ui.label(RichText::new("Change this sound").heading());
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(TextEdit::singleline(&mut s.name).desired_width(150.0));
                });
                ui.label("Start from a ready-made sound:");
                ui.horizontal_wrapped(|ui| {
                    for p in Preset::ALL {
                        if ui.small_button(p.name()).clicked() {
                            let name = s.name.clone();
                            s = p.sound(s.id);
                            s.name = name;
                            play = Some(s.clone());
                        }
                    }
                });
                let before_wave = s.wave;
                ComboBox::from_id_salt("wave").selected_text(s.wave.label()).width(200.0).show_ui(ui, |ui| {
                    for w in Wave::ALL {
                        ui.selectable_value(&mut s.wave, w, w.label());
                    }
                });
                if s.wave != before_wave {
                    play = Some(s.clone());
                }
                for r in [
                    ui.add(Slider::new(&mut s.pitch, 60.0..=2400.0).logarithmic(true).text("Pitch")),
                    ui.add(Slider::new(&mut s.slide, -5.0..=5.0).text("Slide")),
                    ui.add(Slider::new(&mut s.length, 0.06..=1.5).text("Length")),
                    ui.add(Slider::new(&mut s.volume, 0.1..=1.0).text("Loudness")),
                    ui.add(Slider::new(&mut s.vibrato, 0.0..=1.0).text("Wobble")),
                    ui.add(Slider::new(&mut s.extra_note, -12.0..=12.0).step_by(1.0).text("Extra note")),
                ] {
                    if r.drag_stopped() || r.lost_focus() {
                        dragged_off = true;
                    }
                }
                if dragged_off {
                    play = Some(s.clone());
                }
                ui.horizontal(|ui| {
                    if ui.add(Button::new(RichText::new("Hear it").strong()).fill(style::GREEN).min_size(vec2(90.0, 32.0))).clicked() {
                        play = Some(s.clone());
                    }
                    if ui.add(Button::new("Surprise me!").min_size(vec2(110.0, 32.0))).on_hover_text("Change the sound a little bit, at random").clicked() {
                        let seed = (ui.input(|i| i.time) * 1000.0) as u32;
                        s = surprise(&s, seed);
                        play = Some(s.clone());
                    }
                    if ui.add_enabled(self.game.sounds.len() > 1, Button::new("Delete").min_size(vec2(70.0, 32.0))).clicked() {
                        delete = true;
                    }
                });

                if delete {
                    self.snapshot();
                    self.game.remove_sound(id);
                    self.sound_sel = None;
                } else if s != original {
                    let now = ui.input(|i| i.time);
                    self.snapshot_soft(now);
                    if let Some(slot) = self.game.sounds.iter_mut().find(|x| x.id == id) {
                        *slot = s;
                    }
                }
            }
        }

        // ---- music and volume ----
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("Music").heading());
        let mut music = self.game.music;
        ComboBox::from_id_salt("music").selected_text(music.label()).width(200.0).show_ui(ui, |ui| {
            for m in Music::ALL {
                ui.selectable_value(&mut music, m, m.label());
            }
        });
        let mut mv = self.game.music_volume;
        let mut sv = self.game.sfx_volume;
        let r1 = ui.add(Slider::new(&mut mv, 0.0..=1.0).text("Music"));
        let r2 = ui.add(Slider::new(&mut sv, 0.0..=1.0).text("Effects"));
        if music != self.game.music || mv != self.game.music_volume || sv != self.game.sfx_volume {
            self.game.music = music;
            self.game.music_volume = mv;
            self.game.sfx_volume = sv;
            self.dirty = true;
        }
        if r1.changed() {
            if let Some(a) = &mut self.audio {
                a.set_music_volume(mv);
            }
        }
        if r2.drag_stopped() {
            play = self.game.sounds.first().cloned();
        }
        ui.horizontal(|ui| {
            if ui.add(Button::new(if self.music_preview { "Stop the music" } else { "Hear the music" }).min_size(vec2(130.0, 30.0))).clicked() {
                self.ensure_audio();
                if self.music_preview {
                    if let Some(a) = &mut self.audio {
                        a.stop_music();
                    }
                    self.music_preview = false;
                } else if let Some(a) = &mut self.audio {
                    a.music_volume = mv;
                    a.set_music(music);
                    self.music_preview = true;
                }
            }
        });
        if self.music_preview && music != self.game.music {
            // (kept in sync above, nothing to do)
        }
        if self.music_preview {
            if let Some(a) = &mut self.audio {
                a.set_music(self.game.music);
            }
        }
        if self.audio_tried && self.audio.is_none() {
            ui.label(RichText::new("No sound device was found on this computer, so you can't hear sounds here.").small().color(Color32::from_rgb(200, 80, 80)));
        }

        if let Some(s) = play {
            self.preview_sound(&s);
        }
    }
}
