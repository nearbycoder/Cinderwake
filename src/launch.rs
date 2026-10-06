//! Testing shortcut: `--start-at <target>` (or `?start=<target>` in the browser
//! build) starts a practice run beside one kind of object, so interactions can
//! be checked with real key presses without first crossing a level.
use crate::game::{Game, Screen};
use crate::world::{EnemyKind, ObjectKind, FLOOR};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StartAt {
    Chest,
    Memory,
    Well,
    Forge,
    Cache,
    Gate,
    Keeper,
    Regent,
}

impl StartAt {
    pub const ALL: [Self; 8] = [
        Self::Chest,
        Self::Memory,
        Self::Well,
        Self::Forge,
        Self::Cache,
        Self::Gate,
        Self::Keeper,
        Self::Regent,
    ];
    /// Guardians within chase range of the start point (320 across, 150 up
    /// or down, plus a margin) are removed.
    const CLEAR: Vec2 = vec2(340., 160.);

    pub fn name(self) -> &'static str {
        match self {
            Self::Chest => "chest",
            Self::Memory => "memory",
            Self::Well => "well",
            Self::Forge => "forge",
            Self::Cache => "cache",
            Self::Gate => "gate",
            Self::Keeper => "keeper",
            Self::Regent => "regent",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.trim()))
    }

    pub fn names() -> String {
        Self::ALL.map(Self::name).join(", ")
    }

    /// Reads `--start-at <target>` or `--start-at=<target>`. An unknown or
    /// missing target is an error naming the valid ones.
    pub fn from_args(args: &[String]) -> Result<Option<Self>, String> {
        let mut value = None;
        for (i, arg) in args.iter().enumerate() {
            if let Some(v) = arg.strip_prefix("--start-at=") {
                value = Some(v.to_string());
            } else if arg == "--start-at" {
                value = Some(args.get(i + 1).cloned().unwrap_or_default());
            }
        }
        match value {
            None => Ok(None),
            Some(v) => Self::parse(&v).map(Some).ok_or_else(|| {
                format!(
                    "Unknown --start-at target {v:?}. Choose one of: {}.",
                    Self::names()
                )
            }),
        }
    }

    /// The browser build reads the page's `?start=` query instead.
    pub fn from_page() -> Option<Self> {
        page_query("start").and_then(|v| Self::parse(&v))
    }

    fn object(self) -> Option<ObjectKind> {
        Some(match self {
            Self::Chest => ObjectKind::Chest,
            Self::Memory => ObjectKind::Scroll,
            Self::Well => ObjectKind::Fountain,
            Self::Forge => ObjectKind::Forge,
            Self::Cache => ObjectKind::Secret,
            Self::Gate | Self::Keeper => ObjectKind::Exit,
            Self::Regent => return None,
        })
    }

    /// Stages a started run for this target. The run keeps its practice
    /// isolation; only position, nearby guardians, and the few resources an
    /// object needs (forge copper, cache kills) change.
    pub fn apply(self, g: &mut Game) {
        if self == Self::Regent {
            // Travel from the second stage lands in the Crown.
            g.stage = 1;
            g.travel();
            g.level.enemies.retain(|e| e.kind == EnemyKind::Regent);
            g.place_player(vec2(735., FLOOR));
        } else if let Some(kind) = self.object() {
            let pos = g
                .level
                .objects
                .iter()
                .find(|o| o.kind == kind)
                .map(|o| o.pos)
                .expect("every normal biome has each object kind");
            let start = vec2(pos.x - 20., pos.y);
            g.level.enemies.retain(|e| {
                let d = (e.pos - start).abs();
                e.kind == EnemyKind::Regent || d.x > Self::CLEAR.x || d.y > Self::CLEAR.y
            });
            g.place_player(start);
        }
        g.intro = 0.;
        match self {
            Self::Forge => g.player.gold = g.player.gold.max(60),
            Self::Cache => g.player.kills = g.player.kills.max(8),
            Self::Keeper => g.screen = Screen::Camp,
            _ => {}
        }
        g.notify(&format!(
            "Started at the {} (testing shortcut).",
            self.name()
        ));
    }
}

