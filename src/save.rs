use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};
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
    pub fn path() -> PathBuf {
        let base = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("Library/Application Support/Cinderwake/progress.json")
    }
    pub fn load() -> Self {
        std::fs::read(Self::path())
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default()
    }
    pub fn store(&self) -> io::Result<()> {
        let path = Self::path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, path)
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
