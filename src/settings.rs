//! Player preferences, stored apart from run progress. Settings change
//! presentation and comfort only; they never touch gameplay randomness.
use crate::controls::Bindings;
use crate::fidelity::Fidelity;
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
    /// Kept in step with `fidelity` (off only at Low), so builds from
    /// before the fidelity setting read the file sensibly.
    pub postfx: bool,
    pub fidelity: Fidelity,
    pub muted: bool,
    /// Desktop builds start fullscreen when this is saved on.
    pub fullscreen: bool,
    /// One-time contextual tips, and which of them have been shown.
    pub hints: bool,
    pub hints_seen: u32,
    /// Gameplay key bindings, stored by name.
    pub keys: Bindings,
    /// Simulation speed in play, in tenths: 5 (half speed) to 10.
    pub speed: u8,
    /// The desktop window's last size in pixels, restored at launch on Linux.
    pub window: Option<[u32; 2]>,
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
            fidelity: Fidelity::default(),
            muted: false,
            fullscreen: false,
            hints: true,
            hints_seen: 0,
            keys: Bindings::default(),
            speed: 10,
            window: None,
        }
    }
}
impl Settings {
    pub const FILE: &str = "settings.json";
    pub const ROWS: usize = 10;
    /// The row that opens the controls page instead of adjusting a value.
    pub const CONTROLS_ROW: usize = 9;
    pub const SLOWEST: u8 = 5;

    /// Settings, and the damaged file if it couldn't be read.
    pub fn load() -> (Self, Option<crate::save::Unreadable>) {
        let (settings, unreadable) = crate::save::load_json::<Self>(Self::FILE);
        (settings.unwrap_or_default().sanitized(), unreadable)
    }
    pub fn store(&self) -> io::Result<()> {
        crate::storage::write(Self::FILE, &serde_json::to_vec_pretty(self)?)
    }
    fn sanitized(mut self) -> Self {
        for level in [&mut self.music, &mut self.effects, &mut self.shake] {
            *level = (*level).min(10);
        }
        self.speed = self.speed.clamp(Self::SLOWEST, 10);
        self.window = self.window.filter(|size| Self::window_fits(*size));
        // Files from before the fidelity setting saved lighting off as
        // `postfx: false`, which is Low now.
        if !self.postfx {
            self.fidelity = Fidelity::Low;
        }
        self.postfx = self.fidelity.post();
        self
    }
    /// The row that sets graphics fidelity.
    pub const FIDELITY_ROW: usize = 5;
    /// Sets graphics fidelity, keeping `postfx` in step.
    pub fn set_fidelity(&mut self, fidelity: Fidelity) {
        self.fidelity = fidelity;
        self.postfx = fidelity.post();
    }
    /// The window opens at 1280 × 720 pixels unless another size was saved.
    pub const DEFAULT_WINDOW: [u32; 2] = [1280, 720];
    /// Whether a window size is worth keeping: at least the world's own
    /// 640 × 360, and no larger than any screen.
    pub fn window_fits([w, h]: [u32; 2]) -> bool {
        (640..=16384).contains(&w) && (360..=16384).contains(&h)
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
    /// How fast play runs: 1 is normal speed. Every part of the simulation
    /// slows evenly, so only the time to react changes.
    pub fn speed_scale(&self) -> f32 {
        self.speed as f32 / 10.
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
            Self::FIDELITY_ROW => self.set_fidelity(self.fidelity.stepped(delta)),
            6 => {
                // Switching tips back on replays them from the start.
                self.hints = !self.hints;
                if self.hints {
                    self.hints_seen = 0;
                }
            }
            7 => self.fullscreen = !self.fullscreen,
            8 => self.speed = (self.speed as i32 + delta).clamp(Self::SLOWEST as i32, 10) as u8,
            _ => {}
        }
    }
    /// Sets a level row directly, as a click on its bar does, within the
    /// same limits as stepping. Other rows are unchanged.
    pub fn set_level(&mut self, row: usize, level: u8) {
        match self.row(row).1 {
            RowValue::Level(now) | RowValue::Steps(now, _) => {
                self.adjust(row, level as i32 - now as i32)
            }
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
                "Strikes, tools, kills, menus, and explosions.",
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
            Self::FIDELITY_ROW => (
                "Graphics fidelity",
                RowValue::Steps(self.fidelity.index(), &FIDELITY_STEPS),
                match self.fidelity {
                    Fidelity::Low => "Low: no lighting or bloom, fewer particles. For slower graphics. F9 cycles.",
                    Fidelity::Medium => "Medium: softer bloom and fewer combat lights. F9 cycles.",
                    Fidelity::High => "High: bloom, grading, and combat lighting. F9 cycles.",
                    Fidelity::Ultra => "Ultra: smoothed scene, lit scenery, richer bloom, more particles. F9 cycles.",
                },
            ),
            6 => (
                "Gameplay tips",
                RowValue::Switch(self.hints),
                "One-time tips when a mechanic first matters. Switch on again to replay them.",
            ),
            7 => (
                "Fullscreen",
                RowValue::Switch(self.fullscreen),
                "Fill the screen, also toggled with F11. Desktop builds remember it.",
            ),
            8 => (
                "Game speed",
                RowValue::Level(self.speed),
                "Slows everything in play evenly, for more time to react. Menus and music keep pace.",
            ),
            _ => (
                "Controls",
                RowValue::Page,
                "Rebind gameplay keys. Arrow keys and mouse buttons always work too.",
            ),
        }
    }
}

