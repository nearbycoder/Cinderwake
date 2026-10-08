//! Graphics fidelity: one saved choice that sets how much the renderer does.
//! Every step draws the same game; only presentation cost and quality change.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fidelity {
    /// No post-processing and half the particles, for weak graphics.
    Low,
    /// Bloom at half resolution and four combat lights.
    Medium,
    /// The look the game has always had.
    #[default]
    High,
    /// A supersampled, filtered scene with lit scenery, full-resolution
    /// bloom, filmic grading, and denser effects.
    Ultra,
}

impl Fidelity {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }
    pub fn index(self) -> u8 {
        self as u8
    }
    pub fn from_index(index: u8) -> Self {
        Self::ALL[(index as usize).min(Self::ALL.len() - 1)]
    }
    /// The step `delta` away, stopping at Low and Ultra.
    pub fn stepped(self, delta: i32) -> Self {
        Self::from_index((self.index() as i32 + delta).clamp(0, 3) as u8)
    }
    /// The next step, wrapping from Ultra to Low (for **F9**).
    pub fn cycled(self) -> Self {
        Self::from_index((self.index() + 1) % 4)
    }
    /// Parses a `--fidelity` launch value.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.name().eq_ignore_ascii_case(name))
    }
    /// Whether the scene is post-processed at all (bloom, grading, lights).
    pub fn post(self) -> bool {
        self != Self::Low
    }
    /// The bloom targets' size in pixels.
    pub fn bloom_size(self) -> (u32, u32) {
        match self {
            Self::Low | Self::Medium => (320, 180),
            Self::High => (640, 360),
            Self::Ultra => (1280, 720),
        }
    }
    /// How many dynamic lights the composite shader takes.
    pub fn lights(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Medium => 4,
            Self::High => 8,
            Self::Ultra => 16,
        }
    }
    /// The scene target's size as a multiple of 1280 × 720. Ultra draws the
    /// world at twice the size and filters it down: supersampling.
    pub fn scene_scale(self) -> u32 {
        if self == Self::Ultra {
            2
        } else {
            1
        }
    }
    /// Whether characters and scenery are sampled from mipmaps, so shrunken
    /// art keeps its detail instead of dropping pixels.
    pub fn smooth_textures(self) -> bool {
        self == Self::Ultra
    }
    /// Particles emitted per effect, relative to High.
    pub fn particle_density(self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium | Self::High => 1.,
            Self::Ultra => 1.5,
        }
    }
    /// The most particles alive at once.
    pub fn particle_cap(self) -> usize {
        match self {
            Self::Low => 256,
            Self::Medium | Self::High => 512,
            Self::Ultra => 1024,
        }
    }
    /// How many particles an effect asking for `count` emits at this step.
    /// High and Medium emit exactly `count`; any effect still emits one.
    /// Smoke and dust (`veils`) never thicken past High, so denser effects
    /// add sparks and debris without hiding guardians behind haze.
    pub fn particles(self, count: usize, veils: bool) -> usize {
        if count == 0 {
            return 0;
        }
        let density = if veils {
            self.particle_density().min(1.)
        } else {
            self.particle_density()
        };
        ((count as f32 * density).round() as usize).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_run_low_to_ultra_and_stop_or_wrap_at_the_ends() {
        assert_eq!(Fidelity::default(), Fidelity::High);
        assert_eq!(
            Fidelity::ALL.map(Fidelity::name),
            ["Low", "Medium", "High", "Ultra"]
        );
        for f in Fidelity::ALL {
            assert_eq!(Fidelity::from_index(f.index()), f);
            assert_eq!(Fidelity::parse(&f.name().to_uppercase()), Some(f));
        }
        assert_eq!(Fidelity::Low.stepped(-1), Fidelity::Low);
        assert_eq!(Fidelity::Ultra.stepped(1), Fidelity::Ultra);
        assert_eq!(Fidelity::Medium.stepped(2), Fidelity::Ultra);
        assert_eq!(Fidelity::Ultra.cycled(), Fidelity::Low);
        assert_eq!(Fidelity::Low.cycled(), Fidelity::Medium);
        assert_eq!(Fidelity::parse("epic"), None);
    }

    #[test]
    fn each_step_does_at_least_as_much_as_the_one_below() {
        for pair in Fidelity::ALL.windows(2) {
            let (lower, higher) = (pair[0], pair[1]);
            assert!(lower.lights() <= higher.lights());
            assert!(lower.bloom_size().0 <= higher.bloom_size().0);
            assert!(lower.particle_cap() <= higher.particle_cap());
            assert!(lower.particles(10, false) <= higher.particles(10, false));
            assert!(lower.scene_scale() <= higher.scene_scale());
        }
        assert!(!Fidelity::Low.post() && Fidelity::Medium.post());
    }

    #[test]
    fn high_emits_exactly_as_before_and_every_effect_still_shows() {
        for count in [0, 1, 5, 13, 33] {
            for veils in [false, true] {
                assert_eq!(Fidelity::High.particles(count, veils), count);
                assert_eq!(Fidelity::Medium.particles(count, veils), count);
            }
        }
        assert_eq!(Fidelity::Low.particles(1, false), 1);
        assert_eq!(Fidelity::Low.particles(20, true), 10);
        assert_eq!(Fidelity::Ultra.particles(20, false), 30);
        assert_eq!(
            Fidelity::Ultra.particles(20, true),
            20,
            "smoke and dust stay as thick as at High"
        );
        assert_eq!(
            (Fidelity::High.particle_cap(), Fidelity::High.lights()),
            (512, 8),
            "High keeps today's pool and light budget"
        );
    }
}
