//! Small named-file persistence for progress and settings. Desktop builds use
//! the platform's per-user data directory and replace files atomically.
use std::{ffi::OsString, io, path::PathBuf};

/// Per-user data directory for an operating system, given an environment lookup.
fn data_dir_for(os: &str, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    let home = || var("HOME").map(PathBuf::from).unwrap_or_else(|| ".".into());
    let nonempty = |name| var(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    match os {
        "macos" => home().join("Library/Application Support/Cinderwake"),
        "windows" => nonempty("APPDATA")
            .unwrap_or_else(|| home().join("AppData/Roaming"))
            .join("Cinderwake"),
        _ => nonempty("XDG_DATA_HOME")
            .unwrap_or_else(|| home().join(".local/share"))
            .join("cinderwake"),
    }
}

pub fn data_dir() -> PathBuf {
    data_dir_for(std::env::consts::OS, |name| std::env::var_os(name))
}

/// Before per-platform directories, every build saved beneath the macOS path.
fn legacy_path(name: &str) -> Option<PathBuf> {
    let legacy = data_dir_for("macos", |name| std::env::var_os(name)).join(name);
    (legacy != data_dir().join(name)).then_some(legacy)
}

pub fn read(name: &str) -> Option<Vec<u8>> {
    std::fs::read(data_dir().join(name))
        .ok()
        .or_else(|| std::fs::read(legacy_path(name)?).ok())
}

pub fn write(name: &str, bytes: &[u8]) -> io::Result<()> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| OsString::from(v))
        }
    }

    #[test]
    fn data_directory_follows_platform_conventions() {
        let home = [("HOME", "/home/a")];
        assert_eq!(
            data_dir_for("linux", env(&home)),
            PathBuf::from("/home/a/.local/share/cinderwake")
        );
        assert_eq!(
            data_dir_for(
                "linux",
                env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "/d")])
            ),
            PathBuf::from("/d/cinderwake")
        );
        // An empty XDG variable is treated as unset, per the base directory spec.
        assert_eq!(
            data_dir_for("linux", env(&[("HOME", "/home/a"), ("XDG_DATA_HOME", "")])),
            PathBuf::from("/home/a/.local/share/cinderwake")
        );
        assert_eq!(
            data_dir_for(
                "macos",
                env(&[("HOME", "/Users/a"), ("XDG_DATA_HOME", "/d")])
            ),
            PathBuf::from("/Users/a/Library/Application Support/Cinderwake")
        );
        assert_eq!(
            data_dir_for("windows", env(&[("APPDATA", "C:/Users/a/AppData/Roaming")])),
            PathBuf::from("C:/Users/a/AppData/Roaming/Cinderwake")
        );
    }

    #[test]
    fn files_round_trip_atomically_through_the_data_directory() {
        // The only test that touches the process environment; the directory is
        // private to this process so real player saves are never involved.
        let root = std::env::temp_dir().join(format!("cinderwake-test-{}", std::process::id()));
        std::env::set_var("HOME", &root);
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
        std::env::set_var("APPDATA", root.join("appdata"));
        assert!(read("probe.json").is_none());
        // Saves written by earlier builds at the macOS-shaped path still load.
        let legacy = root.join("Library/Application Support/Cinderwake");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("legacy.json"), b"{}").unwrap();
        assert_eq!(read("legacy.json").unwrap(), b"{}");
        write("probe.json", b"{\"embers\":7}").unwrap();
        assert_eq!(read("probe.json").unwrap(), b"{\"embers\":7}");
        assert!(!data_dir().join("probe.tmp").exists());
        assert!(data_dir().starts_with(&root));
        std::fs::remove_dir_all(root).ok();
    }
}
