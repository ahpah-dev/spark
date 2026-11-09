//! Sounds, made from numbers. There are no sound files in Spark: every beep, boing and
//! tune is built here from simple waves, a bit like a tiny synthesizer.
//!
//! * [`Sound`] describes one sound effect (a few sliders: wave, pitch, slide, length...).
//! * [`Preset`] gives you good starting points like "Coin" or "Laser".
//! * [`Music`] is a short looping tune.
//!
//! Nothing here touches the speakers, so it is easy to test. See `audio.rs` for playing.

use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

pub const SAMPLE_RATE: u32 = 44_100;
pub type SoundId = u32;

/// Ids of the sounds every new game starts with.
pub mod ids {
    use super::SoundId;
    pub const JUMP: SoundId = 1;
    pub const COIN: SoundId = 2;
    pub const HURT: SoundId = 3;
    pub const WIN: SoundId = 4;
    pub const BOING: SoundId = 5;
    pub const SHOOT: SoundId = 6;
    pub const BOOM: SoundId = 7;
    pub const POWERUP: SoundId = 8;
    pub const BLIP: SoundId = 9;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Wave {
    Square,
    Triangle,
    Saw,
    Sine,
    Noise,
}

impl Wave {
    pub const ALL: [Wave; 5] = [Wave::Square, Wave::Triangle, Wave::Saw, Wave::Sine, Wave::Noise];

    pub fn label(self) -> &'static str {
        match self {
            Wave::Square => "Buzzy (square)",
            Wave::Triangle => "Soft (triangle)",
            Wave::Saw => "Rough (saw)",
            Wave::Sine => "Smooth (sine)",
            Wave::Noise => "Crunchy (noise)",
        }
    }
}

/// One sound effect.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Sound {
    pub id: SoundId,
    pub name: String,
    pub wave: Wave,
    /// How high it starts, in Hz (50 = very low, 2000 = very high).
    pub pitch: f32,
    /// How fast the pitch rises (positive) or falls (negative), in octaves per second.
    pub slide: f32,
    /// How long it lasts, in seconds.
    pub length: f32,
    pub volume: f32,
    /// Wobble (0 = steady, 1 = very wobbly).
    pub vibrato: f32,
    /// Halfway through, jump to a different note (in semitones). Makes "ba-ding!" sounds.
    pub extra_note: f32,
}

impl Sound {
    /// A number that changes whenever the sound changes (used to remember rendered sounds).
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u32| {
            h ^= v as u64;
            h = h.wrapping_mul(0x1000_0000_01b3);
        };
        mix(self.wave as u32);
        for f in [self.pitch, self.slide, self.length, self.volume, self.vibrato, self.extra_note] {
            mix(f.to_bits());
        }
        h
    }

    /// Turn the description into actual audio (mono, [`SAMPLE_RATE`]).
    pub fn render(&self) -> Vec<f32> {
        let length = self.length.clamp(0.03, 3.0);
        let n = (length * SAMPLE_RATE as f32) as usize;
        let mut out = Vec::with_capacity(n);
        let mut phase = 0.0f32;
        let mut rng = 0x1234_5678u32 ^ self.pitch.to_bits();
        let mut held = 0.0f32;
        for i in 0..n {
            let t = i as f32 / SAMPLE_RATE as f32;
            let x = t / length;
            let semis = if x >= 0.5 { self.extra_note } else { 0.0 };
            let vib = 1.0 + self.vibrato.clamp(0.0, 1.0) * 0.06 * (TAU * 9.0 * t).sin();
            let f = (self.pitch * (self.slide * t).exp2() * (semis / 12.0).exp2() * vib).clamp(20.0, 9000.0);
            phase += f / SAMPLE_RATE as f32;
            let wrapped = phase >= 1.0;
            if wrapped {
                phase -= 1.0;
            }
            let s = match self.wave {
                Wave::Square => {
                    if phase < 0.5 { 1.0 } else { -1.0 }
                }
                Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                Wave::Saw => 2.0 * phase - 1.0,
                Wave::Sine => (TAU * phase).sin(),
                Wave::Noise => {
                    if wrapped || i == 0 {
                        rng ^= rng << 13;
                        rng ^= rng >> 17;
                        rng ^= rng << 5;
                        held = (rng >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
                    }
                    held
                }
            };
            let attack = (t / 0.004).min(1.0);
            let decay = (1.0 - x).max(0.0).powf(1.6);
            out.push(s * self.volume.clamp(0.0, 1.0) * 0.5 * attack * decay);
        }
        out
    }
}

