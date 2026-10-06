//! Player preferences, stored apart from run progress. Settings change
//! presentation and comfort only; they never touch gameplay randomness.
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Levels are tenths of the original mix, 0..=10.
    pub music: u8,
    pub effects: u8,
    pub shake: u8,
    pub hitstop: bool,
    pub reduce_flashes: bool,
    pub postfx: bool,
    pub muted: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            music: 10,
            effects: 10,
            shake: 10,
            hitstop: true,
            reduce_flashes: false,
            postfx: true,
            muted: false,
        }
    }
}
impl Settings {
    pub const FILE: &str = "settings.json";
    pub const ROWS: usize = 6;

    pub fn load() -> Self {
        crate::storage::read(Self::FILE)
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
            .sanitized()
    }
    pub fn store(&self) -> io::Result<()> {
        crate::storage::write(Self::FILE, &serde_json::to_vec_pretty(self)?)
    }
    fn sanitized(mut self) -> Self {
        for level in [&mut self.music, &mut self.effects, &mut self.shake] {
            *level = (*level).min(10);
        }
        self
    }
    pub fn music_gain(&self) -> f32 {
        if self.muted {
            0.
        } else {
            0.5 * self.music as f32 / 10.
        }
    }
    pub fn effects_gain(&self) -> f32 {
        if self.muted {
            0.
        } else {
            0.35 * self.effects as f32 / 10.
        }
    }
    pub fn shake_scale(&self) -> f32 {
        self.shake as f32 / 10.
    }
    /// Multiplier for impact flashes, shockwave rings, and combat light bursts.
    pub fn flash_scale(&self) -> f32 {
        if self.reduce_flashes {
            0.3
        } else {
            1.
        }
    }
    /// Steps a row left (`-1`) or right (`1`); switches flip either way.
    pub fn adjust(&mut self, row: usize, delta: i32) {
        let step = |level: &mut u8| *level = (*level as i32 + delta).clamp(0, 10) as u8;
        match row {
            0 => step(&mut self.music),
            1 => step(&mut self.effects),
            2 => step(&mut self.shake),
            3 => self.hitstop = !self.hitstop,
            4 => self.reduce_flashes = !self.reduce_flashes,
            5 => self.postfx = !self.postfx,
            _ => {}
        }
    }
    /// Label, current value as a 0..=10 level (for gauges) or switch, and help text.
    pub fn row(&self, row: usize) -> (&'static str, RowValue, &'static str) {
        match row {
            0 => (
                "Music volume",
                RowValue::Level(self.music),
                "Ambient score loudness. M mutes everything.",
            ),
            1 => (
                "Effects volume",
                RowValue::Level(self.effects),
                "Strikes, footfalls, and explosions.",
            ),
            2 => (
                "Screen shake",
                RowValue::Level(self.shake),
                "Camera shake from hits, slams, and explosions.",
            ),
            3 => (
                "Hit-stop",
                RowValue::Switch(self.hitstop),
                "A brief freeze when strikes land. Off keeps motion continuous.",
            ),
            4 => (
                "Reduce flashes",
                RowValue::Switch(self.reduce_flashes),
                "Dims impact flashes, shockwave rings, and combat light bursts.",
            ),
            _ => (
                "Lighting and bloom",
                RowValue::Switch(self.postfx),
                "Post-processing, also toggled with F9. Turn off on slower graphics.",
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowValue {
    Level(u8),
    Switch(bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_reproduce_the_original_mix_and_feel() {
        let s = Settings::default();
        assert_eq!(s.music_gain(), 0.5);
        assert_eq!(s.effects_gain(), 0.35);
        assert_eq!(s.shake_scale(), 1.);
        assert_eq!(s.flash_scale(), 1.);
        assert!(s.hitstop && s.postfx && !s.muted);
    }

    #[test]
    fn adjustments_clamp_levels_and_flip_switches() {
        let mut s = Settings::default();
        s.adjust(0, 1);
        assert_eq!(s.music, 10);
        for _ in 0..12 {
            s.adjust(2, -1);
        }
        assert_eq!(s.shake, 0);
        assert_eq!(s.shake_scale(), 0.);
        s.adjust(3, 1);
        s.adjust(4, -1);
        assert!(!s.hitstop && s.reduce_flashes);
        assert!(s.flash_scale() < 1.);
        s.muted = true;
        assert_eq!(s.music_gain() + s.effects_gain(), 0.);
    }

    #[test]
    fn stored_settings_tolerate_missing_and_out_of_range_fields() {
        let s: Settings = serde_json::from_str("{\"shake\":4}").unwrap();
        assert_eq!(s.shake, 4);
        assert_eq!(s.music, 10);
        let s = serde_json::from_str::<Settings>("{\"effects\":250}")
            .unwrap()
            .sanitized();
        assert_eq!(s.effects, 10);
        let mut s = Settings::default();
        s.adjust(1, -3);
        let back: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }
}
