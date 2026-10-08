mod animation;
mod art;
mod atlas;
mod audio;
mod bench;
mod blend;
mod controls;
mod environment;
mod fidelity;
mod game;
mod icon;
mod launch;
mod pad;
mod particles;
mod postprocess;
mod render;
mod save;
mod scenery;
mod settings;
mod storage;
mod traversal_capture;
mod ui;
mod ui_fade;
mod ui_skin;
mod world;
use game::*;
use macroquad::prelude::*;
/// Launch flags that keep the run apart from saved progress and settings.
const ISOLATING_FLAGS: [&str; 11] = [
    "--capture",
    "--gallery",
    "--sprite-preview",
    "--ui-gallery",
    "--environment-tour",
    "--motion-capture",
    "--vertical-capture",
    "--demo",
    "--start-at",
    "--pacing-check",
    "--fidelity-bench",
];
fn isolated(args: &[String]) -> bool {
    args.iter().any(|a| ISOLATING_FLAGS.contains(&a.as_str()))
}
/// Whether this build remembers the window's size. Miniquad measures the
/// window in pixels on Linux (X11 and XWayland); macOS and Windows measure
/// it differently and haven't been checked, so they keep the default.
const REMEMBER_WINDOW: bool = cfg!(target_os = "linux");
/// The window size saved in the settings, read before the window opens. A
/// damaged file is left for the game's usual check once it starts.
fn saved_window() -> Option<[u32; 2]> {
    if !REMEMBER_WINDOW || isolated(&std::env::args().collect::<Vec<_>>()) {
        return None;
    }
    let bytes = storage::read(settings::Settings::FILE)?;
    serde_json::from_slice::<settings::Settings>(&bytes)
        .ok()?
        .window
        .filter(|size| settings::Settings::window_fits(*size))
}
/// Seconds a new window size must hold before it's saved, so dragging a
/// window's edge writes once rather than every frame.
const WINDOW_SETTLE: f32 = 1.;
/// Watches the window's size so it can be saved once it settles.
#[derive(Default)]
struct WindowWatch {
    size: Option<[u32; 2]>,
    still: f32,
}
impl WindowWatch {
    /// `size` is the window in pixels. Returns a size to save when it has
    /// held for `WINDOW_SETTLE` seconds and differs from `saved`. Fullscreen
    /// sizes are never saved, and leaving fullscreen starts the wait again.
    fn watch(
        &mut self,
        size: [u32; 2],
        fullscreen: bool,
        dt: f32,
        saved: Option<[u32; 2]>,
    ) -> Option<[u32; 2]> {
        if fullscreen || !settings::Settings::window_fits(size) {
            self.size = None;
            return None;
        }
        if self.size != Some(size) {
            self.size = Some(size);
            self.still = 0.;
            return None;
        }
        let before = self.still;
        self.still += dt;
        let settled = before < WINDOW_SETTLE && self.still >= WINDOW_SETTLE;
        (settled && saved.unwrap_or(settings::Settings::DEFAULT_WINDOW) != size).then_some(size)
    }
}
/// Testing aid: `--window-size 1920x1080` opens a capture or benchmark
/// window at that size instead of 1280 × 720.
fn window_size_flag(args: &[String]) -> Option<[u32; 2]> {
    let value = args.get(args.iter().position(|a| a == "--window-size")? + 1)?;
    let (w, h) = value.split_once('x')?;
    let size = [w.parse().ok()?, h.parse().ok()?];
    (isolated(args) && settings::Settings::window_fits(size)).then_some(size)
}
fn conf() -> Conf {
    let args: Vec<String> = std::env::args().collect();
    let [window_width, window_height] = window_size_flag(&args)
        .or_else(saved_window)
        .unwrap_or(settings::Settings::DEFAULT_WINDOW);
    // The fidelity benchmark measures frames, not the display's refresh.
    let bench = args.iter().any(|a| a == "--fidelity-bench");
    Conf {
        window_title: "Cinderwake — A Clockwork Roguelite".into(),
        window_width: window_width as i32,
        window_height: window_height as i32,
        high_dpi: true,
        window_resizable: true,
        // Browsers ignore the icon, so the build there skips decoding it.
        icon: if cfg!(target_arch = "wasm32") {
            None
        } else {
            icon::window_icon()
        },
        platform: miniquad::conf::Platform {
            linux_wm_class: "cinderwake",
            swap_interval: bench.then_some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}
/// `mouse_strike` is false while a click that chose something in a menu is
/// still held, so it doesn't also strike once play resumes.
fn read_input(g: &Game, mouse_strike: bool) -> Input {
    controls::gather(
        &g.settings.keys,
        &controls::KeyState {
            down: &|k| is_key_down(k),
            pressed: &|k| is_key_pressed(k),
            mouse_strike: mouse_strike && is_mouse_button_down(MouseButton::Left),
            mouse_parry: is_mouse_button_pressed(MouseButton::Right),
        },
    )
}
/// Watches the window's focus. Macroquad reports losing focus (X11 and
/// XWayland FocusOut, a hidden or unfocused browser tab) as minimising.
#[derive(Default)]
struct FocusWatch {
    lost: bool,
    regained: bool,
}
impl miniquad::EventHandler for FocusWatch {
    fn update(&mut self) {}
    fn draw(&mut self) {}
    fn window_minimized_event(&mut self) {
        self.lost = true;
        self.regained = false;
    }
    fn window_restored_event(&mut self) {
        self.regained = true;
    }
}

/// Menu keys pressed this frame, passed in so menus can be tested without a window.
struct MenuKeys<'a> {
    pressed: &'a dyn Fn(KeyCode) -> bool,
    last: Option<KeyCode>,
    /// A left click this frame, in interface coordinates.
    click: Option<Vec2>,
    /// Where the mouse moved to this frame, if it moved.
    hover: Option<Vec2>,
    /// A held direction repeating this frame (`MenuRepeat`).
    repeat: Option<pad::Dir>,
}
/// Seconds a menu direction is held before it repeats, and between repeats.
const REPEAT_DELAY: f32 = 0.4;
const REPEAT_EVERY: f32 = 0.1;
/// Fixed menu keys for each direction.
fn menu_keys(d: pad::Dir) -> [KeyCode; 2] {
    match d {
        pad::Dir::Up => [KeyCode::W, KeyCode::Up],
        pad::Dir::Down => [KeyCode::S, KeyCode::Down],
        pad::Dir::Left => [KeyCode::A, KeyCode::Left],
        pad::Dir::Right => [KeyCode::D, KeyCode::Right],
    }
}
/// Holding a direction on the options and controls pages repeats it, so a
/// bar or a long list doesn't take a press per step. The press itself is
/// handled as before; this only adds the repeats that follow.
#[derive(Default)]
struct MenuRepeat {
    held: Option<pad::Dir>,
    time: f32,
}
impl MenuRepeat {
    /// Only the options and controls pages repeat, and not while a key is
    /// being listened for.
    fn applies(g: &Game) -> bool {
        matches!(g.screen, Screen::Options | Screen::Controls) && !g.rebinding
    }
    /// The direction to follow this frame: the one already held, or else
    /// the first held, given whether each is down.
    fn choose(&self, down: impl Fn(pad::Dir) -> bool) -> Option<pad::Dir> {
        use pad::Dir;
        self.held.filter(|d| down(*d)).or_else(|| {
            [Dir::Up, Dir::Down, Dir::Left, Dir::Right]
                .into_iter()
                .find(|d| down(*d))
        })
    }
    /// Given the direction held this frame and the real seconds since the
    /// last, returns it on each frame where it repeats.
    fn update(&mut self, held: Option<pad::Dir>, dt: f32) -> Option<pad::Dir> {
        if held != self.held {
            // A new direction (or none): its press acts on its own.
            *self = Self { held, time: 0. };
            return None;
        }
        let repeats = |t: f32| {
            if t < REPEAT_DELAY {
                0
            } else {
                1 + ((t - REPEAT_DELAY) / REPEAT_EVERY) as u32
            }
        };
        let before = repeats(self.time);
        self.time += dt;
        held.filter(|_| repeats(self.time) > before)
    }
}
/// A window point in interface coordinates (1280 × 720), given the
/// letterboxed frame in the same logical units as the mouse.
fn to_interface(frame: Rect, at: Vec2) -> Vec2 {
    (at - frame.point()) * 1280. / frame.w
}
fn menus(g: &mut Game, keys: &MenuKeys, pad: &pad::Pad) {
    use pad::{Button, Dir};
    if (keys.pressed)(KeyCode::M) {
        g.settings.muted = !g.settings.muted;
        g.persist_settings();
    }
    if g.screen == Screen::Options {
        options_menu(g, keys, pad);
        return;
    }
    if g.screen == Screen::Controls {
        controls_menu(g, keys, pad);
        return;
    }
    let clicked = keys.click.and_then(|at| ui::click_at(g, at));
    if clicked == Some(ui::Click::Mute) {
        g.settings.muted = !g.settings.muted;
        g.persist_settings();
    }
    if ((keys.pressed)(KeyCode::Tab) || pad.pressed(Button::Select)) && g.screen == Screen::Playing
    {
        g.map = !g.map;
    }
    if matches!(g.screen, Screen::Title | Screen::Paused)
        && ((keys.pressed)(KeyCode::O)
            || pad.pressed(Button::North)
            || clicked == Some(ui::Click::Options))
    {
        g.open_options();
        return;
    }
    if g.screen == Screen::Paused
        && ((keys.pressed)(KeyCode::X)
            || pad.pressed(Button::West)
            || clicked == Some(ui::Click::Abandon))
    {
        g.request_abandon();
        return;
    }
    // Browsers close the tab instead, and show no quit link.
    let quit = clicked == Some(ui::Click::Quit)
        || match g.screen {
            Screen::Title => (keys.pressed)(KeyCode::Escape) || pad.pressed(Button::East),
            Screen::Paused => (keys.pressed)(KeyCode::Q) || pad.pressed(Button::Select),
            _ => false,
        };
    if quit && cfg!(not(target_arch = "wasm32")) {
        g.request_quit();
        return;
    }
    let pause = (keys.pressed)(KeyCode::Escape) || pad.pressed(Button::Start);
    if pause
        || (g.screen == Screen::Paused && pad.pressed(Button::East))
        || clicked == Some(ui::Click::Resume)
    {
        g.abandon_armed = false;
        g.screen = if g.screen == Screen::Playing {
            Screen::Paused
        } else if g.screen == Screen::Paused {
            Screen::Playing
        } else {
            g.screen
        };
    }
    let confirm = (keys.pressed)(KeyCode::Enter)
        || pad.pressed(Button::South)
        || clicked == Some(ui::Click::Confirm);
    // A numbered choice: 1 / 2 / 3, X / Y / B on a controller, or a click.
    let choice = |i: usize| {
        (keys.pressed)([KeyCode::Key1, KeyCode::Key2, KeyCode::Key3][i])
            || pad.pressed(controls::CHOICE_BUTTONS[i])
            || clicked == Some(ui::Click::Choice(i))
    };
    match g.screen {
        Screen::Title if g.resume.is_some() => {
            if confirm || pad.pressed(Button::Start) {
                g.continue_run();
            } else if (keys.pressed)(KeyCode::N)
                || pad.pressed(Button::West)
                || clicked == Some(ui::Click::NewRun)
            {
                g.start();
            }
        }
        Screen::Title => {
            if confirm || pad.pressed(Button::Start) {
                g.start();
            }
        }
        Screen::Dead | Screen::Victory => {
            if g.result_ready() && (confirm || pad.pressed(Button::Start)) {
                g.start();
            }
        }
        Screen::Reliquary => {
            if choice(0) {
                g.choose_weapon(true);
            } else if choice(1) {
                g.choose_weapon(false);
            }
        }
        Screen::Scroll => {
            if let Some(i) = (0..3).find(|i| choice(*i)) {
                g.upgrade(i);
            }
        }
        Screen::Camp => {
            if let Some(i) = (0..3).find(|i| choice(*i)) {
                g.buy(i);
            }
            let route = g.route;
            if (keys.pressed)(KeyCode::A) || (keys.pressed)(KeyCode::Left) || pad.nav(Dir::Left) {
                g.route = 0;
            }
            if (keys.pressed)(KeyCode::D) || (keys.pressed)(KeyCode::Right) || pad.nav(Dir::Right) {
                g.route = 1;
            }
            if clicked == Some(ui::Click::Route) {
                g.route = 1 - g.route;
            }
            if g.route != route {
                g.sounds.push(Sfx::Select);
            }
            if confirm || pad.pressed(Button::Start) {
                g.travel();
            }
        }
        _ => {}
    }
}

fn options_menu(g: &mut Game, keys: &MenuKeys, pad: &pad::Pad) {
    use pad::{Button, Dir};
    use settings::RowValue;
    use ui::Click;
    let pressed = |list: &[KeyCode]| list.iter().any(|k| (keys.pressed)(*k));
    let clicked = keys.click.and_then(|at| ui::click_at(g, at));
    if pressed(&[KeyCode::Escape, KeyCode::O])
        || pad.pressed(Button::East)
        || clicked == Some(Click::Back)
    {
        g.close_options();
        return;
    }
    // Pointing at a row selects it, so its help line shows. Only a moving
    // mouse does this, so a still cursor never fights the keyboard.
    if let Some(Click::Row(row) | Click::Level(row, _) | Click::Step(row, _)) =
        keys.hover.and_then(|at| ui::click_at(g, at))
    {
        g.options_row = row;
    }
    let before = g.settings.clone();
    match clicked {
        Some(Click::Row(row)) => {
            g.options_row = row;
            match g.settings.row(row).1 {
                RowValue::Page => {
                    g.open_controls();
                    return;
                }
                RowValue::Switch(_) => g.settings.adjust(row, 1),
                // A bar's name only selects it.
                RowValue::Level(_) | RowValue::Steps(..) => {}
            }
        }
        Some(Click::Level(row, level)) => {
            g.options_row = row;
            g.settings.set_level(row, level);
        }
        Some(Click::Step(row, delta)) => g.settings.adjust(row, delta as i32),
        _ => {}
    }
    if g.settings != before {
        g.sounds.push(Sfx::Select);
    }
    let rows = settings::Settings::ROWS;
    if pressed(&[KeyCode::W, KeyCode::Up]) || pad.nav(Dir::Up) {
        g.options_row = (g.options_row + rows - 1) % rows;
    }
    if pressed(&[KeyCode::S, KeyCode::Down]) || pad.nav(Dir::Down) {
        g.options_row = (g.options_row + 1) % rows;
    }
    // Repeats stop at the ends instead of wrapping, and step only bars.
    match keys.repeat {
        Some(Dir::Up) => g.options_row = g.options_row.saturating_sub(1),
        Some(Dir::Down) => g.options_row = (g.options_row + 1).min(rows - 1),
        Some(d @ (Dir::Left | Dir::Right)) if g.settings.row(g.options_row).1.is_bar() => {
            let before = g.settings.clone();
            g.settings
                .adjust(g.options_row, if d == Dir::Left { -1 } else { 1 });
            if g.settings != before {
                g.sounds.push(Sfx::Select);
            }
        }
        _ => {}
    }
    let forward = pressed(&[KeyCode::D, KeyCode::Right, KeyCode::Enter, KeyCode::Space])
        || pad.nav(Dir::Right)
        || pad.pressed(Button::South);
    if g.options_row == settings::Settings::CONTROLS_ROW && forward {
        g.open_controls();
        return;
    }
    let delta = if pressed(&[KeyCode::A, KeyCode::Left]) || pad.nav(Dir::Left) {
        -1
    } else if forward {
        1
    } else {
        0
    };
    if delta != 0 {
        g.settings.adjust(g.options_row, delta);
        // Also lets the player hear the effects volume they just chose.
        g.sounds.push(Sfx::Select);
    }
}

fn controls_menu(g: &mut Game, keys: &MenuKeys, pad: &pad::Pad) {
    use pad::{Button, Dir};
    let pressed = |list: &[KeyCode]| list.iter().any(|k| (keys.pressed)(*k));
    let clicked = keys.click.and_then(|at| ui::click_at(g, at));
    if g.rebinding {
        // Checked before Enter below, so the press that began listening is
        // never captured as the new key. Mouse buttons can't be bound, so a
        // click cancels too.
        if pressed(&[KeyCode::Escape]) || pad.pressed(Button::East) || keys.click.is_some() {
            g.rebinding = false;
            g.controls_note = None;
        } else if let Some(key) = keys.last {
            g.rebind(key);
        }
        return;
    }
    if pressed(&[KeyCode::Escape]) || pad.pressed(Button::East) || clicked == Some(ui::Click::Back)
    {
        g.close_controls();
        return;
    }
    let rows = controls::Action::ALL.len() + 1;
    if let Some(ui::Click::Row(row)) = clicked.or(keys.hover.and_then(|at| ui::click_at(g, at))) {
        g.controls_row = row;
    }
    if pressed(&[KeyCode::W, KeyCode::Up]) || pad.nav(Dir::Up) {
        g.controls_row = (g.controls_row + rows - 1) % rows;
    }
    if pressed(&[KeyCode::S, KeyCode::Down]) || pad.nav(Dir::Down) {
        g.controls_row = (g.controls_row + 1) % rows;
    }
    match keys.repeat {
        Some(Dir::Up) => g.controls_row = g.controls_row.saturating_sub(1),
        Some(Dir::Down) => g.controls_row = (g.controls_row + 1).min(rows - 1),
        _ => {}
    }
    if pressed(&[KeyCode::Enter, KeyCode::Space])
        || pad.pressed(Button::South)
        || matches!(clicked, Some(ui::Click::Row(_)))
    {
        if g.controls_row == controls::Action::ALL.len() {
            g.reset_controls();
        } else {
            g.rebinding = true;
            g.controls_note = None;
        }
    }
}

/// What each input device did this frame.
#[derive(Default)]
struct Devices {
    pad: bool,
    typed: bool,
    clicked: bool,
    moved: bool,
}
/// Prompts follow whichever device was used last, and the pointer's
/// highlight lasts from the mouse moving or clicking until a key or a
/// controller is used.
fn follow_devices(g: &mut Game, used: Devices, mouse: Vec2) {
    if used.pad {
        g.pad_prompts = true;
        g.pointer = None;
    } else if used.typed || used.clicked {
        g.pad_prompts = false;
    }
    if used.typed {
        g.pointer = None;
    }
    if used.moved || used.clicked {
        g.pointer = Some(mouse);
    }
}
/// The mouse cursor the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cursor {
    Default,
    /// A hand over anything a click would choose.
    Pointer,
    /// None over the game in play, where a click strikes and doesn't aim.
    Hidden,
}
fn cursor_for(g: &Game) -> Cursor {
    if g.screen == Screen::Playing {
        return Cursor::Hidden;
    }
    match g.pointer.and_then(|at| ui::click_at(g, at)) {
        Some(_) => Cursor::Pointer,
        None => Cursor::Default,
    }
}

