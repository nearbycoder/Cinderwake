//! Rebindable gameplay keys. Menu keys (Escape, Enter, Tab, M, function keys)
//! stay fixed, and the arrow keys and mouse buttons remain alternatives for
//! movement, strikes, and parries whatever the bindings say.
use crate::game::Input;
use macroquad::prelude::KeyCode;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Left,
    Right,
    Jump,
    Down,
    Strike,
    Glassbolt,
    Dodge,
    Parry,
    FireVessel,
    ArcSnare,
    Heal,
    Interact,
}
impl Action {
    pub const ALL: [Action; 12] = [
        Self::Left,
        Self::Right,
        Self::Jump,
        Self::Down,
        Self::Strike,
        Self::Glassbolt,
        Self::Dodge,
        Self::Parry,
        Self::FireVessel,
        Self::ArcSnare,
        Self::Heal,
        Self::Interact,
    ];
    fn id(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Jump => "jump",
            Self::Down => "down",
            Self::Strike => "strike",
            Self::Glassbolt => "glassbolt",
            Self::Dodge => "dodge",
            Self::Parry => "parry",
            Self::FireVessel => "fire_vessel",
            Self::ArcSnare => "arc_snare",
            Self::Heal => "heal",
            Self::Interact => "interact",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Move left",
            Self::Right => "Move right",
            Self::Jump => "Jump",
            Self::Down => "Drop / slam",
            Self::Strike => "Strike",
            Self::Glassbolt => "Glassbolt",
            Self::Dodge => "Dodge",
            Self::Parry => "Parry",
            Self::FireVessel => "Fire vessel",
            Self::ArcSnare => "Arc snare",
            Self::Heal => "Heal",
            Self::Interact => "Interact",
        }
    }
    fn defaults(self) -> &'static [KeyCode] {
        match self {
            Self::Left => &[KeyCode::A],
            Self::Right => &[KeyCode::D],
            Self::Jump => &[KeyCode::Space, KeyCode::W],
            Self::Down => &[KeyCode::S],
            Self::Strike => &[KeyCode::J],
            Self::Glassbolt => &[KeyCode::K],
            Self::Dodge => &[KeyCode::LeftShift, KeyCode::RightShift],
            Self::Parry => &[KeyCode::L],
            Self::FireVessel => &[KeyCode::Q],
            Self::ArcSnare => &[KeyCode::R],
            Self::Heal => &[KeyCode::F],
            Self::Interact => &[KeyCode::E],
        }
    }
}

/// Every bindable key and its stored and displayed name. Keys missing here
/// (menu keys, M, arrows, function keys) cannot be bound.
const KEYS: &[(KeyCode, &str)] = &[
    (KeyCode::A, "A"),
    (KeyCode::B, "B"),
    (KeyCode::C, "C"),
    (KeyCode::D, "D"),
    (KeyCode::E, "E"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::I, "I"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::N, "N"),
    (KeyCode::O, "O"),
    (KeyCode::P, "P"),
    (KeyCode::Q, "Q"),
    (KeyCode::R, "R"),
    (KeyCode::S, "S"),
    (KeyCode::T, "T"),
    (KeyCode::U, "U"),
    (KeyCode::V, "V"),
    (KeyCode::W, "W"),
    (KeyCode::X, "X"),
    (KeyCode::Y, "Y"),
    (KeyCode::Z, "Z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::Space, "SPACE"),
    (KeyCode::LeftShift, "LSHIFT"),
    (KeyCode::RightShift, "RSHIFT"),
    (KeyCode::LeftControl, "LCTRL"),
    (KeyCode::RightControl, "RCTRL"),
    (KeyCode::LeftAlt, "LALT"),
    (KeyCode::RightAlt, "RALT"),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Slash, "/"),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Backslash, "\\"),
    (KeyCode::Minus, "-"),
    (KeyCode::Equal, "="),
    (KeyCode::GraveAccent, "`"),
    (KeyCode::Backspace, "BACKSPACE"),
];

