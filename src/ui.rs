//! Live interface composed over generated frames and item art.
use crate::{
    game::*,
    render::{c, INK},
    ui_skin::Skin,
    world::*,
};
use macroquad::prelude::*;

const GOLD: u32 = 0xf1d29c;
const PALE: u32 = 0xe1e7de;
const MUTED: u32 = 0x98b4b4;
const TEAL: u32 = 0x85dfcc;

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
        self.button(
            "ENTER   Begin the descent",
            Rect::new(145., 483., 418., 64.),
        );
        self.centered_at(
            &format!(
                "{} descents   /   {} embers kept",
                g.save.runs, g.save.embers
            ),
            354.,
            579.,
            16.,
            c(TEAL),
        );
        self.centered_at(
            "A / D  Move   SPACE  Jump   J  Strike",
            354.,
            624.,
            15.,
            c(MUTED),
        );
    }
    fn hud(&self, g: &Game) {
        let p = &g.player;
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
        self.centered_at("TAB  VERTICAL ATLAS", 1150., 83., 11., c(MUTED));

        // Artwork identifies equipment at a glance; bindings and cooldowns stay live.
        let weapon_icon = match p.weapon {
            Weapon::Sabre => 0,
            Weapon::Glaive => 1,
            Weapon::Hammer => 2,
        };
        let slots = [
            (
                "J",
                p.weapon.name(),
                weapon_icon,
                p.attack_cd,
                p.weapon.delay(),
            ),
            ("K", "GLASSBOLT", 3, p.bow_cd, 0.32),
            ("Q", "FIRE VESSEL", 4, p.grenade_cd, 5.),
            ("R", "ARC SNARE", 5, p.trap_cd, 8.),
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
        self.skin.icon(
            6,
            Rect::new(838., 649., 40., 44.),
            if p.flasks == 0 { 0.4 } else { 1. },
        );
        self.key("F", 844., 683.);
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
        self.text("L  PARRY", 1077., 669., 15., c(PALE));
        self.skin.icon(15, Rect::new(1148., 650., 27., 28.), 1.);
        self.text("SHIFT", 1184., 668., 14., c(PALE));
        self.text("DODGE", 1184., 684., 11., c(MUTED));
        self.text("ESC  PAUSE", 1077., 695., 13., c(MUTED));
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
    pub fn draw(&self, g: &Game) {
        if g.screen == Screen::Title {
            self.title_screen(g);
            return;
        }
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
        if g.screen == Screen::Playing && !g.map {
            if let Some(i) = g.nearby() {
                let text = match g.level.objects[i].kind {
                    ObjectKind::Exit => "E   RING THE BELLGATE",
                    ObjectKind::Scroll => "E   CLAIM A MEMORY",
                    ObjectKind::Chest => "E   OPEN RELIQUARY",
                    ObjectKind::Fountain => "E   DRINK FROM THE WELL",
                    ObjectKind::Forge => "E   TEMPER WEAPON / 60 COPPER",
                    ObjectKind::Lore => "E   READ THE INSCRIPTION",
                    ObjectKind::Secret => "E   BREAK THE SEAL",
                };
                self.prompt(text, 587.);
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
                    "{}  /  {}  /  SEED {}",
                    crate::environment::zone_name(g.level.biome, g.camera, g.level.width),
                    Level::tier_name(g.player.pos.y),
                    g.level.seed
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
            self.center(
                "SPACE to climb stairways; press again to double jump  /  S + SPACE to drop  /  S in the air to slam",
                563.,
                16.,
                c(PALE),
            );
            self.center(
                "The outlined window tracks your view. All routes are shown; gaps connect the city's tiers.",
                586.,
                14.,
                c(MUTED),
            );
            self.center("TAB   Return to the descent", 612., 17., c(TEAL));
        }
        if matches!(
            g.screen,
            Screen::Paused | Screen::Scroll | Screen::Camp | Screen::Dead | Screen::Victory
        ) {
            draw_rectangle(0., 0., 1280., 720., INK.with_alpha(0.73));
            match g.screen {
                Screen::Paused => self.paused(g),
                Screen::Scroll => self.disciplines(),
                Screen::Camp => self.camp(g),
                Screen::Dead | Screen::Victory => self.result(g),
                _ => {}
            }
        }
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
        self.modal(Rect::new(255., 114., 770., 493.), "The city can wait", 221.);
        let rows = [
            ("A / D", "Move", "SPACE", "Double jump"),
            ("J", "Strike", "K", "Glassbolt"),
            ("SHIFT", "Dodge", "L", "Parry"),
            ("Q", "Fire vessel", "R", "Arc snare"),
            ("E", "Interact", "F", "Heal"),
            ("S", "Aerial slam", "F9", "Lighting"),
            ("S+SPACE", "Drop through", "TAB", "Atlas"),
        ];
        for (i, (left, a, right, b)) in rows.iter().enumerate() {
            let y = 246. + i as f32 * 34.;
            self.key(left, 311., y);
            self.text(a, 398., y + 18., 19., c(PALE));
            self.key(right, 664., y);
            self.text(b, 751., y + 18., 19., c(PALE));
        }
        self.button("ESC   Resume the descent", Rect::new(423., 493., 434., 57.));
        self.center(
            if g.muted {
                "M  Sound is muted"
            } else {
                "M  Sound is on"
            },
            577.,
            15.,
            c(MUTED),
        );
    }
    fn disciplines(&self) {
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
            let x = 128. + i as f32 * 350.;
            self.skin.panel(Rect::new(x, 300., 324., 267.));
            self.skin
                .icon(10 + i, Rect::new(x + 125., 323., 74., 74.), 1.);
            self.heading(name, x + 162., 435., 35.);
            self.centered_at(line1, x + 162., 470., 17., c(PALE));
            self.centered_at(line2, x + 162., 495., 16., c(MUTED));
            self.skin.plaque(Rect::new(x + 95., 512., 134., 38.));
            self.centered_at(
                &format!("{}  CHOOSE", i + 1),
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
            let y = 224. + i as f32 * 65.;
            self.skin.panel(Rect::new(222., y, 836., 59.));
            self.key(&(i + 1).to_string(), 239., y + 18.);
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
            if g.stage == 0 {
                "A / D   Choose your next destination"
            } else {
                "The Regent awaits above the clouds."
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
            "ENTER   Continue the descent",
            Rect::new(397., 531., 486., 63.),
        );
        if g.notice_time > 0. {
            self.center(&g.notice, 520., 15., c(TEAL));
        }
    }
    fn result(&self, g: &Game) {
        let win = g.screen == Screen::Victory;
        self.skin.panel(Rect::new(227., 118., 826., 464.));
        self.skin.icon(
            if win { 13 } else { 8 },
            Rect::new(599., 143., 82., 80.),
            1.,
        );
        self.heading(
            if win {
                "The sun remembers"
            } else {
                "Ashes, again"
            },
            640.,
            282.,
            48.,
        );
        self.center(
            if win {
                "The Brass Regent is silent. The Crown Rune is yours."
            } else {
                "Your vessel is gone. Your banked memories remain."
            },
            328.,
            20.,
            c(PALE),
        );
        self.center(
            &format!(
                "{} guardians   /   {:02}:{:02} elapsed   /   {} banked embers",
                g.player.kills,
                g.run_time as u32 / 60,
                g.run_time as u32 % 60,
                g.save.embers
            ),
            378.,
            19.,
            c(TEAL),
        );
        self.center(
            if win {
                "The next descent awakens stronger guardians."
            } else {
                "Unbanked embers were lost in the fire."
            },
            420.,
            18.,
            c(MUTED),
        );
        self.button("ENTER   Rise again", Rect::new(443., 472., 394., 63.));
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
        for tier_y in [Level::UPPER, FLOOR, Level::LOWER] {
            let a = Self::map_point(g, rect, vec2(0., tier_y));
            let b = Self::map_point(g, rect, vec2(g.level.width, tier_y));
            draw_line(a.x, a.y, b.x, b.y, 1., c(0x456465).with_alpha(0.45));
        }
        for p in &g.level.platforms {
            let a = Self::map_point(g, rect, vec2(p.x, p.y));
            let b = Self::map_point(g, rect, vec2(p.right(), p.bottom()));
            draw_rectangle(
                a.x,
                a.y,
                (b.x - a.x).max(1.),
                (b.y - a.y).max(if expanded { 2. } else { 1. }),
                c(0x779b91),
            );
        }
        if expanded {
            for hazard in &g.level.hazards {
                let a = Self::map_point(g, rect, vec2(hazard.x, hazard.y));
                let b = Self::map_point(g, rect, vec2(hazard.right(), hazard.bottom()));
                draw_rectangle(a.x, a.y, (b.x - a.x).max(2.), 3., c(0xe09078));
            }
        }
        // This is a complete atlas, not a fog-of-war map. The window is the actual
        // two-dimensional camera footprint, including above- and below-ground views.
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
            for enemy in g.level.enemies.iter().filter(|enemy| enemy.hp > 0.) {
                let pos = Self::map_point(g, rect, enemy.pos - vec2(0., 14.));
                draw_circle(pos.x, pos.y, 2.4, c(0xdb927c));
            }
        }
        for o in g.level.objects.iter().filter(|object| !object.used) {
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