// Miniquad's browser loader already provides this; it reports whether the
// game's canvas is the page's fullscreen element.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn sapp_is_fullscreen() -> bool;
}
/// Browsers can leave fullscreen without the game (their own **Esc**), so the
/// setting follows each change in the browser's state. `seen` is the browser
/// state last observed. Returns whether the setting changed.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn follow_browser_fullscreen(setting: &mut bool, seen: &mut bool, browser: bool) -> bool {
    if browser == *seen {
        return false;
    }
    *seen = browser;
    let changed = *setting != browser;
    *setting = browser;
    changed
}

/// Seconds the browser has to enter fullscreen after the game asks.
const FULLSCREEN_GRACE: f32 = 1.;
/// Browsers can refuse fullscreen, for example without a recent key press
/// or under a page policy, and Miniquad doesn't report it. `waited` counts
/// seconds since the game asked; returns true once the browser has had
/// `FULLSCREEN_GRACE` without entering it, so the setting can be turned off.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn fullscreen_refused(waited: &mut Option<f32>, browser: bool, dt: f32) -> bool {
    let Some(seconds) = waited else {
        return false;
    };
    if browser {
        *waited = None;
        return false;
    }
    *seconds += dt;
    if *seconds < FULLSCREEN_GRACE {
        return false;
    }
    *waited = None;
    true
}

/// Presentation-only camera shake, scaled by the player's comfort setting.
/// Simulated seconds to run for one frame. Scripted modes (captures, the
/// demo, and the tour) ignore the game-speed option so their output is fixed.
fn sim_seconds(g: &Game, frame_dt: f32, scripted: bool) -> f32 {
    if scripted {
        frame_dt
    } else {
        frame_dt * g.sim_speed()
    }
}
fn camera_shake(g: &Game) -> Vec2 {
    if g.shake <= 0. {
        return Vec2::ZERO;
    }
    vec2((g.time * 93.).sin(), (g.time * 79.).cos()) * g.shake * 0.35 * g.settings.shake_scale()
}

const UI_GALLERY_NAMES: [&str; 44] = [
    "ui-00-title",
    "ui-01-playing",
    "ui-02-low-health-cooldowns-hammer",
    "ui-03-map",
    "ui-04-paused",
    "ui-05-scroll",
    "ui-06-camp-conservatory",
    "ui-07-camp-foundry",
    "ui-08-dead",
    "ui-09-victory",
    "ui-10-crown-boss",
    "ui-11-options",
    "ui-12-paused-abandon",
    "ui-13-tip",
    "ui-14-reliquary",
    "ui-15-atlas-fog",
    "ui-16-controls",
    "ui-17-rebound-hud",
    "ui-18-title-continue",
    "ui-19-pad-hud",
    "ui-20-pad-title-continue",
    "ui-21-pad-camp",
    "ui-22-pad-memory",
    "ui-23-paused-quit",
    "ui-24-title-unreadable-save",
    "ui-25-attack-warnings",
    "ui-26-paused-controller-lost",
    "ui-27-hover-title-button",
    "ui-28-hover-reliquary-card",
    "ui-29-hover-options-arrow",
    "ui-30-hover-title-confirm-quit",
    "ui-31-threats-out-of-view",
    "ui-32-low-vitality-steady",
    "ui-33-hover-controls-row",
    "ui-34-hover-camp-route",
    "ui-35-refused-presses",
    "ui-36-forge-short-of-copper",
    "ui-37-atlas-marks",
    "ui-38-notice-in-play",
    "ui-39-flask-drinking",
    "ui-40-flask-interrupted",
    "ui-41-options-fidelity",
    "ui-42-pause-easing-in",
    "ui-43-vitality-trail",
];

