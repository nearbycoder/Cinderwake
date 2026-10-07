use crate::game::Weapon;
use crate::world::Biome;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io;

/// A saved file that existed but couldn't be read, and where its bytes went.
#[derive(Clone, Debug, PartialEq)]
pub struct Unreadable {
    pub file: &'static str,
    /// The copy's name, or why no copy could be made.
    pub kept: Result<String, String>,
}
/// Reads a JSON file. Bytes that exist but don't parse are copied aside
/// first: the defaults used instead are written back over the file as soon
/// as anything is saved, which used to lose the damaged file for good.
pub fn load_json<T: DeserializeOwned>(file: &'static str) -> (Option<T>, Option<Unreadable>) {
    let Some(bytes) = crate::storage::read(file) else {
        return (None, None);
    };
    // An empty file holds nothing worth keeping.
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return (None, None);
    }
    match serde_json::from_slice(&bytes) {
        Ok(value) => (Some(value), None),
        Err(_) => {
            let kept = crate::storage::keep_unreadable(file, &bytes).map_err(|e| e.to_string());
            (None, Some(Unreadable { file, kept }))
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Save {
    pub embers: u32,
    pub vitality: u32,
    pub flask: u32,
    pub rune: bool,
    pub wins: u32,
    pub runs: u32,
    pub best_kills: u32,
    /// The furthest stage (0–2) any run has reached, and the fastest
    /// victory in simulated seconds. Files from before these records lack
    /// them, so a run can't be marked as beating a record nobody kept.
    pub best_stage: Option<u32>,
    pub best_time: Option<f32>,
}
impl Save {
    pub const FILE: &str = "progress.json";
    /// Progress, and the damaged file if it couldn't be read.
    pub fn load() -> (Self, Option<Unreadable>) {
        let (save, unreadable) = load_json::<Self>(Self::FILE);
        (save.unwrap_or_default().sanitized(), unreadable)
    }
    /// Drops records no run could have set.
    fn sanitized(mut self) -> Self {
        self.best_stage = self.best_stage.map(|stage| stage.min(2));
        self.best_time = self.best_time.filter(|t| t.is_finite() && *t > 0.);
        self
    }
    pub fn store(&self) -> io::Result<()> {
        crate::storage::write(Self::FILE, &serde_json::to_vec_pretty(self)?)
    }
}
/// A run in progress, saved on arrival in each biome and at the Keeper so
/// that closing the game doesn't lose it. Continuing restores the run at the
/// start of that biome (or at the Keeper) with the build it had then.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub seed: u64,
    pub stage: u32,
    pub biome: Biome,
    /// Saved on reaching the Keeper, before choosing the next biome.
    pub at_keeper: bool,
    pub weapon: Weapon,
    pub tier: u32,
    pub power: [u32; 3],
    pub gold: u32,
    pub kills: u32,
    pub mutation: u32,
    pub max_hp: f32,
    pub run_time: f32,
}
impl Checkpoint {
    pub const FILE: &str = "run.json";
    /// A missing, unreadable, or inconsistent checkpoint is ignored.
    pub fn load() -> Option<Self> {
        crate::storage::read(Self::FILE).and_then(|bytes| Self::parse(&bytes))
    }
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let c: Self = serde_json::from_slice(bytes).ok()?;
        let route_fits = match c.stage {
            0 => c.biome == Biome::Aqueduct,
            1 => matches!(c.biome, Biome::Garden | Biome::Foundry),
            2 => c.biome == Biome::Crown && !c.at_keeper,
            _ => false,
        };
        let sane = c.max_hp.is_finite() && c.max_hp > 0. && c.run_time.is_finite();
        (route_fits && sane).then_some(c)
    }
    pub fn store(&self) -> io::Result<()> {
        crate::storage::write(Self::FILE, &serde_json::to_vec_pretty(self)?)
    }
    pub fn clear() -> io::Result<()> {
        crate::storage::remove(Self::FILE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_roundtrip_and_old_versions() {
        let s = Save {
            embers: 42,
            rune: true,
            ..Default::default()
        };
        let r: Save = serde_json::from_slice(&serde_json::to_vec(&s).unwrap()).unwrap();
        assert_eq!(r.embers, 42);
        assert!(r.rune);
        let old: Save = serde_json::from_str("{\"embers\":9}").unwrap();
        assert_eq!(old.flask, 0);
        assert_eq!(
            (old.best_stage, old.best_time),
            (None, None),
            "no records yet"
        );
        let odd: Save = serde_json::from_str("{\"best_stage\":9,\"best_time\":-4}").unwrap();
        let odd = odd.sanitized();
        assert_eq!((odd.best_stage, odd.best_time), (Some(2), None));
        assert!(serde_json::from_str::<Save>("broken").is_err());
    }
    #[test]
    fn checkpoints_round_trip_and_reject_damage_or_impossible_runs() {
        let c = Checkpoint {
            seed: 7,
            stage: 1,
            biome: Biome::Foundry,
            at_keeper: false,
            weapon: Weapon::Hammer,
            tier: 4,
            power: [1, 2, 3],
            gold: 50,
            kills: 12,
            mutation: 1,
            max_hp: 140.,
            run_time: 300.,
        };
        let bytes = serde_json::to_vec(&c).unwrap();
        assert_eq!(Checkpoint::parse(&bytes), Some(c.clone()));
        assert_eq!(Checkpoint::parse(b"{\"seed\":1"), None, "truncated");
        assert_eq!(Checkpoint::parse(b"[]"), None);
        let with = |f: &dyn Fn(&mut Checkpoint)| {
            let mut bad = c.clone();
            f(&mut bad);
            Checkpoint::parse(&serde_json::to_vec(&bad).unwrap())
        };
        assert_eq!(
            with(&|b| b.biome = Biome::Crown),
            None,
            "stage 1 isn't the Crown"
        );
        assert_eq!(with(&|b| b.stage = 5), None);
        assert_eq!(
            with(&|b| (b.stage, b.biome, b.at_keeper) = (2, Biome::Crown, true)),
            None
        );
        assert_eq!(with(&|b| b.max_hp = 0.), None);
        assert!(with(&|b| (b.stage, b.biome) = (0, Biome::Aqueduct)).is_some());
    }
}