fn name(key: KeyCode) -> Option<&'static str> {
    KEYS.iter().find(|(k, _)| *k == key).map(|(_, n)| *n)
}
fn from_name(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|(_, n)| *n == name).map(|(k, _)| *k)
}
/// Display name; left and right modifiers read as one key.
fn display(key: KeyCode) -> &'static str {
    match name(key).unwrap_or("?") {
        "LSHIFT" | "RSHIFT" => "SHIFT",
        "LCTRL" | "RCTRL" => "CTRL",
        "LALT" | "RALT" => "ALT",
        other => other,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    from = "BTreeMap<String, Vec<String>>",
    into = "BTreeMap<String, Vec<String>>"
)]
pub struct Bindings([Vec<KeyCode>; 12]);
impl Default for Bindings {
    fn default() -> Self {
        Self(Action::ALL.map(|a| a.defaults().to_vec()))
    }
}
impl From<BTreeMap<String, Vec<String>>> for Bindings {
    /// Unknown names are dropped and missing actions take their defaults. A
    /// key claimed twice means the file was edited by hand: start over.
    fn from(stored: BTreeMap<String, Vec<String>>) -> Self {
        let bindings = Self(Action::ALL.map(|a| {
            let keys: Vec<_> = stored
                .get(a.id())
                .map(|names| names.iter().filter_map(|n| from_name(n)).collect())
                .unwrap_or_default();
            if keys.is_empty() {
                a.defaults().to_vec()
            } else {
                keys
            }
        }));
        let all: Vec<_> = bindings.0.iter().flatten().collect();
        let unique = all.iter().enumerate().all(|(i, k)| !all[..i].contains(k));
        if unique {
            bindings
        } else {
            Self::default()
        }
    }
}
impl From<Bindings> for BTreeMap<String, Vec<String>> {
    fn from(bindings: Bindings) -> Self {
        Action::ALL
            .iter()
            .zip(bindings.0)
            .map(|(a, keys)| {
                let names = keys.iter().filter_map(|k| name(*k)).map(String::from);
                (a.id().to_string(), names.collect())
            })
            .collect()
    }
}
impl Bindings {
    fn slot(action: Action) -> usize {
        Action::ALL.iter().position(|a| *a == action).unwrap()
    }
    pub fn keys(&self, action: Action) -> &[KeyCode] {
        &self.0[Self::slot(action)]
    }
    /// The first key, for compact badges and prompts.
    pub fn short(&self, action: Action) -> &'static str {
        self.keys(action).first().map_or("?", |k| display(*k))
    }
    /// Every key for an action, for the controls page.
    pub fn label(&self, action: Action) -> String {
        let mut names: Vec<&str> = vec![];
        for key in self.keys(action) {
            let shown = display(*key);
            if !names.contains(&shown) {
                names.push(shown);
            }
        }
        names.join(" / ")
    }
    /// Binds `key` as the only key for `action`. If another action used it,
    /// that action loses it and, if left with none, takes `action`'s previous
    /// key. Returns the action that changed, or `Err` for keys that can't be bound.
    pub fn bind(&mut self, action: Action, key: KeyCode) -> Result<Option<Action>, ()> {
        name(key).ok_or(())?;
        let slot = Self::slot(action);
        let previous = self.0[slot].first().copied();
        self.0[slot] = vec![key];
        let mut moved = None;
        for (i, keys) in self.0.iter_mut().enumerate() {
            if i == slot || !keys.contains(&key) {
                continue;
            }
            keys.retain(|k| *k != key);
            if keys.is_empty() {
                keys.extend(previous.filter(|p| *p != key));
            }
            moved = Some(Action::ALL[i]);
        }
        Ok(moved)
    }
}

/// Fixed menu keys, named in prompts alongside the gameplay actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Confirm,
    Back,
    Pause,
    Atlas,
    Options,
    NewRun,
    Abandon,
    /// Quitting the desktop game from the title screen or the pause screen.
    QuitTitle,
    QuitPaused,
    /// Moving between rows, changing a value, or picking a route.
    Rows,
    Adjust,
    /// The first, second, or third of a screen's numbered choices.
    Choice(usize),
}

