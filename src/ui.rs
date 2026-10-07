//! Live interface composed over generated frames and item art.
use crate::{
    controls::{Action, Menu},
    game::*,
    render::{c, INK},
    settings::{RowValue, Settings},
    ui_skin::Skin,
    world::*,
};
use macroquad::prelude::*;

const GOLD: u32 = 0xf1d29c;
const PALE: u32 = 0xe1e7de;
const MUTED: u32 = 0x98b4b4;
const TEAL: u32 = 0x85dfcc;
/// Warnings: confirmations, refusals, and a lost controller.
const WARN: u32 = 0xef9c81;
/// Only the desktop game can close itself; browsers close the tab.
const QUIT: bool = cfg!(not(target_arch = "wasm32"));

// Menu areas a mouse click can choose. Drawing uses the same rectangles.
const TITLE_BUTTON: Rect = Rect::new(145., 483., 418., 64.);
const PAUSE_BUTTON: Rect = Rect::new(423., 493., 434., 57.);
const RESULT_BUTTON: Rect = Rect::new(443., 572., 394., 63.);
const CAMP_BUTTON: Rect = Rect::new(397., 531., 486., 63.);
/// The Keeper's destination line; a click switches the route.
const CAMP_ROUTE: Rect = Rect::new(360., 462., 600., 44.);
fn reliquary_card(i: usize) -> Rect {
    Rect::new(290. + i as f32 * 370., 300., 330., 280.)
}
fn memory_card(i: usize) -> Rect {
    Rect::new(128. + i as f32 * 350., 300., 324., 267.)
}
fn keeper_row(i: usize) -> Rect {
    Rect::new(222., 224. + i as f32 * 65., 836., 59.)
}
const OPTIONS_BACK: Rect = Rect::new(483., 513., 314., 50.);
const CONTROLS_BACK: Rect = Rect::new(483., 516., 314., 46.);
const CONTROLS_RESET: Rect = Rect::new(500., 446., 280., 30.);
fn options_row(row: usize) -> Rect {
    Rect::new(300., 224. + row as f32 * 26., 680., 26.)
}
/// One step of an options bar, widened to cover the gaps beside it.
fn options_segment(row: usize, i: usize) -> Rect {
    Rect::new(637.5 + i as f32 * 24., 224. + row as f32 * 26., 24., 26.)
}
/// The selected bar's < and > arrows.
fn options_arrow(row: usize, up: bool) -> Rect {
    Rect::new(
        if up { 880. } else { 609. },
        224. + row as f32 * 26.,
        26.,
        26.,
    )
}
fn controls_row(i: usize) -> Rect {
    Rect::new(
        294. + (i / 6) as f32 * 345.,
        236. + (i % 6) as f32 * 34.,
        335.,
        30.,
    )
}

/// What a left click on a menu means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// The screen's main button: begin, continue, rise again, or travel.
    Confirm,
    /// Resume from the pause screen.
    Resume,
    /// A numbered card or row.
    Choice(usize),
    /// Switch the Keeper's route.
    Route,
    /// Leave the options or controls page.
    Back,
    /// An options or controls row. It's selected, and switches, the
    /// controls row, an action, and the restore row act as Enter would.
    Row(usize),
    /// Sets an options bar to a level from 1 to 10.
    Level(usize, u8),
    /// Steps the selected options bar down (-1) or up (1).
    Step(usize, i8),
    /// Footer links on the title and pause screens.
    Options,
    NewRun,
    Abandon,
    Quit,
    Mute,
}
/// Footer links on the title and pause screens, in fixed-width slots so a
/// click target never depends on measuring text.
fn links(g: &Game) -> Vec<(Rect, Click)> {
    let pad = g.prompts().pad();
    let mut items = vec![];
    let (cx, y, width) = match g.screen {
        Screen::Title => {
            if g.resume.is_some() {
                items.push(Click::NewRun);
            }
            items.push(Click::Options);
            (354., 627., 140.)
        }
        Screen::Paused => {
            items.extend([Click::Options, Click::Abandon]);
            (640., 560., 170.)
        }
        _ => return vec![],
    };
    // Controllers have no mute button.
    if !pad {
        items.push(Click::Mute);
    }
    if QUIT {
        // Quit last on the title, before mute when paused, as before.
        let at = if g.screen == Screen::Paused && !pad {
            items.len() - 1
        } else {
            items.len()
        };
        items.insert(at, Click::Quit);
    }
    let left = cx - width * items.len() as f32 / 2.;
    items
        .into_iter()
        .enumerate()
        .map(|(i, click)| (Rect::new(left + width * i as f32, y, width, 24.), click))
        .collect()
}
/// Every click target on the current screen, in interface coordinates
/// (1280 × 720). The first that contains a point wins.
pub fn targets(g: &Game) -> Vec<(Rect, Click)> {
    let mut targets = vec![];
    targets.extend(links(g));
    match g.screen {
        Screen::Title => targets.push((TITLE_BUTTON, Click::Confirm)),
        Screen::Paused => targets.push((PAUSE_BUTTON, Click::Resume)),
        Screen::Dead | Screen::Victory if g.result_ready() => {
            targets.push((RESULT_BUTTON, Click::Confirm))
        }
        Screen::Reliquary => targets.extend((0..2).map(|i| (reliquary_card(i), Click::Choice(i)))),
        Screen::Scroll => targets.extend((0..3).map(|i| (memory_card(i), Click::Choice(i)))),
        Screen::Camp => {
            targets.extend((0..3).map(|i| (keeper_row(i), Click::Choice(i))));
            targets.push((CAMP_BUTTON, Click::Confirm));
            if g.stage == 0 {
                targets.push((CAMP_ROUTE, Click::Route));
            }
        }
        Screen::Options => {
            let row = g.options_row;
            if let RowValue::Level(_) = g.settings.row(row).1 {
                targets.push((options_arrow(row, false), Click::Step(row, -1)));
                targets.push((options_arrow(row, true), Click::Step(row, 1)));
            }
            for row in 0..Settings::ROWS {
                if let RowValue::Level(_) = g.settings.row(row).1 {
                    targets.extend(
                        (0..10).map(|i| (options_segment(row, i), Click::Level(row, i as u8 + 1))),
                    );
                }
                targets.push((options_row(row), Click::Row(row)));
            }
            targets.push((OPTIONS_BACK, Click::Back));
        }
        Screen::Controls => {
            let actions = Action::ALL.len();
            targets.extend((0..actions).map(|i| (controls_row(i), Click::Row(i))));
            targets.push((CONTROLS_RESET, Click::Row(actions)));
            targets.push((CONTROLS_BACK, Click::Back));
        }
        _ => {}
    }
    targets
}
/// The menu target under a point in interface coordinates (1280 × 720).
/// Whether the mouse was the last thing used, so hints describe clicks. A
/// key press or controller input clears the pointer.
fn mouse_last(g: &Game) -> bool {
    g.pointer.is_some()
}
/// The options page's footer, for the device used last.
pub fn options_footer(g: &Game) -> String {
    let p = g.prompts();
    if mouse_last(g) {
        "CLICK  a switch to flip it, a bar to set it, < > to step it      M  mute all sound".into()
    } else if p.pad() {
        format!(
            "{}  choose      {}  adjust",
            p.menu(Menu::Rows),
            p.menu(Menu::Adjust)
        )
    } else {
        format!(
            "{}  choose      {}  adjust      M  mute all sound",
            p.menu(Menu::Rows),
            p.menu(Menu::Adjust)
        )
    }
}
/// The controls page's footer, for the device used last.
pub fn controls_footer(g: &Game) -> String {
    let p = g.prompts();
    if mouse_last(g) {
        "CLICK  an action to rebind it, or restore the default keys".into()
    } else {
        format!(
            "{}  choose      {}  rebind or restore",
            p.menu(Menu::Rows),
            p.menu(Menu::Confirm)
        )
    }
}
/// The Keeper's line above the route, for the device used last.
pub fn route_hint(g: &Game) -> String {
    if mouse_last(g) {
        "CLICK   the destination to change it".into()
    } else {
        format!(
            "{}   Choose your next destination",
            g.prompts().menu(Menu::Adjust)
        )
    }
}
pub fn click_at(g: &Game, at: Vec2) -> Option<Click> {
    targets(g)
        .into_iter()
        .find(|(rect, _)| rect.contains(at))
        .map(|(_, click)| click)
}

