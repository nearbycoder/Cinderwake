//! The browser build's on-screen touch controls, drawn by the page
//! (web/cinderwake-touch.js) over the canvas on phones and tablets. Their
//! buttons use the controller's fixed layout, so `pad` reads them together
//! with any controller and menus treat them alike. The page also says whether
//! they are shown (prompts then name them) and whether it is asking for the
//! device to be turned, and hears what is on screen, to show the play buttons
//! only in play. Other builds have no page: nothing is shown or pressed.
use crate::game::{Game, Input, Screen};
use macroquad::prelude::Vec2;

/// What the page reports each frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Page {
    /// The on-screen controls are shown: touch was the last thing used.
    pub shown: bool,
    /// The page covers the game, asking for the device to be turned.
    pub blocked: bool,
}

/// What the page shows over each screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Menus are tapped on the canvas; the play buttons hide.
    Menu = 0,
    Play = 1,
    /// Play with the vertical atlas open; its button closes it.
    Atlas = 2,
}
impl Mode {
    pub fn for_game(g: &Game) -> Self {
        match (g.screen, g.map) {
            (Screen::Playing, false) => Self::Play,
            (Screen::Playing, true) => Self::Atlas,
            _ => Self::Menu,
        }
    }
}

/// The actions in one frame's input, one bit each, for the page's record of
/// what reached the game (its tests read it).
pub fn input_bits(i: &Input) -> u32 {
    [
        i.axis != 0.,
        i.jump,
        i.attack,
        i.dodge,
        i.parry,
        i.bow,
        i.grenade,
        i.trap,
        i.heal,
        i.interact,
        i.down,
    ]
    .iter()
    .enumerate()
    .fold(0, |bits, (n, on)| bits | (u32::from(*on) << n))
}

#[cfg(target_arch = "wasm32")]
mod page {
    extern "C" {
        fn cinderwake_touch_buttons() -> u32;
        fn cinderwake_touch_axis(index: u32) -> f32;
        fn cinderwake_touch_state() -> u32;
        fn cinderwake_touch_frame(mode: u32, input: u32, x: f32, y: f32);
    }
    /// Lets the JS plugin confirm that it matches this build.
    #[no_mangle]
    pub extern "C" fn cinderwake_touch_crate_version() -> u32 {
        1
    }
    // SAFETY (all four): plain value calls into web/cinderwake-touch.js.
    pub fn buttons() -> u32 {
        unsafe { cinderwake_touch_buttons() }
    }
    pub fn axis(index: u32) -> f32 {
        unsafe { cinderwake_touch_axis(index) }
    }
    pub fn state() -> u32 {
        unsafe { cinderwake_touch_state() }
    }
    pub fn frame(mode: u32, input: u32, x: f32, y: f32) {
        unsafe { cinderwake_touch_frame(mode, input, x, y) }
    }
}
#[cfg(not(target_arch = "wasm32"))]
mod page {
    pub fn state() -> u32 {
        0
    }
    pub fn frame(_: u32, _: u32, _: f32, _: f32) {}
}

/// The touch buttons held this frame, as Gamepad API "standard" button bits.
#[cfg(target_arch = "wasm32")]
pub fn buttons() -> u32 {
    page::buttons()
}
/// The on-screen stick; x grows to the right and y downward, within -1..=1.
#[cfg(target_arch = "wasm32")]
pub fn stick() -> Vec2 {
    Vec2::new(page::axis(0), page::axis(1))
}

pub fn page() -> Page {
    let state = page::state();
    Page {
        shown: state & 1 != 0,
        blocked: state & 2 != 0,
    }
}

/// Tells the page what is on screen and what the game made of this frame's
/// input, and where the wanderer stands.
pub fn report(mode: Mode, input: &Input, at: Vec2) {
    page::frame(mode as u32, input_bits(input), at.x, at.y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_action_has_its_own_bit() {
        assert_eq!(input_bits(&Input::default()), 0);
        let all = Input {
            axis: -1.,
            jump: true,
            jump_held: true,
            dodge: true,
            attack: true,
            bow: true,
            parry: true,
            grenade: true,
            trap: true,
            heal: true,
            interact: true,
            down: true,
        };
        assert_eq!(input_bits(&all), 0b111_1111_1111);
        let jump = Input {
            jump: true,
            ..Input::default()
        };
        assert_eq!(input_bits(&jump), 0b10);
    }

    #[test]
    fn play_buttons_show_only_in_play() {
        let mut g = Game::new(4017, crate::save::Save::default());
        assert_eq!(Mode::for_game(&g), Mode::Menu, "the title");
        g.start();
        assert_eq!(Mode::for_game(&g), Mode::Play);
        g.map = true;
        assert_eq!(Mode::for_game(&g), Mode::Atlas);
        g.map = false;
        g.focus_lost();
        assert_eq!(Mode::for_game(&g), Mode::Menu, "paused");
    }

    #[test]
    fn other_builds_have_no_page() {
        assert_eq!(page(), Page::default());
    }
}
