//! Small named-file persistence for progress and settings. Desktop builds use
//! the platform's per-user data directory and replace files atomically; the
//! browser build stores the same bytes in `localStorage` (see `web/`).
#[cfg(target_arch = "wasm32")]
pub use web::{read, remove, write};

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
pub use desktop::{read, remove, write};

// Tests use a per-thread in-memory store, so no test can touch a real data
// folder. The desktop backend is still tested directly below.
#[cfg(test)]
pub use memory::{read, remove, write};

/// Copies a file that exists but couldn't be read to `<stem>.unreadable.json`,
/// or `<stem>.unreadable-2.json` and so on if other copies exist, so nothing
/// saved later can overwrite it. A file already kept with the same bytes
/// isn't copied again. Returns the copy's name.
pub fn keep_unreadable(name: &str, bytes: &[u8]) -> std::io::Result<String> {
    let stem = name.strip_suffix(".json").unwrap_or(name);
    let mut copies = (1..=9).map(|n| match n {
        1 => format!("{stem}.unreadable.json"),
        n => format!("{stem}.unreadable-{n}.json"),
    });
    loop {
        let Some(copy) = copies.next() else {
            return Err(std::io::Error::other(
                "nine unreadable copies are already kept",
            ));
        };
        match read(&copy) {
            Some(kept) if kept == bytes => return Ok(copy),
            Some(_) => {}
            None => return write(&copy, bytes).map(|()| copy),
        }
    }
}

#[cfg(test)]
mod memory {
    use std::{cell::RefCell, collections::HashMap, io};

    thread_local! {
        static FILES: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());
    }

    pub fn read(name: &str) -> Option<Vec<u8>> {
        FILES.with(|f| f.borrow().get(name).cloned())
    }

    pub fn write(name: &str, bytes: &[u8]) -> io::Result<()> {
        FILES.with(|f| f.borrow_mut().insert(name.into(), bytes.to_vec()));
        Ok(())
    }

    pub fn remove(name: &str) -> io::Result<()> {
        FILES.with(|f| f.borrow_mut().remove(name));
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::io;

    // Implemented by web/cinderwake-storage.js.
    extern "C" {
        fn cinderwake_storage_len(key: *const u8, key_len: u32) -> i32;
        fn cinderwake_storage_read(key: *const u8, key_len: u32, out: *mut u8, out_len: u32);
        fn cinderwake_storage_write(
            key: *const u8,
            key_len: u32,
            value: *const u8,
            value_len: u32,
        ) -> i32;
        fn cinderwake_storage_remove(key: *const u8, key_len: u32);
    }

    /// Lets the JS plugin confirm that it matches this build.
    #[no_mangle]
    pub extern "C" fn cinderwake_storage_crate_version() -> u32 {
        1
    }

    fn key(name: &str) -> String {
        format!("cinderwake/{name}")
    }

    pub fn read(name: &str) -> Option<Vec<u8>> {
        let key = key(name);
        // SAFETY: the plugin only reads `key` and writes at most `len` bytes to `out`.
        unsafe {
            let len = cinderwake_storage_len(key.as_ptr(), key.len() as u32);
            if len < 0 {
                return None;
            }
            let mut out = vec![0; len as usize];
            cinderwake_storage_read(key.as_ptr(), key.len() as u32, out.as_mut_ptr(), len as u32);
            Some(out)
        }
    }

    pub fn write(name: &str, bytes: &[u8]) -> io::Result<()> {
        let key = key(name);
        // SAFETY: the plugin only reads the two borrowed byte ranges.
        let stored = unsafe {
            cinderwake_storage_write(
                key.as_ptr(),
                key.len() as u32,
                bytes.as_ptr(),
                bytes.len() as u32,
            )
        };
        if stored == 1 {
            Ok(())
        } else {
            Err(io::Error::other("browser storage is unavailable"))
        }
    }

    pub fn remove(name: &str) -> io::Result<()> {
        let key = key(name);
        // SAFETY: the plugin only reads the borrowed key.
        unsafe { cinderwake_storage_remove(key.as_ptr(), key.len() as u32) };
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::{ffi::OsString, io, path::PathBuf};

    /// Per-user data directory for an operating system, given an environment lookup.
    pub(super) fn data_dir_for(os: &str, var: impl Fn(&str) -> Option<OsString>) -> PathBuf {
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

    /// Deletes a file; one that doesn't exist counts as removed. A copy left
    /// at the legacy path is removed too, so `read` can't fall back to it.
    pub fn remove(name: &str) -> io::Result<()> {
        for path in std::iter::once(data_dir().join(name)).chain(legacy_path(name)) {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::desktop::*;
    use std::{ffi::OsString, path::PathBuf};

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
    fn unreadable_copies_never_overwrite_each_other() {
        // `keep_unreadable` runs on the in-memory test store.
        for n in 1..=9u8 {
            let copy = super::keep_unreadable("limit.json", &[n]).unwrap();
            assert_eq!(super::read(&copy).unwrap(), [n]);
        }
        assert_eq!(
            super::keep_unreadable("limit.json", &[3]).unwrap(),
            "limit.unreadable-3.json",
            "a file already kept isn't copied again"
        );
        assert!(super::keep_unreadable("limit.json", &[10]).is_err());
        assert_eq!(super::read("limit.unreadable.json").unwrap(), [1]);
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
        remove("probe.json").unwrap();
        assert!(read("probe.json").is_none());
        remove("probe.json").expect("removing a missing file is fine");
        remove("legacy.json").unwrap();
        assert!(read("legacy.json").is_none(), "the legacy copy is gone too");
        assert!(data_dir().starts_with(&root));
        std::fs::remove_dir_all(root).ok();
    }
}