/// Good starting points for new sounds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    Coin,
    Jump,
    Hurt,
    Powerup,
    Laser,
    Explosion,
    Blip,
    Boing,
    Win,
}

impl Preset {
    pub const ALL: [Preset; 9] = [
        Preset::Coin,
        Preset::Jump,
        Preset::Hurt,
        Preset::Powerup,
        Preset::Laser,
        Preset::Explosion,
        Preset::Blip,
        Preset::Boing,
        Preset::Win,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Preset::Coin => "Coin",
            Preset::Jump => "Jump",
            Preset::Hurt => "Hurt",
            Preset::Powerup => "Power-up",
            Preset::Laser => "Laser",
            Preset::Explosion => "Explosion",
            Preset::Blip => "Blip",
            Preset::Boing => "Boing",
            Preset::Win => "Win",
        }
    }

    pub fn sound(self, id: SoundId) -> Sound {
        let (wave, pitch, slide, length, vibrato, extra) = match self {
            Preset::Coin => (Wave::Square, 988.0, 0.0, 0.22, 0.0, 5.0),
            Preset::Jump => (Wave::Square, 280.0, 2.0, 0.18, 0.0, 0.0),
            Preset::Hurt => (Wave::Saw, 240.0, -2.2, 0.32, 0.2, 0.0),
            Preset::Powerup => (Wave::Triangle, 330.0, 3.0, 0.4, 0.4, 0.0),
            Preset::Laser => (Wave::Square, 1400.0, -4.5, 0.2, 0.0, 0.0),
            Preset::Explosion => (Wave::Noise, 260.0, -2.0, 0.6, 0.0, 0.0),
            Preset::Blip => (Wave::Square, 660.0, 0.0, 0.08, 0.0, 0.0),
            Preset::Boing => (Wave::Sine, 200.0, 2.5, 0.4, 0.6, 0.0),
            Preset::Win => (Wave::Triangle, 523.0, 0.0, 0.7, 0.2, 7.0),
        };
        Sound { id, name: self.name().to_string(), wave, pitch, slide, length, volume: 0.8, vibrato, extra_note: extra }
    }
}

/// The sounds a new game starts with.
pub fn default_sounds() -> Vec<Sound> {
    use ids::*;
    vec![
        Preset::Jump.sound(JUMP),
        Preset::Coin.sound(COIN),
        Preset::Hurt.sound(HURT),
        Preset::Win.sound(WIN),
        Preset::Boing.sound(BOING),
        Preset::Laser.sound(SHOOT),
        Preset::Explosion.sound(BOOM),
        Preset::Powerup.sound(POWERUP),
        Preset::Blip.sound(BLIP),
    ]
}

/// Change a sound a little bit (the "Surprise me!" button). `seed` picks the surprise.
pub fn surprise(base: &Sound, seed: u32) -> Sound {
    let mut s = seed.wrapping_mul(2_654_435_761).wrapping_add(0x9e37_79b9) | 1;
    let mut r = move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s >> 8) as f32 / (1u32 << 24) as f32
    };
    let mut out = base.clone();
    out.pitch = (base.pitch * (0.6 + r() * 1.0)).clamp(60.0, 2400.0);
    out.slide = (base.slide + (r() - 0.5) * 3.0).clamp(-5.0, 5.0);
    out.length = (base.length * (0.7 + r() * 0.7)).clamp(0.06, 1.2);
    out.vibrato = if r() < 0.3 { r() * 0.8 } else { base.vibrato };
    out.extra_note = if r() < 0.4 { (r() * 12.0).round() } else { 0.0 };
    out
}