// These are frozen visual fixtures for inspecting the interface, not a playthrough.
// Rebuild each fixture from a fixed seed without reading or writing player saves.
fn ui_fixture(index: usize) -> Game {
    let save = save::Save {
        embers: 74,
        vitality: 2,
        flask: 1,
        runs: 4,
        best_kills: 38,
        ..Default::default()
    };
    let mut g = Game::new(4017, save);
    g.practice = true;
    g.screen = Screen::Playing;
    g.player.pos = vec2(870., world::FLOOR);
    g.player.ground = true;
    g.player.hp = 104.;
    g.player.gold = 360;
    g.player.embers = 18;
    g.player.kills = 12;
    g.player.power = [3, 2, 1];
    g.player.tier = 2;
    g.camera = 640.;
    g.time = 4.;
    g.run_time = 327.;
    g.intro = 0.;
    g.settings.muted = true;
    match index {
        0 => {
            g.screen = Screen::Title;
            g.save.best_time = Some(1123.);
        }
        1 => {}
        2 => {
            g.player.hp = 19.;
            g.player.weapon = Weapon::Hammer;
            g.player.attack_cd = 0.38;
            g.player.bow_cd = 0.22;
            g.player.grenade_cd = 3.4;
            g.player.trap_cd = 5.7;
            g.player.parry_cd = 0.4;
            g.player.dodge_cd = 0.5;
            g.player.flasks = 0;
            g.player.mutation = 2;
        }
        3 => g.map = true,
        4 => g.screen = Screen::Paused,
        5 => g.screen = Screen::Scroll,
        6 | 7 => {
            g.screen = Screen::Camp;
            g.route = index - 6;
            if index == 7 {
                g.player.mutation = 1;
                g.save.flask = 3;
                g.notify("The flask is fully reinforced.");
                g.notice_time = 3.;
            }
        }
        // Result screens show the records as normal play keeps them.
        8 => {
            g.practice = false;
            g.screen = Screen::Dead;
            g.stage = 1;
            g.level = world::Level::generate(4017, world::Biome::Foundry, world::Threat::BASE);
            g.player.hp = 0.;
            g.player.weapon = Weapon::Glaive;
            g.player.mutation = 1;
            g.save.best_stage = Some(1);
            g.result_time = game::RESULT_DELAY;
            g.recap = Some(game::Recap {
                cause: Some(game::Cause::Strike(world::EnemyKind::Brute)),
                biome: world::Biome::Foundry,
                stage: 1,
                embers: 18,
                new_kills: false,
                new_stage: false,
                new_time: false,
            });
        }
        9 => {
            g.practice = false;
            g.screen = Screen::Victory;
            g.stage = 2;
            g.level = world::Level::generate(4017, world::Biome::Crown, world::Threat::BASE);
            g.player.kills = 38;
            g.player.tier = 7;
            g.player.power = [4, 2, 3];
            g.player.weapon = Weapon::Hammer;
            g.player.mutation = 2;
            g.run_time = 1123.;
            g.save.rune = true;
            g.save.wins = 1;
            g.save.best_stage = Some(2);
            g.save.best_time = Some(1123.);
            g.result_time = game::RESULT_DELAY;
            g.recap = Some(game::Recap {
                cause: None,
                biome: world::Biome::Crown,
                stage: 2,
                embers: 41,
                new_kills: true,
                new_stage: true,
                new_time: true,
            });
        }
        10 => {
            g.level = world::Level::generate(4017, world::Biome::Crown, world::Threat::BASE);
            g.stage = 2;
            for enemy in &mut g.level.enemies {
                if enemy.kind == world::EnemyKind::Regent {
                    enemy.hp = enemy.max_hp * 0.58;
                }
            }
        }
        11 => {
            g.screen = Screen::Paused;
            g.open_options();
            g.options_row = 2;
            g.settings.shake = 4;
            g.settings.speed = 8;
            g.settings.reduce_flashes = true;
        }
        12 => {
            g.screen = Screen::Paused;
            g.settings.speed = 7;
            g.request_abandon();
        }
        13 => g.hint = Some((Hint::Parry, HINT_SECONDS)),
        15 => {
            // Fog of war applies to normal play only; this staged view shows it.
            g.practice = false;
            g.map = true;
            for x in (0..=8).map(|i| i as f32 * 110.) {
                g.survey
                    .reveal(Rect::new(x, world::FLOOR - 248., 640., 360.));
            }
            g.survey
                .reveal(Rect::new(420., world::Level::UPPER - 132., 640., 360.));
        }
        16 => {
            g.screen = Screen::Paused;
            g.open_options();
            g.options_row = settings::Settings::CONTROLS_ROW;
            g.open_controls();
            g.controls_row = 4;
            g.settings
                .keys
                .bind(controls::Action::Jump, KeyCode::K)
                .ok();
            g.controls_note = Some("Jump is now K; Glassbolt moved to SPACE.".into());
            g.rebinding = true;
        }
        17 => {
            for (action, key) in [
                (controls::Action::Strike, KeyCode::U),
                (controls::Action::Glassbolt, KeyCode::I),
                (controls::Action::Parry, KeyCode::O),
                (controls::Action::Interact, KeyCode::G),
            ] {
                g.settings.keys.bind(action, key).ok();
            }
            g.hint = Some((Hint::Strike, HINT_SECONDS));
            g.player.pos = vec2(905., world::FLOOR);
        }
        18 => {
            g.screen = Screen::Title;
            g.stage = 1;
            g.level = world::Level::generate(4017, world::Biome::Foundry, world::Threat::BASE);
            g.resume = Some(g.checkpoint());
        }
        // The same screens after a controller was used last.
        19 => {
            g.pad_prompts = true;
            g.hint = Some((Hint::Strike, HINT_SECONDS));
        }
        20 => {
            g.pad_prompts = true;
            g.screen = Screen::Title;
            g.stage = 1;
            g.level = world::Level::generate(4017, world::Biome::Foundry, world::Threat::BASE);
            g.resume = Some(g.checkpoint());
        }
        21 => {
            g.pad_prompts = true;
            g.screen = Screen::Camp;
        }
        22 => {
            g.pad_prompts = true;
            g.screen = Screen::Scroll;
        }
        23 => {
            g.screen = Screen::Paused;
            g.request_quit();
        }
        24 => {
            g.screen = Screen::Title;
            g.save = save::Save::default();
            g.unreadable = vec![
                save::Unreadable {
                    file: save::Save::FILE,
                    kept: Ok("progress.unreadable.json".into()),
                },
                save::Unreadable {
                    file: settings::Settings::FILE,
                    kept: Ok("settings.unreadable.json".into()),
                },
            ];
        }
        25 => {
            // A brute, a warden, and an archer partway through their windups.
            g.level.enemies.clear();
            for (x, kind, left) in [
                (790., world::EnemyKind::Brute, 0.45),
                (925., world::EnemyKind::Warden, 0.12),
                (1170., world::EnemyKind::Archer, 0.22),
            ] {
                let mut e = world::Enemy::new(x, world::FLOOR, kind, world::Threat::BASE);
                e.windup = left;
                e.face = if x < g.player.pos.x { 1. } else { -1. };
                g.level.enemies.push(e);
            }
            g.hint = None;
        }
        26 => g.controller_lost(),
        14 => {
            g.screen = Screen::Reliquary;
            g.offer = Some(Weapon::Hammer);
            g.player.tier = 3;
        }
        // The mouse pointing at a button, a card, an options arrow, and the
        // title's quit link after a first click.
        27 => {
            g.screen = Screen::Title;
            g.pointer = Some(vec2(354., 515.));
        }
        28 => {
            g.screen = Screen::Reliquary;
            g.offer = Some(Weapon::Hammer);
            g.player.tier = 3;
            g.pointer = Some(vec2(455., 440.));
        }
        29 => {
            g.screen = Screen::Paused;
            g.open_options();
            g.options_row = 2;
            g.settings.shake = 4;
            g.pointer = Some(vec2(893., 289.));
        }
        30 => {
            g.screen = Screen::Title;
            g.request_quit();
            g.pointer = Some(vec2(494., 639.));
        }
        31 => {
            // An archer winding up above the view, a brute behind the left
            // edge, and a bolt arriving from above on the right.
            g.level.enemies.clear();
            for (pos, kind, left) in [
                (vec2(960., g.camera_y - 30.), world::EnemyKind::Archer, 0.35),
                (
                    vec2(g.camera - 30., world::FLOOR),
                    world::EnemyKind::Brute,
                    0.8,
                ),
            ] {
                let mut e = world::Enemy::new(pos.x, pos.y, kind, world::Threat::BASE);
                e.windup = kind.windup() * left;
                g.level.enemies.push(e);
            }
            let from = g.player.pos + vec2(300., -274.);
            g.shots.push(Shot {
                pos: from,
                vel: (g.player.pos - vec2(0., 14.) - from).normalize() * 170.,
                life: 3.,
                damage: 10.,
                hostile: true,
                kind: 2,
                from: Some(world::EnemyKind::Archer),
            });
            g.hint = None;
        }
        // Low vitality with a flask left, and flashes reduced so the tint holds steady.
        32 => {
            g.player.hp = 30.;
            g.player.flasks = 2;
            g.settings.reduce_flashes = true;
        }
        // The controls page and the Keeper's route with the mouse in use,
        // so their hints describe clicks.
        33 | 34 => {
            g.screen = Screen::Paused;
            let target = if index == 33 {
                g.open_options();
                g.options_row = settings::Settings::CONTROLS_ROW;
                g.open_controls();
                g.controls_row = 2;
                ui::Click::Row(2)
            } else {
                g.screen = Screen::Camp;
                ui::Click::Route
            };
            g.pointer = ui::targets(&g)
                .into_iter()
                .find(|(_, click)| *click == target)
                .map(|(rect, _)| rect.center());
        }
        // The dodge recovering, and presses of the fire vessel (still
        // recovering) and the flask (none left) that couldn't happen.
        35 => {
            g.player.dodge_cd = 0.4;
            g.player.grenade_cd = 3.1;
            g.player.flasks = 0;
            g.player.flask_note = "EMPTY";
            for slot in [game::Slot::FireVessel, game::Slot::Flask] {
                g.player.refused[slot as usize] = game::REFUSAL_SHOW * 0.8;
            }
        }
        // Beside a forge with too little copper: the prompt says how much.
        36 => {
            g.player.gold = 35;
            let at = g.player.pos - vec2(28., 0.);
            if let Some(forge) = g
                .level
                .objects
                .iter_mut()
                .find(|o| o.kind == world::ObjectKind::Forge)
            {
                forge.pos = at;
            }
        }
        // Every kind of object on the atlas, with the forge short of copper
        // and the cache still sealed, so both are drawn dimmed.
        37 => {
            g.map = true;
            g.player.gold = 35;
            g.player.kills = 3;
        }
        // A notice over the Foundry's bright scenery, under a forge prompt.
        38 => {
            g.level = world::Level::generate(4017, world::Biome::Foundry, world::Threat::BASE);
            g.level.enemies.clear();
            g.player.gold = 35;
            let at = g.player.pos - vec2(28., 0.);
            if let Some(forge) = g
                .level
                .objects
                .iter_mut()
                .find(|o| o.kind == world::ObjectKind::Forge)
            {
                forge.pos = at;
            }
            g.notify("The smith asks for 60 copper to temper your weapon.");
            g.notice_time = 3.;
        }
        // A flask half drunk, and one a hit cut short.
        39 => {
            g.player.hp = 52.;
            g.player.heal_time = game::DRINK_TIME * 0.45;
        }
        40 => {
            g.player.hp = 47.;
            g.player.heal_time = game::DRINK_TIME * 0.45;
            g.hurt(5., -1., game::Cause::Hazard);
        }
        41 => {
            // The world behind is drawn at Ultra too.
            g.screen = Screen::Paused;
            g.open_options();
            g.options_row = settings::Settings::FIDELITY_ROW;
            g.settings.set_fidelity(fidelity::Fidelity::Ultra);
            g.pointer = Some(vec2(847., 367.));
        }
        42 => g.screen = Screen::Paused,
        43 => {
            // A hit has just taken vitality from 104 to 64.
            g.player.hp = 64.;
            g.vitality_trail = Some(game::VitalityTrail::after_hit(104., 64.));
        }
        _ => unreachable!("UI gallery fixture index exceeds its capture list"),
    }
    g
}

const TOUR_FRAMES_PER_BIOME: u32 = 360;
const TOUR_FRAMES: u32 = TOUR_FRAMES_PER_BIOME * 4;
const MOTION_FRAMES: u32 = 900;

