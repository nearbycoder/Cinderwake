//! Gamepads, with one fixed layout in the Xbox naming most pads use. The
//! desktop reads controllers through `gilrs`; the browser build reads the
//! Gamepad API through `web/cinderwake-pad.js`. Both feed the same `State`, so
//! the mapping below is shared and testable without a controller.
use crate::controls::Action;
use crate::game::Input;
use macroquad::prelude::{vec2, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    /// A on Xbox pads, the bottom face button.
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    /// View, Back, or Select.
    Select,
    Start,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}
const BUTTONS: usize = 14;
impl Button {
    pub const ALL: [Button; BUTTONS] = [
        Self::South,
        Self::East,
        Self::West,
        Self::North,
        Self::LeftBumper,
        Self::RightBumper,
        Self::LeftTrigger,
        Self::RightTrigger,
        Self::Select,
        Self::Start,
        Self::DpadUp,
        Self::DpadDown,
        Self::DpadLeft,
        Self::DpadRight,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::South => "A",
            Self::East => "B",
            Self::West => "X",
            Self::North => "Y",
            Self::LeftBumper => "LB",
            Self::RightBumper => "RB",
            Self::LeftTrigger => "LT",
            Self::RightTrigger => "RT",
            Self::Select => "VIEW",
            Self::Start => "START",
            Self::DpadUp => "D-UP",
            Self::DpadDown => "DOWN",
            Self::DpadLeft => "LEFT",
            Self::DpadRight => "RIGHT",
        }
    }
    /// The Gamepad API's "standard" layout numbers its buttons in this order.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    fn standard_index(self) -> u32 {
        match self {
            Self::South => 0,
            Self::East => 1,
            Self::West => 2,
            Self::North => 3,
            Self::LeftBumper => 4,
            Self::RightBumper => 5,
            Self::LeftTrigger => 6,
            Self::RightTrigger => 7,
            Self::Select => 8,
            Self::Start => 9,
            Self::DpadUp => 12,
            Self::DpadDown => 13,
            Self::DpadLeft => 14,
            Self::DpadRight => 15,
        }
    }
    fn slot(self) -> usize {
        self as usize
    }
}

/// The fixed gameplay layout.
pub fn button(action: Action) -> Button {
    match action {
        Action::Left => Button::DpadLeft,
        Action::Right => Button::DpadRight,
        Action::Jump => Button::South,
        Action::Down => Button::DpadDown,
        Action::Strike => Button::West,
        Action::Glassbolt => Button::RightTrigger,
        Action::Dodge => Button::East,
        Action::Parry => Button::RightBumper,
        Action::FireVessel => Button::LeftBumper,
        Action::ArcSnare => Button::LeftTrigger,
        Action::Heal => Button::DpadUp,
        Action::Interact => Button::North,
    }
}

/// Short name for prompts. Movement reads as the stick, which also works.
pub fn label(action: Action) -> &'static str {
    match action {
        Action::Left | Action::Right => "STICK",
        _ => button(action).label(),
    }
}

/// Menu directions: the D-pad or the left stick pushed most of the way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// One frame of every connected controller, combined.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub buttons: [bool; BUTTONS],
    /// Left stick; x grows to the right and y downward, each within -1..=1.
    pub stick: Vec2,
}
impl State {
    #[cfg(test)]
    pub fn with(buttons: &[Button], stick: Vec2) -> Self {
        let mut state = Self {
            stick,
            ..Self::default()
        };
        for b in buttons {
            state.buttons[b.slot()] = true;
        }
        state
    }
    fn down(&self, b: Button) -> bool {
        self.buttons[b.slot()]
    }
    /// Stick direction for menus and dropping: pushed past `PUSH` and closer
    /// to that direction than to its neighbours, so running diagonally doesn't
    /// count as down.
    fn stick_toward(&self, d: Dir) -> bool {
        let Vec2 { x, y } = self.stick;
        match d {
            Dir::Up => -y > PUSH && -y > x.abs(),
            Dir::Down => y > PUSH && y > x.abs(),
            Dir::Left => -x > PUSH && -x > y.abs(),
            Dir::Right => x > PUSH && x > y.abs(),
        }
    }
    fn toward(&self, d: Dir) -> bool {
        self.stick_toward(d)
            || self.down(match d {
                Dir::Up => Button::DpadUp,
                Dir::Down => Button::DpadDown,
                Dir::Left => Button::DpadLeft,
                Dir::Right => Button::DpadRight,
            })
    }
}
/// Stick travel ignored around the centre, where worn sticks drift.
pub const DEAD_ZONE: f32 = 0.24;
const PUSH: f32 = 0.6;