// ---------------------------------------------------------------------------
// Music
// ---------------------------------------------------------------------------

/// Little looping tunes you can pick for your game.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Music {
    #[default]
    None,
    Happy,
    Adventure,
    Spooky,
    Chill,
    Action,
}

struct Tune {
    root: i32,
    scale: &'static [i32],
    bpm: f32,
    lead_wave: Wave,
    lead: [i8; 32],
    bass: [i8; 8],
}

impl Music {
    pub const ALL: [Music; 6] = [Music::None, Music::Happy, Music::Adventure, Music::Spooky, Music::Chill, Music::Action];

    pub fn label(self) -> &'static str {
        match self {
            Music::None => "No music",
            Music::Happy => "Happy",
            Music::Adventure => "Adventure",
            Music::Spooky => "Spooky",
            Music::Chill => "Chill",
            Music::Action => "Action!",
        }
    }

    fn tune(self) -> Option<Tune> {
        const R: i8 = -1;
        Some(match self {
            Music::None => return None,
            Music::Happy => Tune {
                root: 60,
                scale: &[0, 2, 4, 7, 9],
                bpm: 140.0,
                lead_wave: Wave::Square,
                lead: [
                    4, R, 3, R, 2, R, 3, R, 4, R, 4, R, 4, R, R, R, 3, R, 3, R, 3, R, R, R, 4, R, 6, R, 6, R, R, R,
                ],
                bass: [0, 0, 3, 3, 2, 2, 3, 3],
            },
            Music::Adventure => Tune {
                root: 57,
                scale: &[0, 3, 5, 7, 10],
                bpm: 128.0,
                lead_wave: Wave::Square,
                lead: [
                    5, R, 7, 8, 7, R, 5, R, 3, R, 4, R, 5, R, R, R, 5, R, 7, 8, 9, R, 8, R, 7, R, 5, R, 4, R, 3, R,
                ],
                bass: [0, 0, 2, 2, 3, 3, 2, 2],
            },
            Music::Spooky => Tune {
                root: 52,
                scale: &[0, 2, 3, 6, 7],
                bpm: 90.0,
                lead_wave: Wave::Triangle,
                lead: [
                    5, R, R, R, 4, R, R, R, 3, R, 2, R, 3, R, R, R, 5, R, R, R, 6, R, R, R, 4, R, 3, R, 2, R, R, R,
                ],
                bass: [0, 0, 0, 0, 3, 3, 2, 2],
            },
            Music::Chill => Tune {
                root: 62,
                scale: &[0, 2, 4, 7, 9],
                bpm: 84.0,
                lead_wave: Wave::Sine,
                lead: [
                    2, R, R, 4, R, R, 3, R, R, 2, R, R, 0, R, R, R, 3, R, R, 4, R, R, 5, R, R, 4, R, R, 2, R, R, R,
                ],
                bass: [0, 0, 0, 0, 2, 2, 1, 1],
            },
            Music::Action => Tune {
                root: 55,
                scale: &[0, 3, 5, 7, 10],
                bpm: 164.0,
                lead_wave: Wave::Saw,
                lead: [
                    0, 0, 5, 0, 0, 0, 7, 0, 0, 0, 5, 0, 8, 7, 5, 3, 0, 0, 5, 0, 0, 0, 7, 0, 9, 8, 7, 5, 7, 5, 3, 0,
                ],
                bass: [0, 0, 0, 0, 2, 2, 3, 3],
            },
        })
    }

    /// One loop of the tune as audio (mono, [`SAMPLE_RATE`]). Empty for "No music".
    pub fn render(self) -> Vec<f32> {
        let Some(t) = self.tune() else { return Vec::new() };
        let step = 60.0 / t.bpm / 2.0; // an eighth note
        let total = (32.0 * step * SAMPLE_RATE as f32) as usize;
        let mut buf = vec![0.0f32; total];
        let hz = |midi: i32| 440.0 * ((midi - 69) as f32 / 12.0).exp2();
        let note = |deg: i8, octave_shift: i32| -> i32 {
            let n = t.scale.len() as i32;
            let d = deg as i32;
            t.root + t.scale[(d % n) as usize] + 12 * (d / n) + 12 * octave_shift
        };
        let mut add = |start: f32, dur: f32, freq: f32, wave: Wave, amp: f32| {
            let s0 = (start * SAMPLE_RATE as f32) as usize;
            let len = (dur * SAMPLE_RATE as f32) as usize;
            let mut phase = 0.0f32;
            for i in 0..len {
                if s0 + i >= total {
                    break;
                }
                phase = (phase + freq / SAMPLE_RATE as f32).fract();
                let w = match wave {
                    Wave::Square => {
                        if phase < 0.5 { 1.0 } else { -1.0 }
                    }
                    Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                    Wave::Saw => 2.0 * phase - 1.0,
                    _ => (TAU * phase).sin(),
                };
                let x = i as f32 / len as f32;
                let env = (i as f32 / (0.004 * SAMPLE_RATE as f32)).min(1.0) * (1.0 - x).powf(0.8);
                buf[s0 + i] += w * amp * env;
            }
        };
        for (i, &deg) in t.lead.iter().enumerate() {
            if deg >= 0 {
                add(i as f32 * step, step * 1.6, hz(note(deg, 1)), t.lead_wave, 0.16);
            }
        }
        for (q, &deg) in t.bass.iter().enumerate() {
            add(q as f32 * step * 4.0, step * 3.6, hz(note(deg, -1)), Wave::Triangle, 0.3);
        }
        for s in &mut buf {
            *s = s.clamp(-1.0, 1.0);
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_makes_sound_in_range() {
        for p in Preset::ALL {
            let s = p.sound(1);
            let audio = s.render();
            assert!(!audio.is_empty(), "{} is empty", p.name());
            assert!(audio.iter().all(|v| v.abs() <= 1.0), "{} is too loud", p.name());
            assert!(audio.iter().any(|v| v.abs() > 0.05), "{} is silent", p.name());
        }
    }

    #[test]
    fn length_matches_the_setting() {
        let mut s = Preset::Blip.sound(1);
        s.length = 0.5;
        let n = s.render().len() as f32 / SAMPLE_RATE as f32;
        assert!((n - 0.5).abs() < 0.01);
    }

    #[test]
    fn surprise_changes_things_but_stays_sensible() {
        let base = Preset::Coin.sound(1);
        let a = surprise(&base, 1);
        let b = surprise(&base, 2);
        assert_ne!(a.fingerprint(), b.fingerprint());
        for s in [a, b] {
            assert!(s.pitch >= 60.0 && s.pitch <= 2400.0 && s.length > 0.05);
        }
    }

    #[test]
    fn fingerprint_changes_when_sound_changes() {
        let mut s = Preset::Jump.sound(1);
        let f = s.fingerprint();
        s.pitch += 1.0;
        assert_ne!(f, s.fingerprint());
    }

    #[test]
    fn music_loops_have_length_and_stay_in_range() {
        for m in Music::ALL {
            let audio = m.render();
            if m == Music::None {
                assert!(audio.is_empty());
            } else {
                assert!(audio.len() > 44_100 * 4, "{} too short", m.label());
                assert!(audio.iter().all(|v| v.abs() <= 1.0));
                assert!(audio.iter().any(|v| v.abs() > 0.05));
            }
        }
    }
}