// The tour holds gameplay still while moving the camera through the complete
// level, making depth motion and scenery changes independently inspectable.
fn tour_position(frame: u32) -> (world::Biome, f32) {
    let biomes = [
        world::Biome::Aqueduct,
        world::Biome::Garden,
        world::Biome::Foundry,
        world::Biome::Crown,
    ];
    let index = (frame / TOUR_FRAMES_PER_BIOME).min(3) as usize;
    let progress = (frame % TOUR_FRAMES_PER_BIOME) as f32 / (TOUR_FRAMES_PER_BIOME - 1) as f32;
    (biomes[index], progress)
}

fn prepare_tour_frame(g: &mut Game, frame: u32) {
    let (biome, progress) = tour_position(frame);
    if frame.is_multiple_of(TOUR_FRAMES_PER_BIOME) {
        g.level = world::Level::generate(4017, biome, world::Threat::BASE);
        g.level.enemies.clear();
        g.player = Player::new(&save::Save::default());
        g.screen = Screen::Playing;
        g.stage = match biome {
            world::Biome::Aqueduct => 0,
            world::Biome::Garden | world::Biome::Foundry => 1,
            world::Biome::Crown => 2,
        };
        g.intro = 0.;
        g.notice_time = 0.;
    }
    g.camera = (g.level.width - 640.).max(0.) * progress;
    g.player.pos = vec2((g.camera + 230.).min(g.level.width - 20.), world::FLOOR);
    g.player.vel = vec2(145., 0.);
    g.player.ground = true;
    g.time = frame as f32 / 60.;
    g.run_time = g.time;
}

// Scripted inputs still pass through the normal physics and combat simulation.
// The opening heal is safe and observable before the player enters combat.
/// The graphics driver's name for the GPU, for benchmark reports.
fn gl_renderer() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    // SAFETY: GL_RENDERER is a valid name, and the driver returns a static,
    // NUL-terminated string (or null).
    unsafe {
        let name = miniquad::gl::glGetString(0x1F01);
        if !name.is_null() {
            return std::ffi::CStr::from_ptr(name as *const _)
                .to_string_lossy()
                .into_owned();
        }
    }
    "unknown".into()
}
/// The motion capture's opening: low on vitality, so a flask shows.
fn motion_start(g: &mut Game) {
    g.player.hp = g.player.max_hp * 0.45;
    g.intro = 0.;
    g.notice_time = 0.;
}
fn motion_input(frame: u32) -> Input {
    Input {
        axis: if (160..300).contains(&frame) || frame >= 570 {
            1.
        } else {
            0.
        },
        heal: frame == 60,
        parry: frame == 130 || frame == 720,
        jump: matches!(frame, 200 | 215 | 600 | 620 | 810),
        jump_held: (200..245).contains(&frame)
            || (600..650).contains(&frame)
            || (810..835).contains(&frame),
        down: (245..259).contains(&frame),
        dodge: matches!(frame, 260 | 675 | 840),
        attack: (300..380).contains(&frame) || (570..900).contains(&frame) && frame % 90 < 40,
        bow: (405..465).contains(&frame) || (750..780).contains(&frame),
        grenade: frame == 490,
        trap: frame == 525,
        ..Default::default()
    }
}

