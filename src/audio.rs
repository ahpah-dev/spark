//! Playing sounds through the speakers.
//!
//! `Audio::new()` gives `None` if the computer has no sound device. The game keeps working
//! without sound, so we never crash because of it.

use crate::sound::{Music, SAMPLE_RATE, Sound};
use rodio::buffer::SamplesBuffer;
use rodio::source::Source;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::HashMap;
use std::num::NonZero;

pub struct Audio {
    sink: MixerDeviceSink,
    music_player: Option<Player>,
    music_now: Music,
    /// Rendered sounds, remembered so we only build each one once.
    cache: HashMap<u64, Vec<f32>>,
    pub sfx_volume: f32,
    pub music_volume: f32,
}

impl Audio {
    pub fn new() -> Option<Audio> {
        let mut sink = DeviceSinkBuilder::open_default_sink().ok()?;
        sink.log_on_drop(false);
        Some(Audio { sink, music_player: None, music_now: Music::None, cache: HashMap::new(), sfx_volume: 1.0, music_volume: 0.5 })
    }

    fn buffer(samples: Vec<f32>) -> SamplesBuffer {
        SamplesBuffer::new(NonZero::new(1u16).unwrap(), NonZero::new(SAMPLE_RATE).unwrap(), samples)
    }

    /// Play a sound effect once. Many can play at the same time.
    pub fn play(&mut self, sound: &Sound) {
        if self.cache.len() > 200 {
            self.cache.clear();
        }
        let samples = self.cache.entry(sound.fingerprint()).or_insert_with(|| sound.render()).clone();
        self.sink.mixer().add(Self::buffer(samples).amplify(self.sfx_volume.clamp(0.0, 1.0)));
    }

    /// Start (or change) the background music. Does nothing if it is already playing.
    pub fn set_music(&mut self, music: Music) {
        if music == self.music_now && self.music_player.is_some() == (music != Music::None) {
            return;
        }
        self.stop_music();
        self.music_now = music;
        if music == Music::None {
            return;
        }
        let player = Player::connect_new(self.sink.mixer());
        player.append(Self::buffer(music.render()).repeat_infinite());
        player.set_volume(self.music_volume.clamp(0.0, 1.0) * 0.6);
        self.music_player = Some(player);
    }

    pub fn set_music_volume(&mut self, v: f32) {
        self.music_volume = v;
        if let Some(p) = &self.music_player {
            p.set_volume(v.clamp(0.0, 1.0) * 0.6);
        }
    }

    pub fn stop_music(&mut self) {
        if let Some(p) = self.music_player.take() {
            p.stop();
        }
        self.music_now = Music::None;
    }
}