const FIDELITY_STEPS: [&str; 4] = ["LOW", "MEDIUM", "HIGH", "ULTRA"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowValue {
    Level(u8),
    /// One of a few named steps, chosen like a level.
    Steps(u8, &'static [&'static str]),
    Switch(bool),
    /// Opens another page with Enter.
    Page,
}
impl RowValue {
    /// Bars and steps are adjusted left and right, with arrows when selected.
    pub fn is_bar(self) -> bool {
        matches!(self, Self::Level(_) | Self::Steps(..))
    }
    /// The values a click on one of the bar's segments sets, in order.
    pub fn segments(self) -> Vec<u8> {
        match self {
            Self::Level(_) => (1..=10).collect(),
            Self::Steps(_, names) => (0..names.len() as u8).collect(),
            _ => vec![],
        }
    }
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
        assert!(s.hitstop && s.postfx && !s.muted && s.hints && !s.fullscreen);
        assert_eq!(s.speed_scale(), 1.);
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
        s.hints_seen = 0b101;
        s.adjust(6, 1);
        assert!(!s.hints);
        assert_eq!(s.hints_seen, 0b101, "switching tips off keeps progress");
        s.adjust(6, 1);
        assert!(s.hints);
        assert_eq!(s.hints_seen, 0, "switching tips on replays them");
        s.adjust(7, -1);
        assert!(s.fullscreen);
        assert_eq!(s.row(7).1, RowValue::Switch(true));
        assert_eq!(s.row(Settings::CONTROLS_ROW).1, RowValue::Page);
        let back: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert!(back.fullscreen, "fullscreen is saved");
        s.adjust(8, 1);
        assert_eq!(s.speed, 10, "normal speed is the fastest");
        for _ in 0..8 {
            s.adjust(8, -1);
        }
        assert_eq!(
            (s.speed, s.speed_scale()),
            (5, 0.5),
            "half speed is the slowest"
        );
        assert_eq!(s.row(8).1, RowValue::Level(5));
        let back: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back.speed, 5, "game speed is saved");
    }

    #[test]
    fn levels_can_be_set_directly_within_their_limits() {
        let mut s = Settings::default();
        s.set_level(0, 3);
        assert_eq!(s.music, 3);
        s.set_level(2, 0);
        assert_eq!(s.shake, 0);
        s.set_level(8, 2);
        assert_eq!(s.speed, Settings::SLOWEST, "speed stops at half");
        let before = s.clone();
        s.set_level(3, 1);
        assert_eq!(s, before, "switches aren't levels");
    }

    #[test]
    fn graphics_fidelity_steps_like_a_bar_and_keeps_postfx_in_step() {
        let mut s = Settings::default();
        let row = Settings::FIDELITY_ROW;
        assert_eq!(s.fidelity, Fidelity::High, "today's look is the default");
        assert_eq!(s.row(row).1, RowValue::Steps(2, &FIDELITY_STEPS));
        assert!(s.row(row).1.is_bar());
        assert_eq!(s.row(row).1.segments(), vec![0, 1, 2, 3]);
        s.adjust(row, 1);
        assert_eq!(s.fidelity, Fidelity::Ultra);
        s.adjust(row, 1);
        assert_eq!(s.fidelity, Fidelity::Ultra, "stops at Ultra");
        for _ in 0..5 {
            s.adjust(row, -1);
        }
        assert_eq!((s.fidelity, s.postfx), (Fidelity::Low, false));
        s.set_level(row, 1);
        assert_eq!((s.fidelity, s.postfx), (Fidelity::Medium, true));
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"fidelity\":\"medium\""), "{json}");
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.sanitized().fidelity, Fidelity::Medium);
    }

    #[test]
    fn files_saved_before_fidelity_keep_their_lighting_choice() {
        let load = |json: &str| serde_json::from_str::<Settings>(json).unwrap().sanitized();
        assert_eq!(load("{}").fidelity, Fidelity::High);
        assert_eq!(load("{\"postfx\":true}").fidelity, Fidelity::High);
        let off = load("{\"postfx\":false}");
        assert_eq!((off.fidelity, off.postfx), (Fidelity::Low, false));
        let ultra = load("{\"fidelity\":\"ultra\",\"postfx\":true}");
        assert_eq!((ultra.fidelity, ultra.postfx), (Fidelity::Ultra, true));
        let low = load("{\"fidelity\":\"low\",\"postfx\":true}");
        assert!(!low.postfx, "postfx follows the saved fidelity");
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
        assert_eq!(
            s.speed, 10,
            "files from before game speed run at full speed"
        );
        let s = serde_json::from_str::<Settings>("{\"speed\":2}")
            .unwrap()
            .sanitized();
        assert_eq!(s.speed, Settings::SLOWEST);
        let mut s = Settings::default();
        s.adjust(1, -3);
        s.window = Some([1920, 1080]);
        let back: Settings = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn only_sensible_window_sizes_are_kept() {
        let size = |json: &str| {
            serde_json::from_str::<Settings>(json)
                .unwrap()
                .sanitized()
                .window
        };
        assert_eq!(size("{}"), None, "files from before window sizes");
        assert_eq!(size("{\"window\":[1600,900]}"), Some([1600, 900]));
        assert_eq!(size("{\"window\":[640,360]}"), Some([640, 360]));
        for small_or_huge in ["[639,720]", "[1280,359]", "[0,0]", "[20000,900]"] {
            assert_eq!(
                size(&format!("{{\"window\":{small_or_huge}}}")),
                None,
                "{small_or_huge}"
            );
        }
    }
}