/// Names for on-screen prompts: the player's keys, or the controller's fixed
/// layout while a controller was the last thing used.
#[derive(Clone, Copy)]
pub struct Prompts<'a> {
    keys: &'a Bindings,
    pad: bool,
}
impl<'a> Prompts<'a> {
    pub fn new(keys: &'a Bindings, pad: bool) -> Self {
        Self { keys, pad }
    }
    pub fn pad(&self) -> bool {
        self.pad
    }
    pub fn action(&self, action: Action) -> &'static str {
        if self.pad {
            crate::pad::label(action)
        } else {
            self.keys.short(action)
        }
    }
    /// Both directions of movement, for control lists.
    pub fn movement(&self) -> String {
        if self.pad {
            "STICK".into()
        } else {
            format!(
                "{} / {}",
                self.keys.short(Action::Left),
                self.keys.short(Action::Right)
            )
        }
    }
    pub fn menu(&self, menu: Menu) -> &'static str {
        use crate::pad::Button;
        if self.pad {
            match menu {
                Menu::Confirm => Button::South.label(),
                Menu::Back | Menu::QuitTitle => Button::East.label(),
                Menu::Pause => Button::Start.label(),
                Menu::Atlas | Menu::QuitPaused => Button::Select.label(),
                Menu::Options => Button::North.label(),
                Menu::NewRun | Menu::Abandon => Button::West.label(),
                Menu::Rows => "D-PAD",
                Menu::Adjust => "LEFT / RIGHT",
                Menu::Choice(i) => CHOICE_BUTTONS[i.min(2)].label(),
            }
        } else {
            match menu {
                Menu::Confirm => "ENTER",
                Menu::Back | Menu::Pause | Menu::QuitTitle => "ESC",
                Menu::Atlas => "TAB",
                Menu::Options => "O",
                Menu::NewRun => "N",
                Menu::Abandon => "X",
                Menu::QuitPaused => "Q",
                Menu::Rows => "W / S",
                Menu::Adjust => "A / D",
                Menu::Choice(i) => ["1", "2", "3"][i.min(2)],
            }
        }
    }
}
/// Numbered choices on a controller: left, top, and right face buttons, in
/// the same order as the cards on screen.
pub const CHOICE_BUTTONS: [crate::pad::Button; 3] = [
    crate::pad::Button::West,
    crate::pad::Button::North,
    crate::pad::Button::East,
];