/// Walking speed from the stick: nothing inside the dead zone, then rising
/// to a full run at 0.7, which a stick pushed diagonally still reaches.
pub fn walk(x: f32) -> f32 {
    let travel = ((x.abs() - DEAD_ZONE) / (0.7 - DEAD_ZONE)).clamp(0., 1.);
    travel * x.signum()
}

pub struct Pad {
    now: State,
    before: State,
    /// Buttons still held from a menu; they act again only after a release.
    held_over: [bool; BUTTONS],
    /// Controllers connected at the last poll, and how that count changed.
    connected: usize,
    change: Connection,
    backend: Backend,
}
/// How the number of connected controllers changed at the last poll.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Same,
    Lost,
    Found,
}
impl Pad {
    pub fn new() -> Self {
        Self {
            now: State::default(),
            before: State::default(),
            held_over: [false; BUTTONS],
            connected: 0,
            change: Connection::Same,
            backend: Backend::new(),
        }
    }
    /// Reads every controller once per frame.
    pub fn poll(&mut self) {
        let state = self.backend.read();
        let count = self.backend.count();
        self.count(count);
        self.feed(state);
    }
    /// Records how many controllers are connected now.
    pub fn count(&mut self, connected: usize) {
        self.change = match connected.cmp(&self.connected) {
            std::cmp::Ordering::Less => Connection::Lost,
            std::cmp::Ordering::Greater => Connection::Found,
            std::cmp::Ordering::Equal => Connection::Same,
        };
        self.connected = connected;
    }
    pub fn connection(&self) -> Connection {
        self.change
    }
    pub fn feed(&mut self, state: State) {
        self.before = self.now;
        self.now = state;
        for (held, down) in self.held_over.iter_mut().zip(state.buttons) {
            *held &= down;
        }
    }
    pub fn pressed(&self, b: Button) -> bool {
        self.now.down(b) && !self.before.down(b)
    }
    /// A menu direction held this frame, however long ago it started.
    pub fn holding(&self, d: Dir) -> bool {
        self.now.toward(d)
    }
    /// A menu direction that started this frame.
    pub fn nav(&self, d: Dir) -> bool {
        self.now.toward(d) && !self.before.toward(d)
    }
    /// Any button press or stick push this frame, for choosing which prompts
    /// to show.
    pub fn touched(&self) -> bool {
        Button::ALL.iter().any(|b| self.pressed(*b))
            || [Dir::Up, Dir::Down, Dir::Left, Dir::Right]
                .iter()
                .any(|d| self.now.stick_toward(*d) && !self.before.stick_toward(*d))
    }
    /// Called while a menu is open: whatever is held there must be released
    /// before it acts in play, so leaving a menu with **A** doesn't also jump.
    pub fn hold_over(&mut self) {
        self.held_over = self.now.buttons;
    }
    pub fn gameplay(&self) -> Input {
        let live = |b: Button| self.now.down(b) && !self.held_over[b.slot()];
        let pressed = |b: Button| live(b) && !self.before.down(b);
        let act_down = |a: Action| live(button(a));
        let act_pressed = |a: Action| pressed(button(a));
        let axis = if act_down(Action::Right) != act_down(Action::Left) {
            if act_down(Action::Right) {
                1.
            } else {
                -1.
            }
        } else {
            walk(self.now.stick.x)
        };
        Input {
            axis,
            jump: act_pressed(Action::Jump),
            jump_held: act_down(Action::Jump),
            dodge: act_pressed(Action::Dodge),
            attack: act_down(Action::Strike),
            bow: act_down(Action::Glassbolt),
            parry: act_pressed(Action::Parry),
            grenade: act_pressed(Action::FireVessel),
            trap: act_pressed(Action::ArcSnare),
            heal: act_pressed(Action::Heal),
            interact: act_pressed(Action::Interact),
            down: act_down(Action::Down) || self.now.stick_toward(Dir::Down),
        }
    }
}

