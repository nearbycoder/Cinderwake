use serde::{Deserialize, Serialize};
use std::io;
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
}
impl Save {
    pub const FILE: &str = "progress.json";
    pub fn load() -> Self {
        crate::storage::read(Self::FILE)
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default()
    }
    pub fn store(&self) -> io::Result<()> {
        crate::storage::write(Self::FILE, &serde_json::to_vec_pretty(self)?)
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
        assert!(serde_json::from_str::<Save>("broken").is_err());
    }
}