/// Fits the 16:9 frame inside the window. Drawing rectangles use logical
/// (DPI-independent) units, but camera viewports address physical framebuffer
/// pixels, so the HUD viewport must be scaled or it shrinks into a corner on
/// fractional and Retina displays.
fn letterbox(screen_w: f32, screen_h: f32, dpi: f32) -> (Rect, (i32, i32, i32, i32)) {
    let scale = (screen_w / 1280.).min(screen_h / 720.);
    let (w, h) = (1280. * scale, 720. * scale);
    let (x, y) = ((screen_w - w) / 2., (screen_h - h) / 2.);
    let px = |v: f32| (v * dpi).round() as i32;
    (Rect::new(x, y, w, h), (px(x), px(y), px(w), px(h)))
}

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let capture = args.iter().any(|s| s == "--capture");
    let gallery = args.iter().any(|s| s == "--gallery");
    let sprite_preview = args.iter().any(|s| s == "--sprite-preview");
    let ui_gallery = args.iter().any(|s| s == "--ui-gallery");
    let environment_tour = args.iter().any(|s| s == "--environment-tour");
    let motion_capture = args.iter().any(|s| s == "--motion-capture");
    let vertical_capture = args.iter().any(|s| s == "--vertical-capture");
    let fidelity_bench = args.iter().any(|s| s == "--fidelity-bench");
    let automated = capture || motion_capture || vertical_capture || fidelity_bench;
    let no_postfx = args.iter().any(|s| s == "--no-postfx");
    let profile_render = args.iter().any(|s| s == "--profile-render");
    // Testing aid: measures how whole steps fall across real frames here.
    let pacing_check = args.iter().any(|s| s == "--pacing-check");
    let demo = args.iter().any(|s| s == "--demo") || automated;
    let staged = ui_gallery || gallery || environment_tour;
    // Testing aid: saves the frame every so many seconds of real time, so
    // runs driven from outside (such as scripts/virtual-pad.py) leave a record.
    let snapshot_every = args
        .iter()
        .position(|a| a == "--snapshot-every")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|s| *s > 0. && cfg!(not(target_arch = "wasm32")));
    let mut next_snapshot = 0.;
    let mut snapshots = 0u32;
    let start_at = if cfg!(target_arch = "wasm32") {
        launch::StartAt::from_page()
    } else {
        launch::StartAt::from_args(&args).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(2)
        })
    };
    let practice = isolated(&args) || start_at.is_some();
    debug_assert_eq!(
        practice,
        demo || staged || sprite_preview || start_at.is_some()
    );
    let seed = if practice {
        4017
    } else {
        // `std::time` is unavailable in the browser; miniquad's clock is portable.
        miniquad::date::now() as u64
    };
    let mut g = if practice {
        Game::new(seed, save::Save::default())
    } else {
        Game::load(seed)
    };
    g.practice = practice;
    // Launch flags choose a fidelity for this session without saving it.
    let launch_fidelity = match args.iter().position(|a| a == "--fidelity") {
        Some(i) => Some(
            args.get(i + 1)
                .and_then(|v| fidelity::Fidelity::parse(v))
                .unwrap_or_else(|| {
                    eprintln!("--fidelity takes low, medium, high, or ultra");
                    std::process::exit(2)
                }),
        ),
        None => no_postfx.then_some(fidelity::Fidelity::Low),
    };
    g.session_fidelity = launch_fidelity;
    let mut chosen_fidelity = g.settings.fidelity;
    if practice {
        g.start();
    }
    if let Some(target) = start_at {
        target.apply(&mut g);
    }
    if motion_capture {
        motion_start(&mut g);
    }
    if pacing_check {
        // Nothing should end the run while frame times are measured.
        g.level.enemies.clear();
        g.level.hazards.clear();
        g.intro = 0.;
    }
    if vertical_capture {
        // Navigation proof: authored geometry and live input/physics, with combat
        // removed so the entire route can be inspected without a scripted fight.
        g.level.enemies.clear();
        g.level.hazards.clear();
        g.intro = 0.;
        g.notice_time = 0.;
    }
    let mut traversal = traversal_capture::Traversal::new();
    let mut arrival_hold = 0_u32;
    let ui = render::Ui::new();
    let mut audio = audio::Audio::new().await;
    let mut art = art::Art::new();
    let mut postfx = postprocess::PostProcess::new();
    // The world is drawn into a 1280 × 720 scene, or one twice that size at
    // Ultra, which is filtered down to the window (supersampling).
    let scene_target = |scale: u32| {
        let target = render_target(1280 * scale, 720 * scale);
        target.texture.set_filter(if scale > 1 {
            FilterMode::Linear
        } else {
            FilterMode::Nearest
        });
        target
    };
    let mut targets = vec![(1, scene_target(1))];
    let mut accumulator = 0.;
    // Play is drawn between the last two steps; menus, staged views, and
    // scripted captures draw whole steps, so their output doesn't change.
    // (Until round 10 the motion and vertical captures, which run at real
    // frame times, were blended too.)
    let blending = !staged && !demo && !sprite_preview && !automated;
    let mut pose = blend::Pose::default();
    let mut pacing = blend::Pacing::default();
    let mut pacing_warmup = 1.0_f32;
    let mut pending = Input::default();
    let mut frame = 0u32;
    // Browsers only allow fullscreen after a key press, so only desktop
    // builds restore a saved choice at launch.
    let mut fullscreen = false;
    let mut window_watch = WindowWatch::default();
    #[cfg(target_arch = "wasm32")]
    let mut browser_fullscreen = false;
    // Seconds since the browser build asked for fullscreen, until it enters.
    #[cfg(target_arch = "wasm32")]
    let mut fullscreen_wait: Option<f32> = None;
    if cfg!(target_arch = "wasm32") {
        g.settings.fullscreen = false;
    } else if g.settings.fullscreen && !staged && !automated {
        fullscreen = true;
        set_fullscreen(true);
    }
    let mut pad = pad::Pad::new();
    let mut mouse_held_over = false;
    let mut menu_repeat = MenuRepeat::default();
    let mut last_mouse = Vec2::ZERO;
    let mut shown_cursor = Cursor::Default;
    let focus_events = macroquad::input::utils::register_input_subscriber();
    let mut focused = true;
    let mut previous_level = (g.level.seed, g.level.biome, g.seed);
    let mut arrival = 0.0_f32;
    let mut render_times = Vec::new();
    let mut bench = bench::Bench::default();
    let mut menu_timer = ui::MenuTimer::default();
    loop {
        if fidelity_bench {
            #[cfg(not(target_arch = "wasm32"))]
            // SAFETY: a plain GL call on the thread that owns the context.
            unsafe {
                miniquad::gl::glFinish()
            };
            bench.mark(frame, miniquad::date::now() * 1000.);
            if bench::Bench::phase(frame) == bench::Phase::Finished {
                println!(
                    "fidelity-bench {}x{} window pixels, renderer {}\n{}",
                    screen_width() * screen_dpi_scale(),
                    screen_height() * screen_dpi_scale(),
                    gl_renderer(),
                    bench.report()
                );
                break;
            }
            if bench::Bench::script_frame(frame) == 0 {
                // Every pass plays the script from the same start.
                g = Game::new(seed, save::Save::default());
                g.practice = true;
                g.start();
                motion_start(&mut g);
                art.restart();
                accumulator = 0.;
                pending = Input::default();
            }
            g.session_fidelity = Some(bench::Bench::fidelity(frame));
        }
        let frame_started = miniquad::date::now();
        if g.settings.fidelity != chosen_fidelity {
            // Choosing a step in the game replaces a launch flag's.
            chosen_fidelity = g.settings.fidelity;
            g.session_fidelity = None;
        }
        if ui_gallery {
            g = ui_fixture(frame as usize);
            g.session_fidelity = launch_fidelity;
        } else if environment_tour {
            prepare_tour_frame(&mut g, frame);
        } else if gallery {
            let biome = [
                world::Biome::Aqueduct,
                world::Biome::Garden,
                world::Biome::Foundry,
                world::Biome::Crown,
            ][frame as usize];
            g.level = world::Level::generate(4017, biome, world::Threat::BASE);
            g.player.pos = vec2(870., world::FLOOR);
            g.camera = 640.;
            g.time = 4.;
            g.intro = 0.;
        }
        let mut watch = FocusWatch::default();
        macroquad::input::utils::repeat_all_miniquad_input(&mut watch, focus_events);
        if watch.lost {
            focused = false;
            // Captures and staged views run unattended, often unfocused.
            if !staged && !automated {
                g.focus_lost();
            }
        }
        if watch.regained {
            focused = true;
        }
        // Controllers are read whatever has focus, so they wait until the
        // game is back in front.
        if focused {
            pad.poll();
            match pad.connection() {
                // Captures and staged views run unattended.
                pad::Connection::Lost if !staged && !automated => g.controller_lost(),
                pad::Connection::Found => g.pad_lost = false,
                _ => {}
            }
        } else {
            pad.feed(pad::State::default());
        }
        let screen_before_menus = g.screen;
        if !staged && !automated {
            let (frame, _) = letterbox(screen_width(), screen_height(), screen_dpi_scale());
            let mouse = to_interface(frame, mouse_position().into());
            let clicked = is_mouse_button_pressed(MouseButton::Left);
            let moved = mouse.distance(last_mouse) > 0.5;
            last_mouse = mouse;
            follow_devices(
                &mut g,
                Devices {
                    pad: pad.touched(),
                    typed: !get_keys_pressed().is_empty(),
                    clicked: clicked || is_mouse_button_pressed(MouseButton::Right),
                    moved,
                },
                mouse,
            );
            let pressed = |k| is_key_pressed(k);
            let held = if MenuRepeat::applies(&g) {
                menu_repeat
                    .choose(|d| pad.holding(d) || menu_keys(d).iter().any(|k| is_key_down(*k)))
            } else {
                None
            };
            let repeat = menu_repeat.update(held, get_frame_time());
            let keys = MenuKeys {
                pressed: &pressed,
                last: get_last_key_pressed(),
                click: clicked.then_some(mouse),
                hover: moved.then_some(mouse),
                repeat,
            };
            menus(&mut g, &keys, &pad);
            let cursor = cursor_for(&g);
            if cursor != shown_cursor {
                shown_cursor = cursor;
                miniquad::window::show_mouse(cursor != Cursor::Hidden);
                miniquad::window::set_mouse_cursor(if cursor == Cursor::Pointer {
                    miniquad::CursorIcon::Pointer
                } else {
                    miniquad::CursorIcon::Default
                });
            }
            if g.screen != screen_before_menus {
                g.quit_armed = false;
            }
            if g.quit {
                break;
            }
        }
        if screen_before_menus != Screen::Playing || g.screen != Screen::Playing {
            pad.hold_over();
            if is_mouse_button_pressed(MouseButton::Left) {
                mouse_held_over = true;
            }
        }
        if !is_mouse_button_down(MouseButton::Left) {
            mouse_held_over = false;
        }
        if !staged && !automated {
            if is_key_pressed(KeyCode::F11) {
                g.settings.fullscreen = !g.settings.fullscreen;
                g.persist_settings();
            }
            #[cfg(target_arch = "wasm32")]
            {
                // SAFETY: a plain value call into Miniquad's loader.
                let browser = unsafe { sapp_is_fullscreen() };
                if follow_browser_fullscreen(
                    &mut g.settings.fullscreen,
                    &mut browser_fullscreen,
                    browser,
                ) {
                    // Already applied by the browser; asking it to leave
                    // again would only log an error.
                    fullscreen = browser;
                    g.persist_settings();
                }
                if fullscreen_refused(&mut fullscreen_wait, browser, get_frame_time()) {
                    // The page never left windowed mode, so there's nothing
                    // to undo; only the setting is wrong.
                    g.settings.fullscreen = false;
                    fullscreen = false;
                    g.persist_settings();
                    g.notify("The browser didn't allow fullscreen.");
                }
            }
            // F11 and the options row both change the setting; follow it.
            if g.settings.fullscreen != fullscreen {
                fullscreen = g.settings.fullscreen;
                set_fullscreen(fullscreen);
                #[cfg(target_arch = "wasm32")]
                {
                    fullscreen_wait = fullscreen.then_some(0.);
                }
            }
            if REMEMBER_WINDOW && !practice {
                let (w, h) = miniquad::window::screen_size();
                if let Some(size) = window_watch.watch(
                    [w as u32, h as u32],
                    fullscreen,
                    get_frame_time(),
                    g.settings.window,
                ) {
                    g.settings.window = Some(size);
                    g.persist_settings();
                }
            }
        }
        if !staged && !automated && is_key_pressed(KeyCode::F9) {
            let next = g.fidelity().cycled();
            g.settings.set_fidelity(next);
            g.session_fidelity = None;
            chosen_fidelity = next;
            g.persist_settings();
            if next.post() && !postfx.available() {
                g.notify("Lighting is unavailable on this graphics backend.");
            } else {
                g.notify(&format!("Graphics fidelity: {}", next.name()));
            }
        }
        let mut input = if staged {
            Input::default()
        } else {
            pad::combine(read_input(&g, !mouse_held_over), pad.gameplay())
        };
        if vertical_capture && !staged {
            input = traversal.input(&g);
        } else if motion_capture && !staged {
            input = motion_input(frame);
        } else if fidelity_bench {
            input = motion_input(bench::Bench::script_frame(frame));
        } else if demo && !staged {
            let t = frame as f32 / 60.;
            input = Input {
                axis: if t < 1.2 { 0. } else { 1. },
                attack: t > 2.,
                bow: t > 5.,
                jump: frame % 150 == 90,
                jump_held: frame % 150 >= 90 && frame % 150 < 115,
                dodge: frame.is_multiple_of(230) && frame > 0,
                grenade: frame % 310 == 160,
                trap: frame % 400 == 200,
                interact: true,
                ..Default::default()
            };
            if g.screen == Screen::Scroll {
                g.upgrade(0);
            }
            if g.screen == Screen::Reliquary {
                g.choose_weapon(true);
            }
            if g.screen == Screen::Camp {
                g.travel();
            }
            if g.screen == Screen::Dead {
                g.start();
            }
        }
        input.jump |= pending.jump;
        input.dodge |= pending.dodge;
        input.parry |= pending.parry;
        input.grenade |= pending.grenade;
        input.trap |= pending.trap;
        input.heal |= pending.heal;
        input.interact |= pending.interact;
        let paused = g.screen.freezes_world();
        let frame_dt = if demo || environment_tour {
            1. / 60.
        } else {
            get_frame_time().min(0.1)
        };
        if staged || paused {
            accumulator = 0.;
        }
        accumulator += if staged || paused {
            0.
        } else {
            sim_seconds(&g, frame_dt, demo || environment_tour)
        };
        let mut animation_dt = 0.;
        let mut steps = 0;
        while accumulator >= world::STEP {
            let animate_step =
                g.screen == Screen::Title || (g.screen == Screen::Playing && g.hitstop <= 0.);
            pose.record(&g);
            g.tick(world::STEP, input);
            steps += 1;
            if animate_step {
                animation_dt += world::STEP;
            }
            accumulator -= world::STEP;
            input.jump = false;
            input.dodge = false;
            input.parry = false;
            input.grenade = false;
            input.trap = false;
            input.heal = false;
            input.interact = false;
        }
        pending = if staged || paused {
            Input::default()
        } else {
            input
        };
        if pacing_check {
            if pacing_warmup > 0. {
                pacing_warmup -= frame_dt;
            } else {
                pacing.push(frame_dt, steps);
            }
            if pacing.seconds() >= 8. {
                println!("pacing {}", pacing.report());
                break;
            }
        }
        let silent = automated || staged || sprite_preview;
        let score = audio::Track::for_game(&g);
        audio.update(
            &mut g.sounds,
            score,
            get_frame_time().min(0.1),
            if silent { 0. } else { g.settings.music_gain() },
            if silent {
                0.
            } else {
                g.settings.effects_gain()
            },
        );
        let fidelity = if sprite_preview {
            fidelity::Fidelity::Low
        } else {
            g.fidelity()
        };
        art.set_smooth(fidelity.smooth_textures());
        let scale = fidelity.scene_scale();
        if !targets.iter().any(|(s, _)| *s == scale) {
            targets.push((scale, scene_target(scale)));
        }
        let target = targets
            .iter()
            .find(|(s, _)| *s == scale)
            .map(|(_, t)| t.clone())
            .expect("made above");
        art.animate(
            &g,
            if environment_tour {
                frame_dt
            } else {
                animation_dt
            },
        );
        let current_level = (g.level.seed, g.level.biome, g.seed);
        if current_level != previous_level && !ui_gallery && !gallery {
            arrival = 0.45;
        }
        previous_level = current_level;
        if !paused {
            arrival = (arrival - frame_dt).max(0.);
        }
        // Drawn partway into the next step; put back once the frame is drawn.
        let shown = (blending && g.screen == Screen::Playing)
            .then(|| pose.show(&mut g, blend::leftover(accumulator)));
        let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 640., 360.));
        camera.render_target = Some(target.clone());
        camera.target += camera_shake(&g);
        camera.target = camera.target.round();
        let mut backdrop_camera = Camera2D::from_display_rect(Rect::new(0., 0., 640., 360.));
        backdrop_camera.render_target = Some(target.clone());
        backdrop_camera.target = camera.target;
        camera.target.y += (g.camera_y * 2.).round() / 2.;
        let camera_offset = camera.target - vec2(320., 180.);
        if sprite_preview && !ui_gallery {
            set_camera(&backdrop_camera);
            clear_background(render::INK);
            art.preview(g.time);
            draw_text(
                "CINDERWAKE / IMAGEGEN ANIMATION ATLAS",
                46.,
                26.,
                16.,
                Color::from_hex(0xf2d7a2),
            );
        } else {
            set_camera(&backdrop_camera);
            art.environment.background(
                g.level.biome,
                (g.camera * 2.).round() / 2.,
                g.level.width,
                g.time,
                g.camera_y,
            );
            set_camera(&camera);
            render::scene(&g, &art);
        }
        if fidelity.post() {
            postfx.prepare(&target.texture, &g, camera_offset);
        }
        set_default_camera();
        clear_background(render::INK);
        let (frame_rect, viewport) = letterbox(screen_width(), screen_height(), screen_dpi_scale());
        let Rect { x, y, w, h } = frame_rect;
        postfx.draw(
            &target.texture,
            frame_rect,
            w * screen_dpi_scale(),
            fidelity,
        );
        if arrival > 0. && !sprite_preview {
            let amount = arrival / 0.45;
            draw_rectangle(x, y, w, h, render::INK.with_alpha(amount * amount));
        }
        let ui_cam = Camera2D {
            zoom: vec2(2. / 1280., 2. / 720.),
            viewport: Some(viewport),
            ..Camera2D::from_display_rect(Rect::new(0., 0., 1280., 720.))
        };
        set_camera(&ui_cam);
        // Menus ease in over real time; staged views and captures draw them
        // in place.
        let reveal = if ui_gallery && UI_GALLERY_NAMES[frame as usize] == "ui-42-pause-easing-in" {
            // The pause screen caught partway into place.
            ui::Reveal {
                dim: ui::ease_in_place(ui::Reveal::SECONDS * 0.3),
                panel: ui::ease_in_place(ui::Reveal::SECONDS * 0.3),
            }
        } else if staged || automated {
            ui::Reveal::SHOWN
        } else {
            menu_timer.update(&g, get_frame_time())
        };
        if !sprite_preview || ui_gallery {
            ui.draw(&g, reveal);
        }
        if let Some(now) = shown {
            now.restore(&mut g);
        } else {
            // Nothing to draw between, so the next frame starts from here.
            pose.record(&g);
        }
        if environment_tour && !ui_gallery {
            draw_text(
                "STAGED ENVIRONMENT TOUR / CAMERA TRAVERSAL",
                20.,
                604.,
                15.,
                Color::from_hex(0xa1c6c4),
            );
        } else if vertical_capture && !staged {
            draw_text(
                "ROUTE TEST / LIVE MOVEMENT AND PHYSICS / COMBAT DISABLED",
                20.,
                604.,
                15.,
                Color::from_hex(0xa1c6c4),
            );
        } else if motion_capture && !staged {
            draw_text(
                "SCRIPTED INPUT CAPTURE / LIVE PHYSICS AND COMBAT",
                20.,
                604.,
                15.,
                Color::from_hex(0xa1c6c4),
            );
        }
        set_default_camera();
        if profile_render && frame >= 60 {
            // CPU simulation + draw submission only; capture readback and vsync
            // are excluded, so this is not presented as GPU frame rate.
            render_times.push((miniquad::date::now() - frame_started) * 1000.);
        }
        if let Some(every) = snapshot_every {
            let now = miniquad::date::now();
            if now >= next_snapshot {
                next_snapshot = now + every;
                std::fs::create_dir_all("captures/snapshots").ok();
                get_screen_data().export_png(&format!("captures/snapshots/{snapshots:04}.png"));
                snapshots += 1;
            }
        }
        // Browsers have no writable `captures/` directory.
        if !staged
            && (is_key_pressed(KeyCode::F12) && cfg!(not(target_arch = "wasm32"))
                || (capture && frame.is_multiple_of(6))
                || (sprite_preview && frame == 120))
        {
            std::fs::create_dir_all("captures").ok();
            let path = if sprite_preview {
                "captures/animation-preview.png".into()
            } else {
                format!("captures/frame-{frame:05}.png")
            };
            get_screen_data().export_png(&path);
        }
        if (environment_tour || motion_capture || vertical_capture)
            && !ui_gallery
            && frame.is_multiple_of(3)
        {
            let directory = if environment_tour {
                "tour"
            } else if vertical_capture {
                "vertical"
            } else {
                "motion"
            };
            std::fs::create_dir_all(format!("captures/{directory}")).ok();
            get_screen_data().export_png(&format!("captures/{directory}/frame-{frame:05}.png"));
        }
        if ui_gallery {
            std::fs::create_dir_all("captures").ok();
            get_screen_data().export_png(&format!(
                "captures/{}.png",
                UI_GALLERY_NAMES[frame as usize]
            ));
            if frame as usize + 1 == UI_GALLERY_NAMES.len() {
                break;
            }
        }
        if !ui_gallery && sprite_preview && frame >= 180 {
            break;
        }
        if gallery && !ui_gallery {
            std::fs::create_dir_all("captures").ok();
            get_screen_data().export_png(&format!("captures/biome-{frame}.png"));
            if frame == 3 {
                break;
            }
        }
        if capture && !ui_gallery && frame >= 900 {
            break;
        }
        if fidelity_bench && bench::Bench::shot(frame) {
            std::fs::create_dir_all("captures/fidelity").ok();
            get_screen_data().export_png(&format!(
                "captures/fidelity/{}.png",
                bench::Bench::fidelity(frame).name().to_lowercase()
            ));
        }
        if (environment_tour && frame + 1 >= TOUR_FRAMES)
            || (motion_capture && !staged && frame + 1 >= MOTION_FRAMES)
        {
            break;
        }
        if vertical_capture {
            if traversal.finished(&g) {
                arrival_hold += 1;
            }
            if arrival_hold >= 60 || frame >= 5400 {
                eprintln!(
                    "Vertical route: {}/{} targets reached in {:.2}s; finished={}",
                    traversal.waypoint,
                    g.level.traversal.len(),
                    frame as f32 / 60.,
                    traversal.finished(&g)
                );
                break;
            }
        }
        frame += 1;
        next_frame().await
    }
    if !render_times.is_empty() {
        render_times.sort_by(f64::total_cmp);
        let mean = render_times.iter().sum::<f64>() / render_times.len() as f64;
        let p95 = render_times[(render_times.len() * 95 / 100).min(render_times.len() - 1)];
        eprintln!("CPU simulation/draw submission: mean {mean:.2} ms, p95 {p95:.2} ms ({} frames; excludes readback/vsync)", render_times.len());
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;

    #[test]
    fn game_speed_slows_play_but_not_menus_or_scripted_modes() {
        let mut g = Game::new(4017, save::Save::default());
        g.screen = Screen::Playing;
        // One real second of 60 fps frames, at each speed.
        let second = |g: &Game, scripted: bool| {
            (0..60)
                .map(|_| sim_seconds(g, 1. / 60., scripted))
                .sum::<f32>()
        };
        assert!((second(&g, false) - 1.).abs() < 1e-4);
        g.settings.speed = 7;
        assert!((second(&g, false) - 0.7).abs() < 1e-4);
        g.settings.speed = settings::Settings::SLOWEST;
        assert!((second(&g, false) - 0.5).abs() < 1e-4);
        assert!(
            (second(&g, true) - 1.).abs() < 1e-4,
            "captures keep their timing"
        );
        for screen in [Screen::Title, Screen::Dead, Screen::Victory] {
            g.screen = screen;
            assert!((second(&g, false) - 1.).abs() < 1e-4, "{screen:?}");
        }
        // Half speed takes twice as long to run the same simulation.
        let at = |speed| {
            let mut g = Game::new(4017, save::Save::default());
            g.start();
            g.settings.speed = speed;
            g
        };
        let (mut normal, mut slow) = (at(10), at(5));
        let run = |g: &mut Game, frames: u32| {
            let mut accumulator = 0.;
            for _ in 0..frames {
                accumulator += sim_seconds(g, 1. / 60., false);
                while accumulator >= world::STEP {
                    g.tick(
                        world::STEP,
                        Input {
                            axis: 1.,
                            ..Default::default()
                        },
                    );
                    accumulator -= world::STEP;
                }
            }
        };
        run(&mut normal, 60);
        run(&mut slow, 120);
        assert!((normal.player.pos.x - slow.player.pos.x).abs() < 0.5);
        assert!((normal.run_time - slow.run_time).abs() < 0.02);
    }
    #[test]
    fn the_fullscreen_setting_follows_the_browser() {
        // F11 turns the setting on; the browser enters a frame or two later.
        let (mut setting, mut seen) = (true, false);
        assert!(!follow_browser_fullscreen(&mut setting, &mut seen, false));
        assert!(setting, "a pending request isn't undone");
        assert!(!follow_browser_fullscreen(&mut setting, &mut seen, true));
        // The browser's Esc leaves fullscreen: the setting follows.
        assert!(follow_browser_fullscreen(&mut setting, &mut seen, false));
        assert!(!setting && !seen);
        // So the next F11 is a real request again, not a catch-up.
        setting = !setting;
        assert!(setting);
        // F11 leaving fullscreen: the browser agrees, nothing to follow.
        assert!(!follow_browser_fullscreen(&mut setting, &mut seen, true));
        setting = false;
        assert!(!follow_browser_fullscreen(&mut setting, &mut seen, false));
        assert!(!setting);
    }
    #[test]
    fn a_refused_browser_fullscreen_gives_up_after_a_second() {
        let mut waited = None;
        assert!(!fullscreen_refused(&mut waited, false, 0.5), "not asked");
        waited = Some(0.);
        for _ in 0..3 {
            assert!(!fullscreen_refused(&mut waited, false, 0.25));
        }
        assert!(fullscreen_refused(&mut waited, false, 0.25));
        assert_eq!(waited, None, "and only once");
        assert!(!fullscreen_refused(&mut waited, false, 1.));
        // A browser that enters, however slowly, ends the wait.
        waited = Some(0.);
        assert!(!fullscreen_refused(&mut waited, false, 0.9));
        assert!(!fullscreen_refused(&mut waited, true, 0.2));
        assert_eq!(waited, None);
    }

    #[test]
    fn shake_setting_scales_only_the_presented_camera() {
        let mut g = Game::new(4017, save::Save::default());
        g.time = 1.3;
        g.shake = 6.;
        let full = camera_shake(&g);
        assert!(full.length() > 0.);
        g.settings.shake = 5;
        assert!((camera_shake(&g) - full * 0.5).length() < 1e-4);
        g.settings.shake = 0;
        assert_eq!(camera_shake(&g), Vec2::ZERO);
        assert_eq!(g.shake, 6., "the simulation's shake timer is untouched");
    }

    /// Holds a key or controller buttons for `seconds` of 60 Hz frames, the
    /// way the main loop reads them, then releases them.
    fn hold(g: &mut Game, key: Option<KeyCode>, buttons: &[pad::Button], seconds: f32) {
        let mut repeat = MenuRepeat::default();
        let mut pad = pad::Pad::new();
        let frames = (seconds * 60.).round() as u32;
        for frame in 0..=frames {
            let down = frame < frames;
            pad.feed(if down {
                pad::State::with(buttons, Vec2::ZERO)
            } else {
                pad::State::default()
            });
            let pressed = |k: KeyCode| frame == 0 && Some(k) == key;
            let held = if MenuRepeat::applies(g) {
                repeat.choose(|d| {
                    down && (pad.holding(d) || key.is_some_and(|k| menu_keys(d).contains(&k)))
                })
            } else {
                None
            };
            let keys = MenuKeys {
                pressed: &pressed,
                last: None,
                click: None,
                hover: None,
                repeat: repeat.update(held, 1. / 60.),
            };
            menus(g, &keys, &pad);
        }
    }

    #[test]
    fn held_directions_repeat_after_a_pause() {
        use pad::Dir::*;
        let mut r = MenuRepeat::default();
        let fired: Vec<u32> = (0..60)
            .filter(|_| r.update(Some(Left), 1. / 60.).is_some())
            .collect();
        // The press acts by itself; repeats follow at 0.4 s, then every 0.1 s.
        assert_eq!(fired.len(), 6, "{fired:?}");
        assert!((24..=25).contains(&fired[0]), "{fired:?}");
        assert_eq!(r.update(None, 1. / 60.), None, "releasing stops at once");
        assert_eq!(
            r.update(Some(Right), 0.5),
            None,
            "a new direction starts over"
        );
        assert_eq!(r.update(Some(Right), 0.39), None);
        assert_eq!(r.update(Some(Right), 0.02), Some(Right));
        // A direction already held is kept while another joins it.
        let r = MenuRepeat {
            held: Some(Down),
            time: 1.,
        };
        assert_eq!(r.choose(|_| true), Some(Down));
        assert_eq!(r.choose(|d| d == Left), Some(Left));
    }

    #[test]
    fn holding_steps_bars_and_rows_without_wrapping() {
        use pad::Button::*;
        let mut g = Game::new(4017, save::Save::default());
        g.open_options();
        assert_eq!((g.options_row, g.settings.music), (0, 10));
        hold(&mut g, Some(KeyCode::A), &[], 0.3);
        assert_eq!(g.settings.music, 9, "a short hold is one step");
        g.sounds.clear();
        hold(&mut g, Some(KeyCode::A), &[], 1.5);
        assert_eq!(g.settings.music, 0, "a long hold keeps stepping");
        assert_eq!(g.sounds.len(), 9, "one cue per change, and none at the end");
        hold(&mut g, Some(KeyCode::Right), &[], 0.75);
        assert_eq!(g.settings.music, 5, "the press and four repeats");
        // A controller held down walks the rows and stops at the last.
        hold(&mut g, None, &[DpadDown], 2.);
        assert_eq!(g.options_row, settings::Settings::CONTROLS_ROW);
        assert_eq!(g.screen, Screen::Options, "repeats don't open the controls");
        hold(&mut g, Some(KeyCode::Up), &[], 0.3);
        assert_eq!(g.options_row, settings::Settings::CONTROLS_ROW - 1);
        // Switches flip once per press, however long it's held.
        g.options_row = 3;
        let hitstop = g.settings.hitstop;
        hold(&mut g, None, &[DpadRight], 1.5);
        assert_eq!(g.settings.hitstop, !hitstop);
        // Fidelity steps like a bar and stops at Low.
        g.options_row = settings::Settings::FIDELITY_ROW;
        hold(&mut g, None, &[DpadLeft], 1.5);
        assert_eq!(g.settings.fidelity, fidelity::Fidelity::Low);
        hold(&mut g, Some(KeyCode::D), &[], 0.3);
        assert_eq!(g.settings.fidelity, fidelity::Fidelity::Medium);
        // Up to the first row, where it stops.
        hold(&mut g, Some(KeyCode::W), &[], 2.);
        assert_eq!(g.options_row, 0);
        // The controls page: down to restore, never round to the top.
        g.open_controls();
        hold(&mut g, Some(KeyCode::S), &[], 2.);
        assert_eq!(g.controls_row, controls::Action::ALL.len());
        // Nothing moves while a key is being listened for.
        g.controls_row = 2;
        g.rebinding = true;
        hold(&mut g, None, &[DpadDown], 1.5);
        assert_eq!(g.controls_row, 2);
    }

    #[test]
    fn a_controller_reaches_every_menu() {
        use pad::Button::*;
        let none = |_: KeyCode| false;
        let keys = MenuKeys {
            pressed: &none,
            last: None,
            click: None,
            hover: None,
            repeat: None,
        };
        let mut pad = pad::Pad::new();
        // One frame with the buttons down, then one with them released.
        let mut press = |g: &mut Game, buttons: &[pad::Button]| {
            pad.feed(pad::State::with(buttons, Vec2::ZERO));
            menus(g, &keys, &pad);
            pad.feed(pad::State::default());
            menus(g, &keys, &pad);
        };
        let mut g = Game::new(4017, save::Save::default());
        assert_eq!(g.screen, Screen::Title);
        press(&mut g, &[East]);
        assert!(g.quit_armed && !g.quit);
        press(&mut g, &[East]);
        assert!(g.quit, "B twice quits from the title");
        g.quit = false;
        g.quit_armed = false;
        press(&mut g, &[South]);
        assert_eq!(g.screen, Screen::Playing);
        press(&mut g, &[Select]);
        assert!(g.map);
        press(&mut g, &[Select]);
        assert!(!g.map);
        press(&mut g, &[Start]);
        assert_eq!(g.screen, Screen::Paused);
        press(&mut g, &[East]);
        assert_eq!(g.screen, Screen::Playing, "B also resumes");
        press(&mut g, &[Start]);
        press(&mut g, &[North]);
        assert_eq!(g.screen, Screen::Options);
        press(&mut g, &[DpadDown]);
        assert_eq!(g.options_row, 1);
        let before = g.settings.clone();
        press(&mut g, &[DpadLeft]);
        assert_ne!(g.settings, before, "left lowers the effects volume");
        g.options_row = settings::Settings::CONTROLS_ROW;
        press(&mut g, &[South]);
        assert_eq!(g.screen, Screen::Controls);
        press(&mut g, &[DpadDown]);
        assert_eq!(g.controls_row, 1);
        press(&mut g, &[East]);
        assert_eq!(g.screen, Screen::Options);
        press(&mut g, &[East]);
        assert_eq!(g.screen, Screen::Paused);
        press(&mut g, &[Select]);
        assert!(g.quit_armed, "View asks to quit from the pause screen");
        press(&mut g, &[West]);
        assert!(g.abandon_armed && g.screen == Screen::Paused);
        press(&mut g, &[West]);
        assert_eq!(g.screen, Screen::Dead);
        press(&mut g, &[South]);
        assert_eq!(g.screen, Screen::Dead, "the recap holds for a second");
        g.result_time = game::RESULT_DELAY;
        press(&mut g, &[South]);
        assert_eq!(g.screen, Screen::Playing);
        g.screen = Screen::Scroll;
        press(&mut g, &[North]);
        assert_eq!((g.screen, g.player.power), (Screen::Playing, [1, 2, 1]));
        g.screen = Screen::Reliquary;
        g.offer = Some(Weapon::Hammer);
        press(&mut g, &[West]);
        assert_eq!(g.player.weapon, Weapon::Hammer);
        g.screen = Screen::Camp;
        g.route = 0;
        press(&mut g, &[DpadRight]);
        assert_eq!(g.route, 1);
        press(&mut g, &[South]);
        assert_eq!(
            (g.screen, g.level.biome),
            (Screen::Playing, world::Biome::Foundry)
        );
    }

    #[test]
    fn mouse_clicks_choose_in_menus() {
        let none = |_: KeyCode| false;
        let pad = pad::Pad::new();
        let click = |g: &mut Game, x: f32, y: f32| {
            let keys = MenuKeys {
                pressed: &none,
                last: None,
                click: Some(vec2(x, y)),
                hover: None,
                repeat: None,
            };
            menus(g, &keys, &pad);
        };
        let mut g = Game::new(4017, save::Save::default());
        click(&mut g, 900., 300.);
        assert_eq!(
            g.screen,
            Screen::Title,
            "a click off the button does nothing"
        );
        click(&mut g, 354., 515.);
        assert_eq!(g.screen, Screen::Playing, "the title's button begins");
        g.screen = Screen::Paused;
        click(&mut g, 640., 300.);
        assert_eq!(g.screen, Screen::Paused);
        click(&mut g, 640., 520.);
        assert_eq!(g.screen, Screen::Playing, "the pause button resumes");
        g.screen = Screen::Scroll;
        click(&mut g, 640., 400.);
        assert_eq!((g.screen, g.player.power), (Screen::Playing, [1, 2, 1]));
        g.screen = Screen::Reliquary;
        g.offer = Some(Weapon::Hammer);
        click(&mut g, 825., 400.);
        assert_eq!(
            (g.screen, g.player.weapon),
            (Screen::Playing, Weapon::Sabre),
            "the right card keeps the weapon"
        );
        g.screen = Screen::Camp;
        g.save.embers = 100;
        click(&mut g, 640., 250.);
        assert_eq!(g.save.vitality, 1, "the first row buys vitality");
        g.route = 0;
        click(&mut g, 661., 485.);
        assert_eq!(g.route, 1, "the destination switches");
        click(&mut g, 640., 560.);
        assert_eq!(
            (g.screen, g.level.biome),
            (Screen::Playing, world::Biome::Foundry)
        );
        g.screen = Screen::Paused;
        g.request_abandon();
        g.request_abandon();
        assert_eq!(g.screen, Screen::Dead);
        click(&mut g, 640., 600.);
        assert_eq!(g.screen, Screen::Dead, "the recap holds for a second");
        g.result_time = game::RESULT_DELAY;
        click(&mut g, 640., 600.);
        assert_eq!(g.screen, Screen::Playing);
        // A click in play is a strike, not a menu choice.
        g.screen = Screen::Playing;
        click(&mut g, 354., 515.);
        assert_eq!(g.screen, Screen::Playing);
    }

    /// A point that clicks a target on the current screen: its centre, or the
    /// first point across it that isn't covered by another target.
    fn centre(g: &Game, click: ui::Click) -> Vec2 {
        let (rect, _) = ui::targets(g)
            .into_iter()
            .find(|(_, c)| *c == click)
            .unwrap_or_else(|| panic!("{click:?} isn't on the {:?} screen", g.screen));
        let y = rect.center().y;
        std::iter::once(rect.center())
            .chain((1..20).map(|i| vec2(rect.x + rect.w * i as f32 / 20., y)))
            .find(|p| ui::click_at(g, *p) == Some(click))
            .unwrap_or_else(|| panic!("{click:?} is hidden by other targets"))
    }

    #[test]
    fn the_mouse_works_the_options_and_controls_pages() {
        use ui::Click::*;
        let none = |_: KeyCode| false;
        let pad = pad::Pad::new();
        let at = |g: &mut Game, point: Vec2, last: Option<KeyCode>| {
            let pressed = |k: KeyCode| Some(k) == last;
            let keys = MenuKeys {
                pressed: if last.is_some() { &pressed } else { &none },
                last,
                click: Some(point).filter(|_| last.is_none()),
                hover: None,
                repeat: None,
            };
            menus(g, &keys, &pad);
        };
        let click = |g: &mut Game, target: ui::Click| {
            let point = centre(g, target);
            at(g, point, None)
        };
        let mut g = Game::new(4017, save::Save::default());
        g.screen = Screen::Paused;
        g.open_options();
        // Clicking a bar's name selects it; its bar sets the level.
        click(&mut g, Row(2));
        assert_eq!((g.options_row, g.settings.shake), (2, 10));
        click(&mut g, Level(0, 3));
        assert_eq!((g.options_row, g.settings.music), (0, 3));
        assert_eq!(g.sounds.pop(), Some(Sfx::Select));
        // The selected bar's arrows step it, down to nothing.
        for _ in 0..4 {
            click(&mut g, Step(0, -1));
        }
        assert_eq!(g.settings.music, 0);
        click(&mut g, Step(0, 1));
        assert_eq!(g.settings.music, 1);
        click(&mut g, Level(8, 1));
        assert_eq!(g.settings.speed, settings::Settings::SLOWEST);
        // Graphics fidelity: a click on a step chooses it, the arrows step it,
        // and a click on its name only selects it.
        let fidelity_row = settings::Settings::FIDELITY_ROW;
        click(&mut g, Row(fidelity_row));
        assert_eq!(g.settings.fidelity, fidelity::Fidelity::High);
        click(&mut g, Level(fidelity_row, 0));
        assert_eq!(
            (g.settings.fidelity, g.settings.postfx),
            (fidelity::Fidelity::Low, false)
        );
        click(&mut g, Level(fidelity_row, 3));
        assert_eq!(g.settings.fidelity, fidelity::Fidelity::Ultra);
        click(&mut g, Step(fidelity_row, -1));
        assert_eq!(g.settings.fidelity, fidelity::Fidelity::High);
        // A switch flips on a click anywhere on its row.
        click(&mut g, Row(3));
        assert!(!g.settings.hitstop);
        click(&mut g, Row(3));
        assert!(g.settings.hitstop);
        click(&mut g, Row(settings::Settings::CONTROLS_ROW));
        assert_eq!(g.screen, Screen::Controls);
        // A click on an action listens for its key; a key binds it.
        click(&mut g, Row(2));
        assert!(g.rebinding && g.controls_row == 2);
        at(&mut g, Vec2::ZERO, Some(KeyCode::G));
        assert!(!g.rebinding);
        assert_eq!(g.settings.keys.label(controls::Action::Jump), "G");
        // A click while listening cancels; mouse buttons can't be bound.
        click(&mut g, Row(4));
        assert!(g.rebinding);
        at(&mut g, vec2(10., 10.), None);
        assert!(!g.rebinding);
        click(&mut g, Row(controls::Action::ALL.len()));
        assert_eq!(g.settings.keys, controls::Bindings::default());
        click(&mut g, Back);
        assert_eq!(g.screen, Screen::Options);
        click(&mut g, Back);
        assert_eq!(g.screen, Screen::Paused);
    }

    #[test]
    fn the_mouse_reaches_the_title_and_pause_links() {
        use ui::Click::*;
        let none = |_: KeyCode| false;
        let pad = pad::Pad::new();
        let click = |g: &mut Game, target: ui::Click| {
            let keys = MenuKeys {
                pressed: &none,
                last: None,
                click: Some(centre(g, target)),
                hover: None,
                repeat: None,
            };
            menus(g, &keys, &pad);
        };
        let has = |g: &Game, target| ui::targets(g).iter().any(|(_, c)| *c == target);
        let mut g = Game::new(4017, save::Save::default());
        assert!(!has(&g, NewRun), "no new descent without a run to continue");
        click(&mut g, Mute);
        assert!(g.settings.muted);
        click(&mut g, Mute);
        assert!(!g.settings.muted);
        click(&mut g, Options);
        assert_eq!(g.screen, Screen::Options);
        click(&mut g, Back);
        assert_eq!(g.screen, Screen::Title);
        click(&mut g, Quit);
        assert!(g.quit_armed && !g.quit);
        click(&mut g, Quit);
        assert!(g.quit, "a second click on Confirm quit quits");
        g.quit = false;
        g.quit_armed = false;
        // New descent replaces a run there is to continue.
        g.resume = Some(g.checkpoint());
        click(&mut g, NewRun);
        assert_eq!(g.screen, Screen::Playing);
        assert!(g.resume.is_none());
        g.screen = Screen::Paused;
        click(&mut g, Options);
        assert_eq!(g.screen, Screen::Options);
        click(&mut g, Back);
        click(&mut g, Quit);
        assert!(g.quit_armed);
        click(&mut g, Abandon);
        assert!(
            g.abandon_armed && !g.quit_armed,
            "one confirmation at a time"
        );
        click(&mut g, Abandon);
        assert_eq!(g.screen, Screen::Dead);
        // Controllers see no mute link.
        g.screen = Screen::Paused;
        g.pad_prompts = true;
        assert!(!has(&g, Mute) && has(&g, Abandon));
    }

    #[test]
    fn the_pointer_highlights_until_a_key_or_controller_is_used() {
        let mut g = Game::new(4017, save::Save::default());
        let button = centre(&g, ui::Click::Confirm);
        assert_eq!(cursor_for(&g), Cursor::Default, "no pointer yet");
        let moved = Devices {
            moved: true,
            ..Default::default()
        };
        follow_devices(&mut g, moved, button);
        assert_eq!(g.pointer, Some(button));
        assert_eq!(cursor_for(&g), Cursor::Pointer, "a hand over the button");
        follow_devices(&mut g, Devices::default(), vec2(900., 300.));
        assert_eq!(g.pointer, Some(button), "a still mouse keeps its place");
        let typed = Devices {
            typed: true,
            ..Default::default()
        };
        follow_devices(&mut g, typed, button);
        assert_eq!((g.pointer, cursor_for(&g)), (None, Cursor::Default));
        let moved = Devices {
            moved: true,
            ..Default::default()
        };
        follow_devices(&mut g, moved, vec2(900., 300.));
        assert_eq!(cursor_for(&g), Cursor::Default, "nothing to click there");
        let pad = Devices {
            pad: true,
            ..Default::default()
        };
        follow_devices(&mut g, pad, button);
        assert!(g.pointer.is_none() && g.pad_prompts);
    }

    #[test]
    fn a_settled_window_size_is_saved_once() {
        let mut watch = WindowWatch::default();
        let frames = |watch: &mut WindowWatch, size, fullscreen, saved, seconds: f32| {
            (0..(seconds * 60.) as usize)
                .filter_map(|_| watch.watch(size, fullscreen, 1. / 60., saved))
                .collect::<Vec<_>>()
        };
        let default = Some(settings::Settings::DEFAULT_WINDOW);
        assert!(
            frames(&mut watch, [1280, 720], false, None, 3.).is_empty(),
            "the default isn't written"
        );
        // Dragging an edge: each new size starts the wait again.
        for w in (1300..1600).step_by(20) {
            assert!(frames(&mut watch, [w, 900], false, default, 0.25).is_empty());
        }
        assert_eq!(
            frames(&mut watch, [1600, 900], false, default, 3.),
            vec![[1600, 900]]
        );
        assert!(frames(&mut watch, [1600, 900], false, Some([1600, 900]), 3.).is_empty());
        // Fullscreen is never saved; leaving it waits for the window to settle.
        let saved = Some([1600, 900]);
        assert!(frames(&mut watch, [3840, 2160], true, saved, 3.).is_empty());
        assert!(frames(&mut watch, [3840, 2160], false, saved, 0.1).is_empty());
        assert!(
            frames(&mut watch, [1600, 900], false, saved, 3.).is_empty(),
            "back to the saved size"
        );
        assert!(
            frames(&mut watch, [500, 300], false, saved, 3.).is_empty(),
            "too small to keep"
        );
    }

    #[test]
    fn the_window_size_flag_applies_only_to_testing_modes() {
        let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            window_size_flag(&args(&[
                "c",
                "--fidelity-bench",
                "--window-size",
                "1920x1080"
            ])),
            Some([1920, 1080])
        );
        assert_eq!(
            window_size_flag(&args(&["c", "--window-size", "1920x1080"])),
            None,
            "play keeps its saved size"
        );
        for bad in ["300x200", "1920", "wide"] {
            assert_eq!(
                window_size_flag(&args(&["c", "--ui-gallery", "--window-size", bad])),
                None
            );
        }
    }

    #[test]
    fn isolating_flags_match_the_practice_modes() {
        let args = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(!isolated(&args(&["cinderwake"])));
        assert!(!isolated(&args(&["cinderwake", "--snapshot-every", "1"])));
        for flag in ISOLATING_FLAGS {
            assert!(isolated(&args(&["cinderwake", flag])), "{flag}");
        }
    }

    #[test]
    fn menu_hints_follow_the_device_used_last() {
        let mut g = Game::new(4017, save::Save::default());
        g.stage = 0;
        let device = |pad, typed, moved| Devices {
            pad,
            typed,
            moved,
            ..Default::default()
        };
        let hints = |g: &Game| {
            [
                ui::options_footer(g),
                ui::controls_footer(g),
                ui::route_hint(g),
            ]
        };
        let keys = hints(&g);
        assert!(keys[0].starts_with("W / S  choose") && keys[0].contains("M  mute"));
        assert!(keys[1].contains("ENTER  rebind or restore"));
        assert!(keys[2].starts_with("A / D"));
        follow_devices(&mut g, device(false, false, true), vec2(640., 360.));
        let mouse = hints(&g);
        for hint in &mouse {
            assert!(hint.starts_with("CLICK"), "{hint}");
        }
        assert!(mouse[0].contains("M  mute"), "the mute key still works");
        follow_devices(&mut g, device(true, false, false), vec2(640., 360.));
        let pad = hints(&g);
        assert!(pad[0].starts_with("D-PAD  choose") && !pad[0].contains("mute"));
        assert!(pad[1].contains("A  rebind"));
        follow_devices(&mut g, device(false, false, true), vec2(600., 300.));
        assert!(hints(&g)[1].starts_with("CLICK"), "the mouse again");
        follow_devices(&mut g, device(false, true, false), vec2(600., 300.));
        assert_eq!(hints(&g), keys, "a key press brings the keys back");
    }

    #[test]
    fn the_cursor_hides_only_in_play() {
        let mut g = Game::new(4017, save::Save::default());
        g.pointer = Some(vec2(900., 300.));
        assert_eq!(cursor_for(&g), Cursor::Default, "shown on the title");
        g.start();
        assert_eq!(cursor_for(&g), Cursor::Hidden);
        g.map = true;
        assert_eq!(cursor_for(&g), Cursor::Hidden, "the atlas is still play");
        g.map = false;
        for screen in [
            Screen::Paused,
            Screen::Options,
            Screen::Controls,
            Screen::Scroll,
            Screen::Reliquary,
            Screen::Camp,
            Screen::Dead,
            Screen::Victory,
        ] {
            g.screen = screen;
            assert_ne!(cursor_for(&g), Cursor::Hidden, "shown on {screen:?}");
        }
        g.screen = Screen::Playing;
        g.focus_lost();
        assert_eq!(cursor_for(&g), Cursor::Default, "back when focus pauses");
    }

    #[test]
    fn pointing_at_a_row_selects_it_only_while_the_mouse_moves() {
        let none = |_: KeyCode| false;
        let pad = pad::Pad::new();
        let mut g = Game::new(4017, save::Save::default());
        g.open_options();
        let hover = |g: &mut Game, at: Option<Vec2>| {
            let keys = MenuKeys {
                pressed: &none,
                last: None,
                click: None,
                hover: at,
                repeat: None,
            };
            menus(g, &keys, &pad);
        };
        let row = centre(&g, ui::Click::Row(5));
        hover(&mut g, Some(row));
        assert_eq!(g.options_row, 5);
        g.options_row = 1;
        hover(&mut g, None);
        assert_eq!(
            g.options_row, 1,
            "a still pointer leaves the keyboard's row"
        );
        let bar = centre(&g, ui::Click::Level(8, 4));
        hover(&mut g, Some(bar));
        assert_eq!(g.options_row, 8, "pointing at a bar selects its row");
        assert_eq!(g.settings.speed, 10, "without changing it");
        g.open_controls();
        let action = centre(&g, ui::Click::Row(7));
        hover(&mut g, Some(action));
        assert_eq!(g.controls_row, 7);
        g.rebinding = true;
        let other = centre(&g, ui::Click::Row(3));
        hover(&mut g, Some(other));
        assert_eq!(g.controls_row, 7, "listening for a key holds the row");
    }

    #[test]
    fn clicks_map_through_the_letterbox() {
        // A 1500 x 700 window pillarboxes a 1244.4 x 700 frame.
        let (frame, _) = letterbox(1500., 700., 1.25);
        let centre = to_interface(frame, vec2(750., 350.));
        assert!((centre - vec2(640., 360.)).length() < 0.01);
        let corner = to_interface(frame, vec2(frame.x, frame.y + frame.h));
        assert!((corner - vec2(0., 720.)).length() < 0.01);
    }

    #[test]
    fn hud_viewport_covers_the_physical_frame_at_any_dpi() {
        // A 1280 x 720 framebuffer at 1.25x reports a 1024 x 576 logical screen.
        let (rect, viewport) = letterbox(1024., 576., 1.25);
        assert_eq!(rect, Rect::new(0., 0., 1024., 576.));
        assert_eq!(viewport, (0, 0, 1280, 720));
        let (_, viewport) = letterbox(1280., 720., 1.);
        assert_eq!(viewport, (0, 0, 1280, 720));
        // Retina: pillarboxed logical window, doubled physical viewport.
        let (rect, viewport) = letterbox(1600., 720., 2.);
        assert_eq!(rect, Rect::new(160., 0., 1280., 720.));
        assert_eq!(viewport, (320, 0, 2560, 1440));
    }

    #[test]
    fn environment_tour_covers_full_width_of_every_biome() {
        let mut g = Game::new(4017, save::Save::default());
        g.practice = true;
        let expected = [
            world::Biome::Aqueduct,
            world::Biome::Garden,
            world::Biome::Foundry,
            world::Biome::Crown,
        ];
        for (index, biome) in expected.into_iter().enumerate() {
            let start = index as u32 * TOUR_FRAMES_PER_BIOME;
            prepare_tour_frame(&mut g, start);
            assert_eq!(g.level.biome, biome);
            assert_eq!(g.camera, 0.);
            assert!(g.level.enemies.is_empty());
            prepare_tour_frame(&mut g, start + TOUR_FRAMES_PER_BIOME - 1);
            assert_eq!(g.camera, g.level.width - 640.);
            assert!(g.player.pos.x > g.camera && g.player.pos.x < g.camera + 640.);
            assert!(g.practice);
        }
    }

    #[test]
    fn motion_capture_inputs_reach_all_requested_actions() {
        let mut g = Game::new(4017, save::Save::default());
        g.practice = true;
        g.start();
        g.player.hp = g.player.max_hp * 0.45;
        let mut seen = [false; 11];
        for frame in 0..MOTION_FRAMES {
            let mut input = motion_input(frame);
            for _ in 0..2 {
                let was_slamming = g.player.slam;
                g.tick(world::STEP, input);
                seen[0] |= g.player.heal_time > 0.;
                seen[1] |= g.player.parry > 0.;
                seen[2] |= g.player.dodge > 0.;
                seen[3] |= g.player.attack > 0.;
                seen[4] |= g.player.bow_cd > 0.;
                seen[5] |= g.player.jumps >= 2;
                seen[6] |= g.player.grenade_cd > 0.;
                seen[7] |= g.player.trap_cd > 0.;
                seen[8] |= g.player.ground && g.player.vel.x.abs() > 18.;
                seen[9] |= g.player.slam && !g.player.ground;
                seen[10] |= was_slamming && !g.player.slam && g.player.ground;
                input.jump = false;
                input.dodge = false;
                input.parry = false;
                input.grenade = false;
                input.trap = false;
                input.heal = false;
            }
        }
        assert!(
            seen.into_iter().all(|action| action),
            "missing actions: {seen:?}"
        );
        assert_eq!(g.screen, Screen::Playing, "capture must remain in gameplay");
        assert_eq!(g.player.flasks, 1, "opening heal must complete");
    }
}