/// Keyboard and controller together: either can act, and the keyboard's
/// direction wins if both are steering.
pub fn combine(keys: Input, pad: Input) -> Input {
    Input {
        axis: if keys.axis != 0. { keys.axis } else { pad.axis },
        jump: keys.jump || pad.jump,
        jump_held: keys.jump_held || pad.jump_held,
        dodge: keys.dodge || pad.dodge,
        attack: keys.attack || pad.attack,
        bow: keys.bow || pad.bow,
        parry: keys.parry || pad.parry,
        grenade: keys.grenade || pad.grenade,
        trap: keys.trap || pad.trap,
        heal: keys.heal || pad.heal,
        interact: keys.interact || pad.interact,
        down: keys.down || pad.down,
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
struct Backend(Option<gilrs::Gilrs>);
#[cfg(all(not(target_arch = "wasm32"), not(test)))]
impl Backend {
    fn new() -> Self {
        // Without controller support (for example, no udev) the game simply
        // runs on the keyboard.
        Self(match gilrs::Gilrs::new() {
            Ok(g) => Some(g),
            Err(gilrs::Error::NotImplemented(g)) => Some(g),
            Err(e) => {
                eprintln!("Gamepads unavailable: {e}");
                None
            }
        })
    }
    fn read(&mut self) -> State {
        use gilrs::{Axis, Button as G};
        let Some(gilrs) = &mut self.0 else {
            return State::default();
        };
        // Draining events keeps each controller's cached state current and
        // notices controllers being connected or removed.
        while gilrs.next_event().is_some() {}
        let mut state = State::default();
        for (_, pad) in gilrs.gamepads() {
            for b in Button::ALL {
                let native = match b {
                    Button::South => G::South,
                    Button::East => G::East,
                    Button::West => G::West,
                    Button::North => G::North,
                    Button::LeftBumper => G::LeftTrigger,
                    Button::RightBumper => G::RightTrigger,
                    Button::LeftTrigger => G::LeftTrigger2,
                    Button::RightTrigger => G::RightTrigger2,
                    Button::Select => G::Select,
                    Button::Start => G::Start,
                    Button::DpadUp => G::DPadUp,
                    Button::DpadDown => G::DPadDown,
                    Button::DpadLeft => G::DPadLeft,
                    Button::DpadRight => G::DPadRight,
                };
                state.buttons[b.slot()] |= pad.is_pressed(native);
            }
            // gilrs reports the stick with y growing upward.
            let stick = vec2(pad.value(Axis::LeftStickX), -pad.value(Axis::LeftStickY));
            if stick.length() > state.stick.length() {
                state.stick = stick;
            }
        }
        state
    }
    /// Connected controllers, as of the events drained by `read`.
    fn count(&self) -> usize {
        self.0.as_ref().map_or(0, |gilrs| gilrs.gamepads().count())
    }
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn cinderwake_pad_buttons() -> u32;
    fn cinderwake_pad_axis(index: u32) -> f32;
    fn cinderwake_pad_count() -> u32;
}
/// Lets the JS plugin confirm that it matches this build. Without it,
/// Miniquad's loader logs that the plugin isn't used.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn cinderwake_pad_crate_version() -> u32 {
    1
}
#[cfg(target_arch = "wasm32")]
struct Backend;
#[cfg(target_arch = "wasm32")]
impl Backend {
    fn new() -> Self {
        Self
    }
    fn read(&mut self) -> State {
        // SAFETY: plain value calls into web/cinderwake-pad.js.
        let (bits, x, y) = unsafe {
            (
                cinderwake_pad_buttons(),
                cinderwake_pad_axis(0),
                cinderwake_pad_axis(1),
            )
        };
        let mut state = State {
            stick: vec2(x, y),
            ..State::default()
        };
        for b in Button::ALL {
            state.buttons[b.slot()] = bits & (1 << b.standard_index()) != 0;
        }
        state
    }
    fn count(&self) -> usize {
        // SAFETY: a plain value call into web/cinderwake-pad.js.
        unsafe { cinderwake_pad_count() as usize }
    }
}

/// Tests never open real devices; they feed states directly.
#[cfg(all(test, not(target_arch = "wasm32")))]
struct Backend;
#[cfg(all(test, not(target_arch = "wasm32")))]
impl Backend {
    fn new() -> Self {
        Self
    }
    fn read(&mut self) -> State {
        State::default()
    }
    fn count(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pad: &mut Pad, buttons: &[Button], stick: Vec2) -> Input {
        pad.feed(State::with(buttons, stick));
        pad.gameplay()
    }

    #[test]
    fn connection_changes_are_reported_for_one_poll() {
        let mut pad = Pad::new();
        for (count, change) in [
            (1, Connection::Found),
            (1, Connection::Same),
            (2, Connection::Found),
            (1, Connection::Lost),
            (1, Connection::Same),
            (0, Connection::Lost),
        ] {
            pad.count(count);
            assert_eq!(pad.connection(), change, "{count}");
        }
    }

    #[test]
    fn the_layout_reaches_every_action() {
        let mut pad = Pad::new();
        let i = frame(
            &mut pad,
            &[
                Button::South,
                Button::West,
                Button::East,
                Button::North,
                Button::RightBumper,
                Button::RightTrigger,
                Button::LeftBumper,
                Button::LeftTrigger,
                Button::DpadUp,
                Button::DpadDown,
                Button::DpadRight,
            ],
            Vec2::ZERO,
        );
        assert_eq!(i.axis, 1.);
        assert!(i.jump && i.jump_held && i.attack && i.dodge && i.interact);
        assert!(i.parry && i.bow && i.grenade && i.trap && i.heal && i.down);
        // Held on the next frame: only the held actions continue.
        let i = frame(
            &mut pad,
            &[Button::South, Button::West, Button::RightTrigger],
            Vec2::ZERO,
        );
        assert!(i.jump_held && i.attack && i.bow);
        assert!(!i.jump && !i.dodge && !i.parry && !i.heal && !i.interact);
        assert_eq!(i.axis, 0.);
        for action in crate::controls::Action::ALL {
            assert!(!label(action).is_empty());
        }
        assert_eq!(label(Action::Jump), "A");
        assert_eq!(label(Action::Glassbolt), "RT");
    }

    #[test]
    fn the_stick_has_a_dead_zone_and_a_full_run() {
        assert_eq!(walk(0.2), 0.);
        assert_eq!(walk(-DEAD_ZONE), 0.);
        assert!(walk(0.5) > 0.3 && walk(0.5) < 0.7);
        assert_eq!(walk(0.71), 1., "a diagonal push still runs");
        assert_eq!(walk(0.95), 1.);
        assert_eq!(walk(-1.), -1.);
        let mut pad = Pad::new();
        assert_eq!(frame(&mut pad, &[], vec2(0.15, 0.1)).axis, 0.);
        assert_eq!(frame(&mut pad, &[], vec2(-1., 0.)).axis, -1.);
        // Running diagonally down-right isn't a drop or a slam; pushing down is.
        let i = frame(&mut pad, &[], vec2(0.8, 0.65));
        assert!(i.axis > 0.9 && !i.down);
        assert!(frame(&mut pad, &[], vec2(0.3, 0.9)).down);
        // The D-pad overrides the stick, as a full push.
        assert_eq!(
            frame(&mut pad, &[Button::DpadLeft], vec2(0.5, 0.)).axis,
            -1.
        );
    }

    #[test]
    fn buttons_held_from_a_menu_wait_for_a_release() {
        let mut pad = Pad::new();
        pad.feed(State::with(&[Button::South, Button::West], Vec2::ZERO));
        assert!(pad.pressed(Button::South));
        pad.hold_over();
        let i = pad.gameplay();
        assert!(!i.jump && !i.jump_held && !i.attack);
        let i = frame(&mut pad, &[Button::South, Button::West], vec2(1., 0.));
        assert!(!i.jump_held && !i.attack, "still held from the menu");
        assert_eq!(i.axis, 1., "the stick isn't held over");
        let i = frame(&mut pad, &[Button::West], Vec2::ZERO);
        assert!(!i.attack);
        frame(&mut pad, &[], Vec2::ZERO);
        let i = frame(&mut pad, &[Button::South, Button::West], Vec2::ZERO);
        assert!(i.jump && i.attack, "pressed again after the release");
    }

    #[test]
    fn menus_see_presses_and_stick_pushes_once() {
        let mut pad = Pad::new();
        pad.feed(State::with(&[], vec2(0., -0.9)));
        assert!(pad.nav(Dir::Up) && pad.touched());
        pad.feed(State::with(&[], vec2(0., -0.95)));
        assert!(!pad.nav(Dir::Up) && !pad.touched(), "held, not repeated");
        pad.feed(State::with(&[Button::DpadUp], vec2(0., -0.95)));
        assert!(!pad.nav(Dir::Up), "already pointing up");
        pad.feed(State::with(&[], vec2(0.1, 0.)));
        assert!(!pad.touched(), "drifting inside the threshold");
        pad.feed(State::with(&[Button::Start], Vec2::ZERO));
        assert!(pad.pressed(Button::Start) && pad.touched());
    }

    #[test]
    fn keyboard_and_pad_combine() {
        let keys = Input {
            axis: -1.,
            attack: true,
            ..Input::default()
        };
        let pad = Input {
            axis: 0.7,
            jump: true,
            ..Input::default()
        };
        let both = combine(keys, pad);
        assert_eq!(both.axis, -1.);
        assert!(both.attack && both.jump);
        assert_eq!(combine(Input::default(), pad).axis, 0.7);
    }
}