#[cfg(target_arch = "wasm32")]
fn page_query(name: &str) -> Option<String> {
    // Implemented by web/cinderwake-storage.js.
    extern "C" {
        fn cinderwake_query_len(key: *const u8, key_len: u32) -> i32;
        fn cinderwake_query_read(key: *const u8, key_len: u32, out: *mut u8, out_len: u32);
    }
    // SAFETY: the plugin only reads `name` and writes at most `len` bytes to `out`.
    unsafe {
        let len = cinderwake_query_len(name.as_ptr(), name.len() as u32);
        if len < 0 {
            return None;
        }
        let mut out = vec![0; len as usize];
        cinderwake_query_read(
            name.as_ptr(),
            name.len() as u32,
            out.as_mut_ptr(),
            len as u32,
        );
        String::from_utf8(out).ok()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn page_query(_name: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Input, Weapon};
    use crate::save::Save;
    use crate::world::STEP;

    fn started(target: StartAt) -> Game {
        let mut g = Game::new(4017, Save::default());
        g.practice = true;
        g.start();
        target.apply(&mut g);
        g
    }

    #[test]
    fn arguments_name_a_target_or_explain_the_choices() {
        let args = |s: &[&str]| s.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        assert_eq!(StartAt::from_args(&args(&["game"])), Ok(None));
        assert_eq!(
            StartAt::from_args(&args(&["game", "--start-at", "chest"])),
            Ok(Some(StartAt::Chest))
        );
        assert_eq!(
            StartAt::from_args(&args(&["game", "--start-at=Regent"])),
            Ok(Some(StartAt::Regent))
        );
        for bad in [&["game", "--start-at", "moon"][..], &["game", "--start-at"]] {
            let err = StartAt::from_args(&args(bad)).unwrap_err();
            assert!(err.contains("chest, memory, well"), "{err}");
        }
    }

    #[test]
    fn every_target_starts_beside_its_object_or_screen() {
        for target in StartAt::ALL {
            let mut g = started(target);
            assert!(g.practice, "{target:?} keeps practice isolation");
            // Settle onto the support and confirm nothing knocks the player away.
            for _ in 0..60 {
                g.tick(STEP, Input::default());
            }
            let p = g.player.pos;
            let support = g.level.support_at(p.x, p.y).expect("standing on a support");
            assert!((support.y - p.y).abs() < 1., "{target:?} stands on a ledge");
            assert!(g.player.hp >= g.player.max_hp, "{target:?} start is safe");
            assert!(
                g.level.enemies.iter().all(|e| e.kind == EnemyKind::Regent
                    || (e.pos.x - p.x).abs() > 320.
                    || (e.pos.y - p.y).abs() > 150.),
                "{target:?} leaves no guardian within chase range"
            );
            match target {
                StartAt::Keeper => assert_eq!(g.screen, Screen::Camp),
                StartAt::Regent => {
                    assert_eq!(g.level.biome, crate::world::Biome::Crown);
                    assert!(g.level.enemies.iter().all(|e| e.kind == EnemyKind::Regent));
                    assert_eq!(g.screen, Screen::Playing);
                }
                _ => {
                    let near = g.nearby().expect("an object within reach");
                    assert_eq!(Some(g.level.objects[near].kind), target.object());
                    assert_eq!(g.screen, Screen::Playing);
                }
            }
        }
    }

    #[test]
    fn staged_objects_can_be_used_straight_away() {
        let use_it = |target| {
            let mut g = started(target);
            g.tick(
                STEP,
                Input {
                    interact: true,
                    ..Default::default()
                },
            );
            g
        };
        let g = use_it(StartAt::Chest);
        // The fixed practice seed offers a different weapon, so the choice opens.
        assert_eq!(g.screen, Screen::Reliquary);
        assert_ne!(g.offer, Some(Weapon::Sabre));
        assert_eq!(use_it(StartAt::Memory).screen, Screen::Scroll);
        assert_eq!(use_it(StartAt::Forge).player.tier, 2);
        assert_eq!(use_it(StartAt::Cache).player.embers, 15);
        assert_eq!(use_it(StartAt::Gate).screen, Screen::Camp);
    }
}