pub struct Ui {
    font: Option<Font>,
    skin: Skin,
}
impl Ui {
    pub fn new() -> Self {
        let font = load_ttf_font_from_bytes(include_bytes!("../assets/title.ttf")).ok();
        // Cache all UI glyph sizes before issuing draws. Growing Macroquad's font
        // atlas mid-frame can invalidate earlier text when switching menu screens.
        let ascii: String = (32u8..=126).map(char::from).collect();
        for size in 10..=24 {
            measure_text(&ascii, None, size, 1.);
        }
        for size in [31, 35, 37, 43, 48, 75] {
            measure_text(&ascii, font.as_ref(), size, 1.);
        }
        Self {
            font,
            skin: Skin::new(),
        }
    }
    fn text(&self, s: &str, x: f32, y: f32, size: f32, col: Color) {
        // A small dark shadow keeps live glyphs crisp over textured frames.
        draw_text_ex(
            s,
            x + 1.,
            y + 1.,
            TextParams {
                font_size: size as u16,
                color: INK,
                ..Default::default()
            },
        );
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font_size: size as u16,
                color: col,
                ..Default::default()
            },
        );
    }
    fn title(&self, s: &str, x: f32, y: f32, size: f32, col: Color) {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: self.font.as_ref(),
                font_size: size as u16,
                color: col,
                ..Default::default()
            },
        );
    }
    fn centered_at(&self, s: &str, cx: f32, y: f32, size: f32, col: Color) {
        let d = measure_text(s, None, size as u16, 1.);
        self.text(s, cx - d.width / 2., y, size, col);
    }
    fn center(&self, s: &str, y: f32, size: f32, col: Color) {
        self.centered_at(s, 640., y, size, col);
    }
    fn heading(&self, s: &str, cx: f32, y: f32, size: f32) {
        let d = measure_text(s, self.font.as_ref(), size as u16, 1.);
        self.title(s, cx - d.width / 2., y, size, c(GOLD));
    }
    fn key(&self, key: &str, x: f32, y: f32) {
        let width = (measure_text(key, None, 14, 1.).width + 15.).max(25.);
        self.skin.slot(Rect::new(x, y, width, 24.));
        self.centered_at(key, x + width / 2., y + 17., 14., c(GOLD));
    }
    fn button(&self, text: &str, rect: Rect) {
        self.skin.plaque(rect);
        self.centered_at(
            text,
            rect.x + rect.w / 2.,
            rect.y + rect.h / 2. + 7.,
            21.,
            c(GOLD),
        );
    }
    fn title_screen(&self, g: &Game) {
        draw_rectangle(0., 0., 1280., 720., INK.with_alpha(0.32));
        self.skin.panel(Rect::new(48., 48., 612., 624.));
        self.skin.crest(Rect::new(266., 73., 176., 126.));
        self.centered_at("A CLOCKWORK ROGUELITE", 354., 222., 15., c(TEAL));
        self.heading("CINDERWAKE", 354., 294., 75.);
        self.heading("The sun is broken.", 354., 350., 31.);
        self.heading("Carry what remains.", 354., 386., 31.);
        self.centered_at(
            "Master the blade. Bank your embers.",
            354.,
            430.,
            18.,
            c(PALE),
        );
        self.centered_at("Defy the Regent.", 354., 455., 18., c(PALE));
        let record = format!(
            "{} {}   /   {} embers kept{}",
            g.save.runs,
            if g.save.runs == 1 {
                "descent"
            } else {
                "descents"
            },
            g.save.embers,
            g.save.best_time.map_or(String::new(), |t| format!(
                "   /   fastest win {}",
                clock(t)
            ))
        );
        let p = g.prompts();
        if let Some(run) = &g.resume {
            self.button(
                &format!("{}   Continue the descent", p.menu(Menu::Confirm)),
                TITLE_BUTTON,
            );
            self.centered_at(
                &format!(
                    "{}  /  {}",
                    run.biome.name(),
                    if run.at_keeper {
                        "AT THE KEEPER".to_string()
                    } else {
                        format!("STAGE {} OF 3", run.stage + 1)
                    },
                ),
                354.,
                574.,
                15.,
                c(TEAL),
            );
            self.centered_at(&record, 354., 596., 14., c(MUTED));
        } else {
            self.button(
                &format!("{}   Begin the descent", p.menu(Menu::Confirm)),
                TITLE_BUTTON,
            );
            self.centered_at(&record, 354., 579., 16., c(TEAL));
        }
        if g.quit_armed {
            self.centered_at(
                &format!("Press {} again to quit.", p.menu(Menu::QuitTitle)),
                354.,
                618.,
                15.,
                c(WARN),
            );
        } else {
            self.centered_at(
                &{
                    let k = |a| p.action(a);
                    format!(
                        "{}  Move   {}  Jump   {}  Strike   {}  Dodge   {}  Parry",
                        p.movement(),
                        k(Action::Jump),
                        k(Action::Strike),
                        k(Action::Dodge),
                        k(Action::Parry)
                    )
                },
                354.,
                618.,
                15.,
                c(MUTED),
            );
        }
        self.links(g, 14.);
        self.unreadable_notice(g);
    }
    /// The footer links, each centred in its slot.
    fn links(&self, g: &Game, size: f32) {
        let p = g.prompts();
        for (rect, click) in links(g) {
            let (key, label, armed) = match click {
                Click::NewRun => (p.menu(Menu::NewRun), "New descent", false),
                Click::Options => (p.menu(Menu::Options), "Options", false),
                Click::Abandon if g.abandon_armed => {
                    (p.menu(Menu::Abandon), "Confirm abandon", true)
                }
                Click::Abandon => (p.menu(Menu::Abandon), "Abandon run", false),
                Click::Quit => {
                    let key = if g.screen == Screen::Title {
                        p.menu(Menu::QuitTitle)
                    } else {
                        p.menu(Menu::QuitPaused)
                    };
                    if g.quit_armed {
                        (key, "Confirm quit", true)
                    } else {
                        (key, "Quit", false)
                    }
                }
                Click::Mute => match (g.screen, g.settings.muted) {
                    (Screen::Title, false) => ("M", "Mute", false),
                    (Screen::Title, true) => ("M", "Unmute", false),
                    (_, false) => ("M", "Sound is on", false),
                    (_, true) => ("M", "Sound is muted", false),
                },
                _ => continue,
            };
            self.centered_at(
                &format!("{key}  {label}"),
                rect.x + rect.w / 2.,
                rect.y + 17.,
                size,
                c(if armed { WARN } else { MUTED }),
            );
        }
    }
    /// Names any saved file that couldn't be read at launch and where its
    /// bytes were kept, so a damaged save is never replaced without a word.
    fn unreadable_notice(&self, g: &Game) {
        if g.unreadable.is_empty() {
            return;
        }
        let height = 84. + 58. * g.unreadable.len() as f32;
        let rect = Rect::new(700., 672. - height, 540., height);
        self.skin.panel(rect);
        self.heading("A saved file couldn't be read", 970., rect.y + 52., 31.);
        for (i, damaged) in g.unreadable.iter().enumerate() {
            let y = rect.y + 84. + 58. * i as f32;
            let lost = if damaged.file == crate::settings::Settings::FILE {
                "options are back to their defaults"
            } else {
                "progress starts fresh"
            };
            self.centered_at(
                &format!("{} couldn't be read, so {lost}.", damaged.file),
                970.,
                y,
                15.,
                c(PALE),
            );
            let (kept, color) = match &damaged.kept {
                Ok(copy) => (format!("The damaged file is kept as {copy}."), c(TEAL)),
                Err(e) => (format!("It couldn't be copied aside: {e}"), c(0xef9c81)),
            };
            self.centered_at(&kept, 970., y + 22., 15., color);
        }
    }
    fn hud(&self, g: &Game) {
        let p = &g.player;
        let prompts = g.prompts();
        // Individual framed clusters leave the central play field unobstructed.
        self.skin.panel(Rect::new(16., 16., 322., 80.));
        self.skin.crest(Rect::new(26., 26., 62., 58.));
        self.text("VITALITY", 99., 40., 15., c(MUTED));
        self.text(
            &format!("{} / {}", p.hp.max(0.).ceil() as u32, p.max_hp as u32),
            207.,
            40.,
            18.,
            c(PALE),
        );
        let ratio = (p.hp / p.max_hp).clamp(0., 1.);
        self.skin.gauge(
            Rect::new(89., 50., 236., 33.),
            ratio,
            c(if ratio < 0.25 { 0xec7465 } else { 0x75c4ac }),
        );
        self.skin.panel(Rect::new(348., 16., 174., 80.));
        self.skin.icon(8, Rect::new(362., 26., 24., 27.), 1.);
        self.skin.icon(9, Rect::new(362., 58., 24., 23.), 1.);
        self.text(&format!("{}", p.embers), 394., 45., 22., c(TEAL));
        self.text(&format!("{}", p.gold), 394., 77., 22., c(GOLD));
        self.text("EMBERS", 448., 44., 12., c(MUTED));
        self.text("COPPER", 448., 76., 12., c(MUTED));
        for (i, color) in [0xef9c81, 0x9ed7eb, 0xb4d89a].into_iter().enumerate() {
            let x = 538. + i as f32 * 46.;
            self.skin.slot(Rect::new(x, 21., 38., 38.));
            self.skin.icon(10 + i, Rect::new(x + 8., 28., 23., 24.), 1.);
            self.centered_at(&p.power[i].to_string(), x + 19., 81., 20., c(color));
        }
        self.text(g.level.biome.name(), 702., 40., 17., c(GOLD));
        self.text(Level::tier_name(p.pos.y), 702., 88., 12., c(TEAL));
        self.text(
            &format!(
                "{:02}:{:02}   /   {} guardians",
                g.run_time as u32 / 60,
                g.run_time as u32 % 60,
                p.kills
            ),
            702.,
            68.,
            16.,
            c(PALE),
        );
        self.skin.panel(Rect::new(1036., 16., 228., 80.));
        self.minimap(g, Rect::new(1054., 27., 192., 44.));
        self.centered_at(
            &format!("{}  VERTICAL ATLAS", prompts.menu(Menu::Atlas)),
            1150.,
            83.,
            11.,
            c(MUTED),
        );

        // Artwork identifies equipment at a glance; bindings and cooldowns stay live.
        let weapon_icon = match p.weapon {
            Weapon::Sabre => 0,
            Weapon::Glaive => 1,
            Weapon::Hammer => 2,
        };
        let slots = [
            (
                prompts.action(Action::Strike),
                p.weapon.name(),
                weapon_icon,
                p.attack_cd,
                p.weapon.delay(),
            ),
            (
                prompts.action(Action::Glassbolt),
                "GLASSBOLT",
                3,
                p.bow_cd,
                0.32,
            ),
            (
                prompts.action(Action::FireVessel),
                "FIRE VESSEL",
                4,
                p.grenade_cd,
                5.,
            ),
            (
                prompts.action(Action::ArcSnare),
                "ARC SNARE",
                5,
                p.trap_cd,
                8.,
            ),
        ];
        for (i, (key, name, icon, cd, max)) in slots.iter().enumerate() {
            let x = 16. + i as f32 * 202.;
            self.skin.panel(Rect::new(x, 638., 194., 72.));
            self.skin.slot(Rect::new(x + 11., 649., 48., 48.));
            self.skin.icon(
                *icon,
                Rect::new(x + 18., 656., 34., 33.),
                if *cd > 0. { 0.5 } else { 1. },
            );
            if *cd > 0. {
                let h = 33. * (*cd / max).clamp(0., 1.);
                draw_rectangle(x + 18., 689. - h, 34., h, INK.with_alpha(0.5));
            }
            self.key(key, x + 22., 682.);
            self.text(name, x + 67., 665., 14., c(PALE));
            let status = if *cd > 0. {
                let rate = if i >= 2 && p.mutation == 2 { 1.35 } else { 1. };
                format!("{:.1}s", (*cd / rate * 10.).ceil() / 10.)
            } else if i == 0 {
                format!("TIER {}", p.tier)
            } else {
                "READY".into()
            };
            self.text(
                &status,
                x + 67.,
                690.,
                16.,
                c(if *cd > 0. { GOLD } else { TEAL }),
            );
        }
        self.skin.panel(Rect::new(824., 638., 190., 72.));
        // At low vitality the flask's slot glows while one is left.
        if crate::render::low_vitality(g) > 0. && p.flasks > 0 && p.heal_time <= 0. {
            let pulse = if g.settings.reduce_flashes {
                0.6
            } else {
                0.4 + 0.6 * (g.time * std::f32::consts::TAU * 1.1).sin().abs()
            };
            let slot = Rect::new(834., 646., 48., 52.);
            draw_rectangle(
                slot.x,
                slot.y,
                slot.w,
                slot.h,
                c(TEAL).with_alpha(0.16 * pulse),
            );
            draw_rectangle_lines(
                slot.x,
                slot.y,
                slot.w,
                slot.h,
                2.,
                c(TEAL).with_alpha(0.8 * pulse),
            );
        }
        self.skin.icon(
            6,
            Rect::new(838., 649., 40., 44.),
            if p.flasks == 0 { 0.4 } else { 1. },
        );
        self.key(prompts.action(Action::Heal), 844., 683.);
        self.text("HEALING FLASK", 886., 665., 14., c(PALE));
        self.text(
            &format!("{} / {}", p.flasks, 2 + g.save.flask),
            888.,
            691.,
            21.,
            c(if p.flasks == 0 { MUTED } else { TEAL }),
        );
        self.skin.panel(Rect::new(1024., 638., 240., 72.));
        self.skin.icon(7, Rect::new(1038., 650., 29., 29.), 1.);
        self.text(
            &format!("{}  PARRY", prompts.action(Action::Parry)),
            1077.,
            669.,
            15.,
            c(PALE),
        );
        self.skin.icon(15, Rect::new(1148., 650., 27., 28.), 1.);
        self.text(prompts.action(Action::Dodge), 1184., 668., 14., c(PALE));
        self.text("DODGE", 1184., 684., 11., c(MUTED));
        self.text(
            &format!("{}  PAUSE", prompts.menu(Menu::Pause)),
            1077.,
            695.,
            13.,
            c(MUTED),
        );
    }
    fn prompt(&self, text: &str, y: f32) {
        let width = (measure_text(text, None, 16, 1.).width + 64.).max(250.);
        self.skin
            .plaque(Rect::new(640. - width / 2., y - 25., width, 38.));
        self.center(text, y, 16., c(GOLD));
    }
    fn modal(&self, rect: Rect, heading: &str, y: f32) {
        self.skin.panel(rect);
        self.skin.crest(Rect::new(605., rect.y + 18., 70., 50.));
        self.heading(heading, 640., y, 37.);
    }
    /// Highlights the target under the mouse pointer, using the same
    /// rectangle a click would.
    fn hover(&self, g: &Game) {
        let Some((rect, click)) = g
            .pointer
            .and_then(|at| targets(g).into_iter().find(|(r, _)| r.contains(at)))
        else {
            return;
        };
        let glow = c(TEAL).with_alpha(0.14);
        match click {
            // Plaques: a glow between the frame's end caps.
            Click::Confirm | Click::Resume | Click::Back => draw_rectangle(
                rect.x + rect.h,
                rect.y + 9.,
                rect.w - rect.h * 2.,
                rect.h - 18.,
                glow,
            ),
            _ => {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, glow);
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2., c(TEAL).with_alpha(0.6));
            }
        }
    }
    /// Low vitality: a red tint from the screen's edges, drawn under the HUD.
    fn low_vitality(&self, g: &Game) {
        let level = crate::render::low_vitality(g);
        if level <= 0. {
            return;
        }
        let edge = crate::render::low_vitality_tint(level, g.time, g.settings.reduce_flashes);
        let red = c(0xb0232c);
        // Twelve bands, each fading toward the centre.
        for i in 0..12 {
            let d = i as f32 * 9.;
            let a = edge * (1. - i as f32 / 12.).powf(1.6);
            let col = red.with_alpha(a);
            draw_rectangle(d, d, 1280. - 2. * d, 9., col);
            draw_rectangle(d, 711. - d, 1280. - 2. * d, 9., col);
            draw_rectangle(d, d + 9., 9., 702. - 2. * d, col);
            draw_rectangle(1271. - d, d + 9., 9., 702. - 2. * d, col);
        }
    }
    /// A threat out of view: a disc on the play area's edge with an arrow
    /// toward it, and the windup's "!" in its warning colour.
    fn threat_marker(&self, m: &crate::render::Marker, t: f32, steady: bool) {
        let (radius, col) = match m.windup {
            Some(progress) => (13., crate::render::warning_colour(progress)),
            None => (8., c(0xff8a5c)),
        };
        let pulse = if steady {
            0.5
        } else {
            (t * 9.).sin() * 0.5 + 0.5
        };
        let at = m.at;
        draw_circle(at.x, at.y, radius + 5. + 4. * pulse, col.with_alpha(0.16));
        draw_circle(at.x, at.y, radius, INK.with_alpha(0.88));
        draw_circle_lines(at.x, at.y, radius, 2., col);
        let side = vec2(-m.toward.y, m.toward.x) * (radius * 0.5);
        let base = at + m.toward * (radius + 1.);
        draw_triangle(
            base + m.toward * radius * 0.75,
            base + side,
            base - side,
            col,
        );
        if m.windup.is_some() {
            draw_rectangle(at.x - 1.5, at.y - 7., 3., 8., col);
            draw_rectangle(at.x - 1.5, at.y + 3., 3., 3., col);
        }
    }
    pub fn draw(&self, g: &Game) {
        let over_title = matches!(g.screen, Screen::Options | Screen::Controls)
            && g.options_from == Screen::Title;
        if g.screen == Screen::Title || over_title {
            self.title_screen(g);
            if over_title {
                draw_rectangle(0., 0., 1280., 720., INK.with_alpha(0.73));
                if g.screen == Screen::Controls {
                    self.controls(g);
                } else {
                    self.options(g);
                }
            }
            self.hover(g);
            return;
        }
        self.low_vitality(g);
        self.hud(g);
        let boss = g
            .level
            .enemies
            .iter()
            .find(|e| e.kind == EnemyKind::Regent && e.hp > 0.);
        if let Some(boss) = boss {
            self.skin.icon(13, Rect::new(444., 107., 30., 26.), 1.);
            self.center("THE BRASS REGENT", 124., 18., c(GOLD));
            self.skin.gauge(
                Rect::new(446., 134., 388., 25.),
                boss.hp / boss.max_hp,
                c(0xd5846d),
            );
        }
        if g.intro > 0. && g.screen == Screen::Playing && boss.is_none() {
            let opacity = (g.intro / 1.2).min(1.);
            self.center(g.level.biome.name(), 145., 23., c(GOLD).with_alpha(opacity));
            self.center(
                g.level.biome.subtitle(),
                174.,
                16.,
                c(PALE).with_alpha(opacity),
            );
        }
        for marker in crate::render::threat_markers(g) {
            self.threat_marker(&marker, g.time, g.settings.reduce_flashes);
        }
        if g.screen == Screen::Playing && !g.map {
            if let Some((hint, _)) = g.hint {
                let text = hint.text(&g.prompts());
                let text = text.as_str();
                // The plaque's ornate end caps need generous padding.
                let width = measure_text(text, None, 16, 1.).width + 176.;
                let rect = Rect::new(640. - width / 2., 172., width, 42.);
                self.skin.plaque(rect);
                self.text("TIP", rect.x + 62., rect.y + 27., 15., c(TEAL));
                self.text(text, rect.x + 110., rect.y + 27., 16., c(PALE));
            }
            if let Some(i) = g.nearby() {
                let action = match g.level.objects[i].kind {
                    ObjectKind::Exit => "RING THE BELLGATE",
                    ObjectKind::Scroll => "CLAIM A MEMORY",
                    ObjectKind::Chest => "OPEN RELIQUARY",
                    ObjectKind::Fountain => "DRINK FROM THE WELL",
                    ObjectKind::Forge => "TEMPER WEAPON / 60 COPPER",
                    ObjectKind::Lore => "READ THE INSCRIPTION",
                    ObjectKind::Secret => "BREAK THE SEAL",
                };
                let key = g.prompts().action(Action::Interact);
                self.prompt(&format!("{key}   {action}"), 587.);
            }
            if g.notice_time > 0. {
                self.center(&g.notice, 620., 15., c(PALE));
            }
        }
        if g.map && g.screen == Screen::Playing {
            draw_rectangle(0., 103., 1280., 530., INK.with_alpha(0.67));
            self.skin.panel(Rect::new(88., 108., 1104., 524.));
            self.heading("Atlas of the dying city", 640., 160., 37.);
            self.center(
                &format!(
                    "{}  /  {}  /  SEED {}{}",
                    crate::environment::zone_name(g.level.biome, g.camera, g.level.width),
                    Level::tier_name(g.player.pos.y),
                    g.level.seed,
                    if g.practice {
                        String::new()
                    } else {
                        format!("  /  {:.0}% SURVEYED", g.survey.fraction() * 100.)
                    }
                ),
                188.,
                15.,
                c(MUTED),
            );
            let map_rect = Rect::new(258., 218., 890., 282.);
            self.minimap(g, map_rect);
            for (y, name, hint) in [
                (Level::UPPER, "UPPER", "GALLERIES"),
                (FLOOR, "SURFACE", "WORKS"),
                (Level::LOWER, "UNDERCROFT", "DEEP ROUTE"),
            ] {
                let py = Self::map_point(g, map_rect, vec2(0., y)).y;
                self.text(name, 120., py - 3., 15., c(GOLD));
                self.text(hint, 120., py + 14., 12., c(MUTED));
            }
            self.center(
                "YOU / TEAL     GATE / GOLD     RELICS / IVORY     WELLS / BLUE     GUARDIANS / CORAL",
                532.,
                14.,
                c(PALE),
            );
            let (jump, down) = (
                g.prompts().action(Action::Jump),
                g.prompts().action(Action::Down),
            );
            self.center(
                &format!("{jump} to climb stairways; press again to double jump  /  {down} + {jump} to drop  /  {down} in the air to slam"),
                563.,
                16.,
                c(PALE),
            );
            self.center(
                if g.practice {
                    "The outlined window tracks your view. All routes are shown; gaps connect the city's tiers."
                } else {
                    "The outlined window tracks your view. Unseen stretches stay dark; the bellgate is always marked."
                },
                586.,
                14.,
                c(MUTED),
            );
            self.center(
                &format!("{}   Return to the descent", g.prompts().menu(Menu::Atlas)),
                612.,
                17.,
                c(TEAL),
            );
        }
        if matches!(
            g.screen,
            Screen::Paused
                | Screen::Scroll
                | Screen::Reliquary
                | Screen::Camp
                | Screen::Dead
                | Screen::Victory
                | Screen::Options
                | Screen::Controls
        ) {
            draw_rectangle(0., 0., 1280., 720., INK.with_alpha(0.73));
            match g.screen {
                Screen::Paused => self.paused(g),
                Screen::Scroll => self.disciplines(g),
                Screen::Reliquary => self.reliquary(g),
                Screen::Camp => self.camp(g),
                Screen::Dead | Screen::Victory => self.result(g),
                Screen::Options => self.options(g),
                Screen::Controls => self.controls(g),
                _ => {}
            }
        }
        self.hover(g);
        if let Some(e) = &g.save_error {
            self.text(&format!("SAVE FAILED: {e}"), 20., 632., 13., RED);
        }
        if g.practice {
            self.text(
                "PRACTICE / PROGRESS NOT SAVED",
                20.,
                if g.map { 105. } else { 627. },
                11.,
                c(MUTED),
            );
        }
    }
    fn paused(&self, g: &Game) {
        self.modal(
            Rect::new(255., 114., 770., 493.),
            if g.pad_lost {
                "Controller disconnected"
            } else {
                "The city can wait"
            },
            221.,
        );
        let p = g.prompts();
        let k = |a| p.action(a).to_string();
        let rows = [
            (p.movement(), "Move", k(Action::Jump), "Double jump"),
            (
                k(Action::Strike),
                "Strike",
                k(Action::Glassbolt),
                "Glassbolt",
            ),
            (k(Action::Dodge), "Dodge", k(Action::Parry), "Parry"),
            (
                k(Action::FireVessel),
                "Fire vessel",
                k(Action::ArcSnare),
                "Arc snare",
            ),
            (k(Action::Interact), "Interact", k(Action::Heal), "Heal"),
            (k(Action::Down), "Aerial slam", "F9".into(), "Lighting"),
            (
                format!("{}+{}", k(Action::Down), k(Action::Jump)),
                "Drop through",
                p.menu(Menu::Atlas).into(),
                "Atlas",
            ),
        ];
        for (i, (left, a, right, b)) in rows.iter().enumerate() {
            let y = 240. + i as f32 * 31.;
            self.key(left, 311., y);
            self.text(a, 398., y + 18., 19., c(PALE));
            self.key(right, 664., y);
            self.text(b, 751., y + 18., 19., c(PALE));
        }
        // The run's place and build, which play shows only in pieces.
        let player = &g.player;
        let mut status = format!(
            "{}   /   STAGE {} OF 3   /   {} TIER {}",
            g.level.biome.name(),
            g.stage + 1,
            player.weapon.name(),
            player.tier
        );
        if let Some(mutation) = player.mutation_name() {
            status += &format!("   /   {}", mutation.to_uppercase());
        }
        if g.settings.speed < 10 {
            status += &format!("   /   GAME SPEED {}%", g.settings.speed as u32 * 10);
        }
        // A confirmation's warning takes the status line, above the links
        // that a second press or click confirms.
        let warning = if g.quit_armed {
            Some(format!(
                "Press {} again to quit. {}",
                p.menu(Menu::QuitPaused),
                if g.run_is_saved() {
                    "Continuing later restarts this biome; carried embers are lost."
                } else {
                    "Runs are saved from the second biome on, so this one ends."
                }
            ))
        } else if g.abandon_armed {
            Some(format!(
                "Press {} again to abandon this run. Carried embers and equipment will be lost.",
                p.menu(Menu::Abandon)
            ))
        } else if g.pad_lost {
            Some("Reconnect it to carry on, or resume with the keyboard.".into())
        } else {
            None
        };
        match warning {
            Some(warning) => self.center(&warning, 478., 15., c(WARN)),
            None => self.center(&status, 478., 13., c(TEAL)),
        }
        self.button(
            &format!("{}   Resume the descent", p.menu(Menu::Pause)),
            PAUSE_BUTTON,
        );
        self.links(g, 15.);
    }
    fn controls(&self, g: &Game) {
        self.modal(Rect::new(255., 114., 770., 493.), "Controls", 221.);
        let keys = &g.settings.keys;
        for (i, action) in Action::ALL.iter().enumerate() {
            let row = controls_row(i);
            let (x, y) = (row.x + 6., row.y + 4.);
            let selected = i == g.controls_row;
            if selected {
                draw_rectangle(row.x, row.y, row.w, row.h, c(TEAL).with_alpha(0.12));
            }
            self.text(
                action.label(),
                x + 10.,
                y + 18.,
                17.,
                c(if selected { GOLD } else { PALE }),
            );
            let label = if selected && g.rebinding {
                "PRESS A KEY".to_string()
            } else {
                keys.label(*action)
            };
            self.text(&label, x + 170., y + 18., 17., c(TEAL));
            // The controller layout is fixed; it's shown for reference.
            self.text(crate::pad::label(*action), x + 282., y + 18., 14., c(GOLD));
        }
        let reset = g.controls_row == Action::ALL.len();
        if reset {
            let r = CONTROLS_RESET;
            draw_rectangle(r.x, r.y, r.w, r.h, c(TEAL).with_alpha(0.12));
        }
        self.center(
            "Restore default keys",
            466.,
            17.,
            c(if reset { GOLD } else { PALE }),
        );
        let p = g.prompts();
        let note = if g.rebinding {
            if p.pad() {
                "Press the new key on the keyboard now. ESC or B cancels."
            } else {
                "Press the new key now. ESC or a click cancels."
            }
        } else {
            g.controls_note.as_deref().unwrap_or(
                "Arrows and mouse buttons work too; menus keep ESC, ENTER, TAB, M. Controller buttons (gold) are fixed.",
            )
        };
        self.center(note, 500., 15., c(MUTED));
        self.button(&format!("{}   Back", p.menu(Menu::Back)), CONTROLS_BACK);
        self.center(&controls_footer(g), 584., 14., c(MUTED));
    }
    fn options(&self, g: &Game) {
        let p = g.prompts();
        self.modal(Rect::new(255., 114., 770., 493.), "Options", 221.);
        for row in 0..Settings::ROWS {
            let y = 228. + row as f32 * 26.;
            let selected = row == g.options_row;
            let (label, value, _) = g.settings.row(row);
            if selected {
                let r = options_row(row);
                draw_rectangle(r.x, r.y, r.w, r.h, c(TEAL).with_alpha(0.12));
                self.text(">", 312., y + 18., 19., c(TEAL));
            }
            self.text(
                label,
                336.,
                y + 18.,
                19.,
                c(if selected { GOLD } else { PALE }),
            );
            match value {
                RowValue::Level(level) => {
                    for i in 0..10 {
                        let rect = Rect::new(640. + i as f32 * 24., y + 7., 19., 12.);
                        let color = if i < level {
                            c(TEAL)
                        } else {
                            c(MUTED).with_alpha(0.22)
                        };
                        draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
                    }
                    if selected {
                        // Arrows for stepping with the mouse, down to 0%.
                        for (up, glyph) in [(false, "<"), (true, ">")] {
                            let r = options_arrow(row, up);
                            self.centered_at(glyph, r.x + r.w / 2., y + 18., 19., c(GOLD));
                        }
                    }
                    self.text(
                        &format!("{}%", level as u32 * 10),
                        914.,
                        y + 18.,
                        17.,
                        c(PALE),
                    );
                }
                RowValue::Page => {
                    self.text(
                        &format!(
                            "{}   Rebind keys",
                            if mouse_last(g) {
                                "CLICK"
                            } else {
                                p.menu(Menu::Confirm)
                            }
                        ),
                        640.,
                        y + 18.,
                        17.,
                        c(TEAL),
                    );
                }
                RowValue::Switch(on) => {
                    self.text(
                        if on { "ON" } else { "OFF" },
                        640.,
                        y + 18.,
                        19.,
                        c(if on { TEAL } else { MUTED }),
                    );
                }
            }
        }
        let (_, _, help) = g.settings.row(g.options_row);
        self.center(help, 503., 15., c(MUTED));
        self.button(&format!("{}   Back", p.menu(Menu::Back)), OPTIONS_BACK);
        self.center(&options_footer(g), 584., 14., c(MUTED));
    }
    fn reliquary(&self, g: &Game) {
        let Some(found) = g.offer else { return };
        let p = &g.player;
        self.skin.crest(Rect::new(593., 111., 94., 71.));
        self.heading("A reliquary opens", 640., 227., 43.);
        self.center(
            &format!(
                "Either weapon is tempered to tier {}. Which will you carry?",
                p.tier
            ),
            262.,
            18.,
            c(PALE),
        );
        for (i, (weapon, verb)) in [(found, "TAKE"), (p.weapon, "KEEP")]
            .into_iter()
            .enumerate()
        {
            let x = reliquary_card(i).x;
            let icon = match weapon {
                Weapon::Sabre => 0,
                Weapon::Glaive => 1,
                Weapon::Hammer => 2,
            };
            self.skin.panel(reliquary_card(i));
            self.skin
                .icon(icon, Rect::new(x + 128., 320., 74., 74.), 1.);
            self.centered_at(weapon.name(), x + 165., 424., 21., c(GOLD));
            self.centered_at(weapon.summary(), x + 165., 450., 16., c(MUTED));
            self.centered_at(
                &format!(
                    "{:.0} damage   {:.0} reach   {:.2}s swing",
                    p.damage_with(weapon),
                    weapon.reach(),
                    weapon.delay()
                ),
                x + 165.,
                482.,
                16.,
                c(PALE),
            );
            self.skin.plaque(Rect::new(x + 90., 515., 150., 40.));
            self.centered_at(
                &format!("{}  {verb}", g.prompts().menu(Menu::Choice(i))),
                x + 165.,
                541.,
                17.,
                c(TEAL),
            );
        }
    }
    fn disciplines(&self, g: &Game) {
        self.skin.crest(Rect::new(593., 111., 94., 71.));
        self.heading("A memory, made yours", 640., 227., 43.);
        self.center(
            "Choose a discipline. Every memory also restores vitality.",
            262.,
            18.,
            c(PALE),
        );
        for (i, (name, line1, line2, color)) in [
            (
                "Ferocity",
                "Melee damage +17%",
                "Maximum vitality +10",
                0xef9c81,
            ),
            (
                "Ingenuity",
                "Stronger bolts and fire",
                "Maximum vitality +10",
                0x9ed7eb,
            ),
            (
                "Resolve",
                "Maximum vitality +24",
                "Endure the next descent",
                0xb4d89a,
            ),
        ]
        .iter()
        .enumerate()
        {
            let x = memory_card(i).x;
            self.skin.panel(memory_card(i));
            self.skin
                .icon(10 + i, Rect::new(x + 125., 323., 74., 74.), 1.);
            self.heading(name, x + 162., 435., 35.);
            self.centered_at(line1, x + 162., 470., 17., c(PALE));
            self.centered_at(line2, x + 162., 495., 16., c(MUTED));
            self.skin.plaque(Rect::new(x + 95., 512., 134., 38.));
            self.centered_at(
                &format!("{}  CHOOSE", g.prompts().menu(Menu::Choice(i))),
                x + 162.,
                537.,
                16.,
                c(*color),
            );
        }
    }
    fn camp(&self, g: &Game) {
        self.modal(Rect::new(180., 57., 920., 580.), "The Keeper's rest", 168.);
        self.center(
            &format!(
                "{} BANKED EMBERS  /  Heart & flask are permanent; mutations last this run",
                g.save.embers
            ),
            201.,
            16.,
            c(TEAL),
        );
        let (mutation_title, mutation_detail) = match g.player.mutation {
            1 => (
                "Switch to swift skills",
                "Mending active / next: skills recharge 35% faster",
            ),
            2 => (
                "Switch to mending",
                "Swift skills active / next: heal 3 vitality per kill",
            ),
            _ => (
                "Bind mending",
                "No mutation active / heal 3 vitality per kill",
            ),
        };
        for (i, (icon, title, detail, cost)) in [
            (
                12,
                "Reinforce your heart",
                "+15 maximum vitality",
                10 + g.save.vitality * 8,
            ),
            (
                6,
                if g.save.flask >= 3 {
                    "Flask fully reinforced"
                } else {
                    "Expand the flask"
                },
                if g.save.flask >= 3 {
                    "All permanent flask upgrades acquired"
                } else {
                    "+1 healing charge"
                },
                20 + g.save.flask * 20,
            ),
            (8, mutation_title, mutation_detail, 15),
        ]
        .iter()
        .enumerate()
        {
            let y = keeper_row(i).y;
            self.skin.panel(keeper_row(i));
            self.key(g.prompts().menu(Menu::Choice(i)), 239., y + 18.);
            self.skin.icon(*icon, Rect::new(282., y + 9., 38., 40.), 1.);
            self.text(title, 338., y + 25., 20., c(PALE));
            self.text(detail, 338., y + 45., 14., c(MUTED));
            if i == 1 && g.save.flask >= 3 {
                self.text("MAXED", 917., y + 36., 18., c(MUTED));
                continue;
            }
            self.skin.icon(8, Rect::new(886., y + 19., 17., 24.), 1.);
            self.text(
                &format!("{cost} embers"),
                917.,
                y + 36.,
                18.,
                c(if g.save.embers >= *cost {
                    TEAL
                } else {
                    0xe09483
                }),
            );
        }
        self.center(
            &if g.stage == 0 {
                route_hint(g)
            } else {
                "The Regent awaits above the clouds.".into()
            },
            452.,
            17.,
            c(MUTED),
        );
        self.skin.icon(14, Rect::new(376., 469., 36., 29.), 1.);
        self.heading(
            if g.stage == 0 {
                if g.route == 0 {
                    "Glassroot Conservatory"
                } else {
                    "The Ember Foundry"
                }
            } else {
                "Crown of the Machine"
            },
            661.,
            495.,
            31.,
        );
        self.button(
            &format!("{}   Continue the descent", g.prompts().menu(Menu::Confirm)),
            CAMP_BUTTON,
        );
        if g.notice_time > 0. {
            self.center(&g.notice, 520., 15., c(TEAL));
        }
    }
    fn result(&self, g: &Game) {
        let win = g.screen == Screen::Victory;
        let p = &g.player;
        self.skin.panel(Rect::new(207., 52., 866., 616.));
        self.skin
            .icon(if win { 13 } else { 8 }, Rect::new(604., 72., 72., 70.), 1.);
        self.heading(
            if win {
                "The sun remembers"
            } else {
                "Ashes, again"
            },
            640.,
            196.,
            48.,
        );
        let recap = g.recap;
        let stage = recap.map_or(g.stage, |r| r.stage);
        let biome = recap.map_or(g.level.biome, |r| r.biome);
        self.center(
            &if win {
                "The Brass Regent is silent. The Crown Rune is yours.".to_string()
            } else {
                let cause = recap
                    .and_then(|r| r.cause)
                    .map_or("Your vessel is gone", Cause::text);
                format!("{cause} in {}.", biome.place())
            },
            236.,
            20.,
            c(PALE),
        );

        // This run, on the left; the records it is measured against, on the right.
        let left = Rect::new(247., 262., 386., 242.);
        let right = Rect::new(647., 262., 386., 242.);
        self.skin.panel(left);
        self.skin.panel(right);
        self.centered_at(
            "THIS DESCENT",
            left.x + left.w / 2.,
            left.y + 30.,
            13.,
            c(MUTED),
        );
        self.centered_at(
            "RECORDS",
            right.x + right.w / 2.,
            right.y + 30.,
            13.,
            c(MUTED),
        );
        let embers = recap.map_or(0, |r| r.embers);
        let rows: [(&str, String); 6] = [
            ("Reached", format!("Stage {} of 3", stage + 1)),
            (
                "Weapon",
                format!("{} / tier {}", title_case(p.weapon.name()), p.tier),
            ),
            ("Memories", String::new()),
            ("Mutation", p.mutation_name().unwrap_or("None").to_string()),
            (
                "Felled",
                format!("{} guardians in {}", p.kills, clock(g.run_time)),
            ),
            (
                if win { "Banked" } else { "Lost" },
                format!(
                    "{embers} carried {}",
                    if embers == 1 { "ember" } else { "embers" }
                ),
            ),
        ];
        for (i, (label, value)) in rows.iter().enumerate() {
            let y = left.y + 62. + i as f32 * 29.;
            self.text(label, left.x + 26., y, 15., c(MUTED));
            self.text(value, left.x + 122., y, 17., c(PALE));
        }
        // Ferocity, Ingenuity, and Resolve, as the HUD shows them.
        for (i, color) in [0xef9c81, 0x9ed7eb, 0xb4d89a].into_iter().enumerate() {
            let x = left.x + 122. + i as f32 * 52.;
            self.skin
                .icon(10 + i, Rect::new(x, left.y + 106., 19., 20.), 1.);
            self.text(
                &p.power[i].to_string(),
                x + 24.,
                left.y + 120.,
                17.,
                c(color),
            );
        }
        if g.settings.speed < 10 {
            self.text(
                &format!("Game speed {}%", g.settings.speed as u32 * 10),
                left.x + 26.,
                left.y + 226.,
                13.,
                c(MUTED),
            );
        }
        let s = &g.save;
        let fresh = |new: bool| new.then_some("NEW");
        let records: [(&str, String, Option<&str>); 5] = [
            (
                "Most felled",
                format!("{} guardians", s.best_kills),
                fresh(recap.is_some_and(|r| r.new_kills)),
            ),
            (
                "Deepest",
                s.best_stage.map_or("Stage 1 of 3".into(), |best| {
                    format!("Stage {} of 3", best + 1)
                }),
                fresh(recap.is_some_and(|r| r.new_stage)),
            ),
            (
                "Fastest win",
                s.best_time.map_or("Not yet".into(), clock),
                fresh(recap.is_some_and(|r| r.new_time)),
            ),
            ("Victories", s.wins.to_string(), None),
            ("Embers kept", s.embers.to_string(), None),
        ];
        for (i, (label, value, new)) in records.iter().enumerate() {
            let y = right.y + 62. + i as f32 * 29.;
            self.text(label, right.x + 26., y, 15., c(MUTED));
            self.text(value, right.x + 136., y, 17., c(PALE));
            if let Some(new) = new {
                self.text(new, right.x + 326., y, 14., c(GOLD));
            }
        }
        if g.practice {
            self.centered_at(
                "Practice runs don't change records.",
                right.x + right.w / 2.,
                right.y + 226.,
                13.,
                c(MUTED),
            );
        }
        self.center(
            if win {
                "The next descent awakens stronger guardians."
            } else {
                "Banked embers, upgrades, and the rune remain. Unbanked embers were lost."
            },
            534.,
            16.,
            c(MUTED),
        );
        // The button appears once confirming is accepted.
        if g.result_ready() {
            self.button(
                &format!("{}   Rise again", g.prompts().menu(Menu::Confirm)),
                RESULT_BUTTON,
            );
        }
    }
    fn map_point(g: &Game, rect: Rect, world: Vec2) -> Vec2 {
        let depth = (g.level.max_y - g.level.min_y).max(1.);
        vec2(
            rect.x + 4. + (world.x / g.level.width).clamp(0., 1.) * (rect.w - 8.),
            rect.y + 4. + ((world.y - g.level.min_y) / depth).clamp(0., 1.) * (rect.h - 8.),
        )
    }
    fn minimap(&self, g: &Game, rect: Rect) {
        let Rect { x, y, w, h } = rect;
        let expanded = h > 120.;
        // Practice and staged captures keep the complete survey.
        let fog = !g.practice;
        let seen = |pos: Vec2| !fog || g.survey.seen(pos);
        draw_rectangle(x, y, w, h, c(0x091923));
        for (top, bottom, color) in [
            (g.level.min_y, 121., 0x142c37),
            (121., 451., 0x172f32),
            (451., g.level.max_y, 0x10232c),
        ] {
            let a = Self::map_point(g, rect, vec2(0., top));
            let b = Self::map_point(g, rect, vec2(g.level.width, bottom));
            draw_rectangle(a.x, a.y, b.x - a.x, b.y - a.y, c(color));
        }
        if fog {
            for cell in g.survey.unseen() {
                let a = Self::map_point(g, rect, vec2(cell.x, cell.y));
                let b = Self::map_point(g, rect, vec2(cell.right(), cell.bottom()));
                draw_rectangle(a.x, a.y, b.x - a.x, b.y - a.y, c(0x03090d).with_alpha(0.78));
            }
        }
        for tier_y in [Level::UPPER, FLOOR, Level::LOWER] {
            let a = Self::map_point(g, rect, vec2(0., tier_y));
            let b = Self::map_point(g, rect, vec2(g.level.width, tier_y));
            draw_line(a.x, a.y, b.x, b.y, 1., c(0x456465).with_alpha(0.45));
        }
        for p in &g.level.platforms {
            let spans = if fog {
                g.survey.seen_spans(p.x, p.right(), p.y)
            } else {
                vec![(p.x, p.right())]
            };
            for (left, right) in spans {
                let a = Self::map_point(g, rect, vec2(left, p.y));
                let b = Self::map_point(g, rect, vec2(right, p.bottom()));
                draw_rectangle(
                    a.x,
                    a.y,
                    (b.x - a.x).max(1.),
                    (b.y - a.y).max(if expanded { 2. } else { 1. }),
                    c(0x779b91),
                );
            }
        }
        if expanded {
            for hazard in g.level.hazards.iter().filter(|h| seen(h.center())) {
                let a = Self::map_point(g, rect, vec2(hazard.x, hazard.y));
                let b = Self::map_point(g, rect, vec2(hazard.right(), hazard.bottom()));
                draw_rectangle(a.x, a.y, (b.x - a.x).max(2.), 3., c(0xe09078));
            }
        }
        // The window is the actual two-dimensional camera footprint, including
        // above- and below-ground views.
        let view_a = Self::map_point(g, rect, vec2(g.camera, g.camera_y));
        let view_b = Self::map_point(g, rect, vec2(g.camera + 640., g.camera_y + 360.));
        draw_rectangle(
            view_a.x,
            view_a.y,
            view_b.x - view_a.x,
            view_b.y - view_a.y,
            c(TEAL).with_alpha(0.06),
        );
        draw_rectangle_lines(
            view_a.x,
            view_a.y,
            view_b.x - view_a.x,
            view_b.y - view_a.y,
            1.,
            c(TEAL).with_alpha(0.48),
        );
        if expanded {
            for enemy in g
                .level
                .enemies
                .iter()
                .filter(|enemy| enemy.hp > 0. && seen(enemy.pos))
            {
                let pos = Self::map_point(g, rect, enemy.pos - vec2(0., 14.));
                draw_circle(pos.x, pos.y, 2.4, c(0xdb927c));
            }
        }
        // The bellgate is always marked so the destination is never lost.
        for o in g
            .level
            .objects
            .iter()
            .filter(|o| !o.used && (o.kind == ObjectKind::Exit || seen(o.pos)))
        {
            let pos = Self::map_point(g, rect, o.pos - vec2(0., 8.));
            let radius = if expanded { 3. } else { 1.6 };
            match o.kind {
                ObjectKind::Exit => {
                    let radius = radius * 1.7;
                    draw_rectangle(
                        pos.x - radius,
                        pos.y - radius,
                        radius * 2.,
                        radius * 2.,
                        INK,
                    );
                    draw_poly(pos.x, pos.y, 4, radius, 0., c(GOLD));
                    if expanded {
                        self.text("GATE", pos.x - 18., pos.y - 10., 12., c(GOLD));
                    }
                }
                ObjectKind::Fountain => {
                    draw_circle(pos.x, pos.y, radius, c(0x8bc9ef));
                }
                _ => {
                    draw_rectangle(
                        pos.x - radius,
                        pos.y - radius,
                        radius * 2.,
                        radius * 2.,
                        c(PALE),
                    );
                }
            }
        }
        let player = Self::map_point(g, rect, g.player.pos - vec2(0., 12.));
        let radius = if expanded { 5. } else { 2.8 };
        draw_circle(player.x, player.y, radius + 1.8, INK);
        draw_circle(player.x, player.y, radius, c(TEAL));
        if expanded {
            draw_circle_lines(player.x, player.y, radius + 4., 1., c(TEAL).with_alpha(0.6));
        }
        draw_rectangle_lines(x, y, w, h, 1., c(0x436263));
    }
}
/// Minutes and seconds, as the HUD shows them.
fn clock(seconds: f32) -> String {
    let s = seconds.max(0.) as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}
/// "FURNACE MAUL" as "Furnace Maul".
fn title_case(name: &str) -> String {
    name.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or(String::new(), |first| {
                first.to_string() + &chars.as_str().to_lowercase()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