/// Keyboard and mouse state for one frame, as closures so input mapping can be
/// tested without a window.
pub struct KeyState<'a> {
    pub down: &'a dyn Fn(KeyCode) -> bool,
    pub pressed: &'a dyn Fn(KeyCode) -> bool,
    pub mouse_strike: bool,
    pub mouse_parry: bool,
}
pub fn gather(b: &Bindings, s: &KeyState) -> Input {
    let down = |a: Action, extra: Option<KeyCode>| {
        b.keys(a).iter().any(|k| (s.down)(*k)) || extra.is_some_and(|k| (s.down)(k))
    };
    let pressed = |a: Action, extra: Option<KeyCode>| {
        b.keys(a).iter().any(|k| (s.pressed)(*k)) || extra.is_some_and(|k| (s.pressed)(k))
    };
    let axis = |a, k| if down(a, Some(k)) { 1. } else { 0. };
    Input {
        axis: axis(Action::Right, KeyCode::Right) - axis(Action::Left, KeyCode::Left),
        jump: pressed(Action::Jump, Some(KeyCode::Up)),
        jump_held: down(Action::Jump, Some(KeyCode::Up)),
        dodge: pressed(Action::Dodge, None),
        attack: down(Action::Strike, None) || s.mouse_strike,
        bow: down(Action::Glassbolt, None),
        parry: pressed(Action::Parry, None) || s.mouse_parry,
        grenade: pressed(Action::FireVessel, None),
        trap: pressed(Action::ArcSnare, None),
        heal: pressed(Action::Heal, None),
        interact: pressed(Action::Interact, None),
        down: down(Action::Down, Some(KeyCode::Down)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_with(b: &Bindings, held: &[KeyCode]) -> Input {
        let down = |k: KeyCode| held.contains(&k);
        gather(
            b,
            &KeyState {
                down: &down,
                pressed: &down,
                mouse_strike: false,
                mouse_parry: false,
            },
        )
    }

    #[test]
    fn defaults_match_the_original_layout() {
        let b = Bindings::default();
        let i = input_with(
            &b,
            &[KeyCode::D, KeyCode::W, KeyCode::J, KeyCode::RightShift],
        );
        assert_eq!(i.axis, 1.);
        assert!(i.jump && i.jump_held && i.attack && i.dodge);
        let i = input_with(&b, &[KeyCode::Left, KeyCode::Up, KeyCode::Down]);
        assert_eq!(i.axis, -1.);
        assert!(i.jump && i.down, "arrow keys are fixed alternatives");
        assert_eq!(b.short(Action::Dodge), "SHIFT");
        assert_eq!(b.label(Action::Jump), "SPACE / W");
    }

    #[test]
    fn prompts_follow_the_last_device() {
        let mut b = Bindings::default();
        b.bind(Action::Jump, KeyCode::K).unwrap();
        let keys = Prompts::new(&b, false);
        let pad = Prompts::new(&b, true);
        assert_eq!(keys.action(Action::Jump), "K");
        assert_eq!(pad.action(Action::Jump), "A");
        assert_eq!(keys.movement(), "A / D");
        assert_eq!(pad.movement(), "STICK");
        assert_eq!(keys.menu(Menu::Confirm), "ENTER");
        assert_eq!(pad.menu(Menu::Confirm), "A");
        assert_eq!(
            (0..3)
                .map(|i| pad.menu(Menu::Choice(i)))
                .collect::<Vec<_>>(),
            ["X", "Y", "B"]
        );
        assert_eq!(keys.menu(Menu::Choice(2)), "3");
    }

    #[test]
    fn binding_a_used_key_swaps_it_and_reserved_keys_refuse() {
        let mut b = Bindings::default();
        assert_eq!(b.bind(Action::Strike, KeyCode::U), Ok(None));
        let i = input_with(&b, &[KeyCode::U]);
        assert!(i.attack);
        assert!(!input_with(&b, &[KeyCode::J]).attack);
        // K belonged to the glassbolt; it takes Strike's previous key, U.
        assert_eq!(
            b.bind(Action::Strike, KeyCode::K),
            Ok(Some(Action::Glassbolt))
        );
        assert_eq!(b.keys(Action::Glassbolt), &[KeyCode::U]);
        assert_eq!(b.keys(Action::Strike), &[KeyCode::K]);
        // A two-key action just loses the key.
        assert_eq!(b.bind(Action::Heal, KeyCode::W), Ok(Some(Action::Jump)));
        assert_eq!(b.keys(Action::Jump), &[KeyCode::Space]);
        for reserved in [
            KeyCode::Escape,
            KeyCode::Enter,
            KeyCode::M,
            KeyCode::Up,
            KeyCode::F9,
        ] {
            assert_eq!(b.bind(Action::Jump, reserved), Err(()));
        }
        let all: Vec<_> = Action::ALL.iter().flat_map(|a| b.keys(*a)).collect();
        assert!(all.iter().enumerate().all(|(i, k)| !all[..i].contains(k)));
    }

    #[test]
    fn bindings_round_trip_and_repair_bad_files() {
        let mut b = Bindings::default();
        b.bind(Action::Parry, KeyCode::Semicolon).unwrap();
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("\"parry\":[\";\"]"));
        assert_eq!(serde_json::from_str::<Bindings>(&json).unwrap(), b);
        let partial: Bindings = serde_json::from_str(r#"{"heal":["H"],"jump":["NOPE"]}"#).unwrap();
        assert_eq!(partial.keys(Action::Heal), &[KeyCode::H]);
        assert_eq!(partial.keys(Action::Jump), Action::Jump.defaults());
        let clash: Bindings = serde_json::from_str(r#"{"heal":["J"]}"#).unwrap();
        assert_eq!(
            clash,
            Bindings::default(),
            "duplicate keys reset to defaults"
        );
    }
}
