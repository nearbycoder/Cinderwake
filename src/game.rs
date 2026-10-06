pub use crate::particles::Particle;
use crate::{
    particles::{self, Effect, VisualRng},
    save::Save,
    settings::Settings,
    world::*,
};
use macroquad::prelude::*;
#[derive(Default, Clone, Copy)]
pub struct Input {
    pub axis: f32,
    pub jump: bool,
    pub jump_held: bool,
    pub dodge: bool,
    pub attack: bool,
    pub bow: bool,
    pub parry: bool,
    pub grenade: bool,
    pub trap: bool,
    pub heal: bool,
    pub interact: bool,
    pub down: bool,
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Screen {
    Title,
    Playing,
    Paused,
    Scroll,
    Camp,
    Options,
    Dead,
    Victory,
}
/// One-time tips, each shown the first time its mechanic becomes relevant.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Hint {
    Climb,
    Strike,
    Parry,
    Drop,
    Heal,
    Tools,
}
impl Hint {
    /// Display priority: tips about immediate danger come first.
    pub const ALL: [Hint; 6] = [
        Self::Strike,
        Self::Parry,
        Self::Heal,
        Self::Climb,
        Self::Drop,
        Self::Tools,
    ];
    pub fn bit(self) -> u32 {
        1 << self as u32
    }
    pub fn text(self) -> &'static str {
        match self {
            Self::Climb => "SPACE jumps. Press it again in the air to double jump onto higher ledges.",
            Self::Strike => "J or left click strikes; hold to chain a combo. SHIFT dodges through attacks.",
            Self::Parry => "Face a bolt and press L or right click to parry it back at the shooter.",
            Self::Drop => "S + SPACE drops through a ledge. Press S in mid-air to slam down.",
            Self::Heal => "F drinks a healing flask. Taking damage interrupts the drink.",
            Self::Tools => "K glassbolt, Q fire vessel, R arc snare. Embers are lost on death until banked at a bellgate.",
        }
    }
}
pub const HINT_SECONDS: f32 = 6.;
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Weapon {
    Sabre,
    Glaive,
    Hammer,
}
impl Weapon {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sabre => "CINDER SABRE",
            Self::Glaive => "GLASS GLAIVE",
            Self::Hammer => "FURNACE MAUL",
        }
    }
    pub fn damage(self) -> f32 {
        match self {
            Self::Sabre => 30.,
            Self::Glaive => 39.,
            Self::Hammer => 66.,
        }
    }
    pub fn reach(self) -> f32 {
        match self {
            Self::Sabre => 48.,
            Self::Glaive => 72.,
            Self::Hammer => 49.,
        }
    }
    pub fn delay(self) -> f32 {
        match self {
            Self::Sabre => 0.27,
            Self::Glaive => 0.38,
            Self::Hammer => 0.58,
        }
    }
}
#[derive(Clone)]
pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub face: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub ground: bool,
    pub jumps: u32,
    pub coyote: f32,
    pub buffer: f32,
    pub dodge: f32,
    pub dodge_cd: f32,
    pub invuln: f32,
    pub attack: f32,
    pub attack_cd: f32,
    pub combo: u32,
    pub parry: f32,
    pub parry_cd: f32,
    pub bow_cd: f32,
    pub grenade_cd: f32,
    pub trap_cd: f32,
    pub heal_time: f32,
    pub flasks: u32,
    pub slam: bool,
    /// Height of the one-way ledge currently being dropped through.
    pub drop_through: Option<f32>,
    pub weapon: Weapon,
    pub tier: u32,
    pub power: [u32; 3],
    pub gold: u32,
    pub embers: u32,
    pub kills: u32,
    pub mutation: u32,
}
impl Player {
    pub fn new(save: &Save) -> Self {
        let hp = 100. + save.vitality as f32 * 15.;
        Self {
            pos: vec2(110., FLOOR),
            vel: Vec2::ZERO,
            face: 1.,
            hp,
            max_hp: hp,
            ground: true,
            jumps: 0,
            coyote: 0.,
            buffer: 0.,
            dodge: 0.,
            dodge_cd: 0.,
            invuln: 0.,
            attack: 0.,
            attack_cd: 0.,
            combo: 0,
            parry: 0.,
            parry_cd: 0.,
            bow_cd: 0.,
            grenade_cd: 0.,
            trap_cd: 0.,
            heal_time: 0.,
            flasks: 2 + save.flask,
            slam: false,
            drop_through: None,
            weapon: Weapon::Sabre,
            tier: 1,
            power: [1, 1, 1],
            gold: 0,
            embers: 0,
            kills: 0,
            mutation: 0,
        }
    }
    pub fn rect(&self) -> Rect {
        Rect::new(self.pos.x - 7., self.pos.y - 28., 14., 28.)
    }
    pub fn damage(&self) -> f32 {
        self.weapon.damage() * (1. + self.power[0] as f32 * 0.17 + self.tier as f32 * 0.09)
    }
}
#[derive(Clone)]
pub struct FloatText {
    pub pos: Vec2,
    pub text: String,
    pub color: Color,
    pub life: f32,
}
#[derive(Clone)]
pub struct Shot {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub damage: f32,
    pub hostile: bool,
    pub kind: u8,
}
#[derive(Clone)]
pub struct Trap {
    pub pos: Vec2,
    pub life: f32,
    pub tick: f32,
}
#[derive(Clone, Copy)]
pub enum Sfx {
    Slash,
    Hit,
    Jump,
    Dodge,
    Parry,
    Loot,
    Hurt,
    Explosion,
    Heal,
}
pub struct Game {
    pub player: Player,
    pub level: Level,
    pub save: Save,
    pub screen: Screen,
    pub particles: Vec<Particle>,
    pub texts: Vec<FloatText>,
    pub shots: Vec<Shot>,
    pub traps: Vec<Trap>,
    pub camera: f32,
    pub camera_y: f32,
    pub shake: f32,
    pub hitstop: f32,
    pub time: f32,
    pub run_time: f32,
    pub stage: u32,
    pub seed: u64,
    pub notice: String,
    pub notice_time: f32,
    pub sounds: Vec<Sfx>,
    pub rng: Rng,
    visual_rng: VisualRng,
    effect_clock: f32,
    footstep_distance: f32,
    pub map: bool,
    pub settings: Settings,
    /// Screen to return to when the options page closes.
    pub options_from: Screen,
    pub options_row: usize,
    /// The first abandon request from the pause screen only asks for confirmation.
    pub abandon_armed: bool,
    /// Contextual tips run only in normal play, never in practice or captures.
    pub teach: bool,
    /// The tip on screen and its remaining seconds.
    pub hint: Option<(Hint, f32)>,
    pending_hints: u32,
    pub intro: f32,
    pub route: usize,
    pub save_error: Option<String>,
    pub practice: bool,
    last_safe_pos: Vec2,
}

/// One-way surfaces only catch downward crossings. Taking the highest crossed
/// top makes collision independent of authored platform order and fall speed.
fn crossed_surface(
    platforms: &[Rect],
    x: f32,
    half_width: f32,
    old_bottom: f32,
    new_bottom: f32,
    ignored_y: Option<f32>,
) -> Option<f32> {
    if new_bottom < old_bottom {
        return None;
    }
    platforms
        .iter()
        .filter(|r| {
            x + half_width > r.x
                && x - half_width < r.right()
                && old_bottom <= r.y + 0.5
                && new_bottom >= r.y
                && !ignored_y.is_some_and(|y| (r.y - y).abs() < 1.)
        })
        .map(|r| r.y)
        .min_by(f32::total_cmp)
}

fn surface_below(platforms: &[Rect], x: f32, y: f32) -> Option<f32> {
    platforms
        .iter()
        .filter(|r| x >= r.x && x <= r.right() && r.y >= y - 0.5)
        .map(|r| r.y)
        .min_by(f32::total_cmp)
}

/// Grounded enemies keep their original ledge, including joined authored pieces.
fn patrol_bounds(platforms: &[Rect], enemy: &Enemy) -> Option<(f32, f32)> {
    let platform = platforms.iter().find(|r| {
        (r.y - enemy.home_y).abs() < 3. && enemy.home >= r.x && enemy.home <= r.right()
    })?;
    let (mut left, mut right) = (platform.x, platform.right());
    loop {
        let before = (left, right);
        for r in platforms.iter().filter(|r| (r.y - enemy.home_y).abs() < 3.) {
            if r.x <= right + 1. && r.right() >= left - 1. {
                left = left.min(r.x);
                right = right.max(r.right());
            }
        }
        if before == (left, right) {
            break;
        }
    }
    let margin = (enemy.rect().w * 0.5 + 2.).min((right - left) * 0.5);
    Some((left + margin, right - margin))
}

fn keep_enemy_on_tier(platforms: &[Rect], enemy: &mut Enemy) {
    if enemy.kind == EnemyKind::Moth {
        enemy.pos.x = enemy.pos.x.clamp(enemy.home - 190., enemy.home + 190.);
        enemy.pos.y = enemy.pos.y.clamp(enemy.home_y - 95., enemy.home_y + 60.);
    } else if let Some((left, right)) = patrol_bounds(platforms, enemy) {
        enemy.pos.x = enemy.pos.x.clamp(left, right);
        enemy.pos.y = enemy.home_y;
    }
}

impl Game {
    pub fn new(seed: u64, save: Save) -> Self {
        let level = Level::generate(seed, Biome::Aqueduct, save.wins);
        let mut player = Player::new(&save);
        player.pos = level.spawn;
        let camera_y = (level.spawn.y - 248.).clamp(level.min_y, level.max_y - 360.);
        let last_safe_pos = level.spawn;
        Self {
            player,
            level,
            save,
            screen: Screen::Title,
            particles: vec![],
            texts: vec![],
            shots: vec![],
            traps: vec![],
            camera: 0.,
            camera_y,
            shake: 0.,
            hitstop: 0.,
            time: 0.,
            run_time: 0.,
            stage: 0,
            seed,
            notice: String::new(),
            notice_time: 0.,
            sounds: vec![],
            rng: Rng(seed.max(1)),
            visual_rng: VisualRng::new(seed),
            effect_clock: 0.,
            footstep_distance: 0.,
            map: false,
            settings: Settings::default(),
            options_from: Screen::Title,
            options_row: 0,
            abandon_armed: false,
            teach: false,
            hint: None,
            pending_hints: 0,
            intro: 4.,
            route: 0,
            save_error: None,
            practice: false,
            last_safe_pos,
        }
    }
    pub fn persist(&mut self) {
        if !self.practice {
            if let Err(e) = self.save.store() {
                self.save_error = Some(e.to_string());
            }
        }
    }
    pub fn start(&mut self) {
        let save = self.save.clone();
        let settings = self.settings.clone();
        let practice = self.practice;
        let teach = self.teach;
        *self = Self::new(self.seed.wrapping_add(173), save);
        self.settings = settings;
        self.teach = teach;
        self.practice = practice;
        self.screen = Screen::Playing;
        self.save.runs += 1;
        self.persist();
        self.notify("Find the bellgate. Bank your embers. Rise again.");
    }
    pub fn notify(&mut self, s: &str) {
        self.notice = s.into();
        self.notice_time = 4.;
    }
    fn effect(&mut self, effect: Effect, pos: Vec2, dir: f32) {
        let floor = self
            .level
            .support_at(pos.x, pos.y)
            .map(|platform| platform.y)
            .unwrap_or(self.level.max_y);
        particles::emit(
            &mut self.particles,
            &mut self.visual_rng,
            effect,
            pos,
            dir,
            floor,
        );
    }
    pub fn label(&mut self, pos: Vec2, text: String, color: Color) {
        self.texts.push(FloatText {
            pos,
            text,
            color,
            life: 1.,
        });
    }
    pub fn hurt(&mut self, damage: f32, dir: f32) {
        let p = &mut self.player;
        if p.invuln > 0. || p.dodge > 0. || p.hp <= 0. {
            return;
        }
        p.hp = (p.hp - damage).max(0.);
        p.invuln = 0.9;
        p.vel.x = dir * 190.;
        p.heal_time = 0.;
        self.shake = 7.;
        self.sounds.push(Sfx::Hurt);
        self.effect(Effect::Hurt, self.player.pos - vec2(0., 14.), dir);
        if self.player.hp <= 0. {
            self.die();
        }
    }
    fn die(&mut self) {
        self.screen = Screen::Dead;
        self.save.best_kills = self.save.best_kills.max(self.player.kills);
        self.player.embers = 0;
        self.persist();
    }
    /// Pause-screen abandon: the first request arms, the second ends the run
    /// with the same losses as a death.
    pub fn request_abandon(&mut self) {
        if self.screen != Screen::Paused {
            return;
        }
        if self.abandon_armed {
            self.abandon_armed = false;
            self.player.hp = 0.;
            self.die();
        } else {
            self.abandon_armed = true;
        }
    }
    pub fn open_options(&mut self) {
        if matches!(self.screen, Screen::Title | Screen::Paused) {
            self.options_from = self.screen;
            self.options_row = 0;
            self.abandon_armed = false;
            self.screen = Screen::Options;
        }
    }
    pub fn close_options(&mut self) {
        if self.screen == Screen::Options {
            self.screen = self.options_from;
            self.persist_settings();
        }
    }
    fn queue_hint(&mut self, hint: Hint) {
        if self.settings.hints_seen & hint.bit() == 0 {
            self.pending_hints |= hint.bit();
        }
    }
    /// Notices relevant mechanics, then shows one queued tip at a time once
    /// the biome title has faded. Tips never pause play.
    fn update_hints(&mut self, dt: f32) {
        if !self.teach || !self.settings.hints {
            self.hint = None;
            self.pending_hints = 0;
            return;
        }
        let p = &self.player;
        let overhead = |r: &Rect| {
            r.y < p.pos.y - 30.
                && r.y > p.pos.y - 100.
                && r.x < p.pos.x + 30.
                && r.x + r.w > p.pos.x - 30.
        };
        let mut seen = vec![];
        if p.ground && self.level.platforms.iter().any(overhead) {
            seen.push(Hint::Climb);
        }
        if self.level.enemies.iter().any(|e| {
            e.hp > 0. && (e.pos.x - p.pos.x).abs() < 170. && (e.pos.y - p.pos.y).abs() < 45.
        }) {
            seen.push(Hint::Strike);
        }
        if self.shots.iter().any(|s| {
            s.hostile && (s.pos - p.pos).length() < 220. && s.vel.x * (p.pos.x - s.pos.x) > 0.
        }) {
            seen.push(Hint::Parry);
        }
        if p.ground && p.pos.y < FLOOR - 20. {
            seen.push(Hint::Drop);
        }
        if p.hp < p.max_hp * 0.6 && p.flasks > 0 {
            seen.push(Hint::Heal);
        }
        for hint in seen {
            self.queue_hint(hint);
        }
        if let Some((_, remaining)) = &mut self.hint {
            *remaining -= dt;
            if *remaining <= 0. {
                self.hint = None;
            }
        }
        if self.hint.is_none() && self.intro <= 0. {
            if let Some(next) = Hint::ALL
                .into_iter()
                .find(|h| self.pending_hints & h.bit() != 0)
            {
                self.pending_hints &= !next.bit();
                self.settings.hints_seen |= next.bit();
                self.hint = Some((next, HINT_SECONDS));
                self.persist_settings();
            }
        }
    }
    pub fn persist_settings(&mut self) {
        if !self.practice {
            if let Err(e) = self.settings.store() {
                self.save_error = Some(e.to_string());
            }
        }
    }
    pub fn hit(&mut self, index: usize, damage: f32, dir: f32, burn: bool) {
        let e = &mut self.level.enemies[index];
        if e.hp <= 0. {
            return;
        }
        e.hp -= damage;
        e.flash = 0.12;
        e.stun = if e.kind == EnemyKind::Regent {
            0.06
        } else {
            0.22
        };
        if burn {
            e.burn = 2.;
        }
        e.pos.x = (e.pos.x + dir * 8.).clamp(15., self.level.width - 15.);
        keep_enemy_on_tier(&self.level.platforms, e);
        let pos = e.pos - vec2(0., 20.);
        let dead = e.hp <= 0.;
        let boss = e.kind == EnemyKind::Regent;
        self.label(pos, format!("{}", damage as u32), Color::from_hex(0xffdb9b));
        self.effect(if dead { Effect::Death } else { Effect::Hit }, pos, dir);
        self.sounds.push(Sfx::Hit);
        self.shake = 3.;
        if dead {
            self.queue_hint(Hint::Tools);
            self.player.kills += 1;
            self.player.gold += if boss { 300 } else { 18 };
            self.player.embers += if boss { 25 } else { 3 };
            if self.player.mutation == 1 {
                self.player.hp = (self.player.hp + 3.).min(self.player.max_hp);
            }
            if boss {
                self.save.rune = true;
                self.notify("THE REGENT FALLS. The bellgate is open.");
            }
            self.label(
                pos - vec2(0., 18.),
                "+ EMBERS".into(),
                Color::from_hex(0x7de8cf),
            );
        }
    }
    pub fn tick(&mut self, dt: f32, input: Input) {
        if matches!(
            self.screen,
            Screen::Paused | Screen::Scroll | Screen::Camp | Screen::Options
        ) {
            return;
        }
        self.time += dt;
        self.notice_time = (self.notice_time - dt).max(0.);
        self.shake = (self.shake - dt * 24.).max(0.);
        particles::update(&mut self.particles, dt);
        for t in &mut self.texts {
            t.life -= dt;
            t.pos.y -= 22. * dt;
        }
        self.texts.retain(|t| t.life > 0.);
        if self.screen != Screen::Playing {
            return;
        }
        self.run_time += dt;
        self.intro = (self.intro - dt).max(0.);
        self.update_hints(dt);
        if self.hitstop > 0. {
            self.hitstop -= dt;
            return;
        }
        let mut effects = Vec::with_capacity(8);
        let p = &mut self.player;
        let was_ground = p.ground;
        let previous_pos = p.pos;
        for timer in [
            &mut p.dodge,
            &mut p.dodge_cd,
            &mut p.invuln,
            &mut p.attack,
            &mut p.attack_cd,
            &mut p.parry,
            &mut p.parry_cd,
            &mut p.bow_cd,
            &mut p.grenade_cd,
            &mut p.trap_cd,
            &mut p.coyote,
            &mut p.buffer,
        ] {
            *timer = (*timer - dt).max(0.);
        }
        if p.mutation == 2 {
            p.grenade_cd = (p.grenade_cd - dt * 0.35).max(0.);
            p.trap_cd = (p.trap_cd - dt * 0.35).max(0.);
        }
        if p.ground {
            p.coyote = 0.09;
            p.jumps = 0;
        }
        let drop_requested = input.down && input.jump && p.ground;
        if drop_requested {
            // The lowest authored floor remains solid. All other ledges can be
            // left deliberately; input is consumed even when dropping is blocked.
            if self.level.platforms.iter().any(|r| r.y > p.pos.y + 1.) {
                p.drop_through = Some(p.pos.y);
                p.pos.y += 1.;
                p.vel.y = 65.;
                p.ground = false;
                p.coyote = 0.;
                p.slam = false;
            }
            p.buffer = 0.;
        } else if input.jump {
            p.buffer = 0.12;
        }
        if input.axis != 0. && p.dodge <= 0. {
            p.face = input.axis.signum();
        }
        if input.dodge && p.dodge_cd <= 0. && p.heal_time <= 0. {
            p.dodge = 0.23;
            p.dodge_cd = 0.65;
            p.invuln = 0.23;
            p.slam = false;
            self.sounds.push(Sfx::Dodge);
            effects.push((Effect::Dodge, p.pos, -p.face));
        }
        if input.parry && p.parry_cd <= 0. {
            p.parry = 0.20;
            p.parry_cd = 0.52;
            effects.push((Effect::ParryReady, p.pos + vec2(p.face * 13., -17.), p.face));
        }
        if p.buffer > 0. && (p.coyote > 0. || p.jumps < 2) && p.dodge <= 0. {
            p.vel.y = -300.;
            p.jumps += 1;
            p.ground = false;
            p.coyote = 0.;
            p.buffer = 0.;
            self.sounds.push(Sfx::Jump);
            effects.push((
                if p.jumps > 1 {
                    Effect::AirJump
                } else {
                    Effect::Jump
                },
                p.pos,
                -p.face,
            ));
        }
        if !input.jump_held && p.vel.y < -135. {
            p.vel.y = -135.;
        }
        if input.down && !p.ground && p.vel.y > 0. && p.drop_through.is_none() {
            p.slam = true;
            p.vel.y = 600.;
        }
        if input.heal && p.flasks > 0 && p.hp < p.max_hp && p.heal_time <= 0. {
            p.heal_time = 0.8;
        }
        if p.heal_time > 0. {
            p.heal_time -= dt;
            if p.heal_time <= 0. {
                p.flasks -= 1;
                p.hp = (p.hp + p.max_hp * 0.6).min(p.max_hp);
                self.sounds.push(Sfx::Heal);
                effects.push((Effect::Heal, p.pos - vec2(0., 14.), 0.));
            }
        }
        let speed = if p.heal_time > 0. {
            0.
        } else {
            input.axis * 145.
        };
        p.vel.x = if p.dodge > 0. {
            p.face * 370.
        } else {
            p.vel.x + (speed - p.vel.x) * (dt * 22.).min(1.)
        };
        p.vel.y = (p.vel.y + 850. * dt).min(650.);
        let old_y = p.pos.y;
        let landing_speed = p.vel.y;
        p.pos += p.vel * dt;
        p.pos.x = p.pos.x.clamp(12., self.level.width - 12.);
        p.ground = false;
        if let Some(y) = crossed_surface(
            &self.level.platforms,
            p.pos.x,
            6.,
            old_y,
            p.pos.y,
            p.drop_through,
        ) {
            p.pos.y = y;
            p.vel.y = 0.;
            p.ground = true;
        }
        if p.drop_through
            .is_some_and(|y| p.pos.y > y + 28. || p.pos.y < y - 2.)
        {
            p.drop_through = None;
        }
        let landed_slam = p.slam && p.ground;
        if landed_slam {
            p.slam = false;
        } else if !was_ground && p.ground && landing_speed > 60. {
            effects.push((Effect::Land(landing_speed), p.pos, 0.));
        }
        if p.ground && p.dodge <= 0. && p.heal_time <= 0. {
            self.footstep_distance += (p.pos.x - previous_pos.x).abs();
            if self.footstep_distance >= 19. {
                self.footstep_distance %= 19.;
                effects.push((Effect::Step, p.pos, -p.face));
            }
        } else {
            self.footstep_distance = 0.;
        }
        let mut melee = None;
        if input.attack && p.attack_cd <= 0. && p.dodge <= 0. && p.heal_time <= 0. {
            p.combo = if p.attack_cd == 0. && p.attack == 0. {
                (p.combo + 1) % 3
            } else {
                0
            };
            p.attack = 0.19;
            p.attack_cd = p.weapon.delay();
            melee = Some((
                p.pos,
                p.face,
                p.weapon.reach(),
                p.damage() * if p.combo == 2 { 1.4 } else { 1. },
            ));
            self.sounds.push(Sfx::Slash);
        }
        if input.bow && p.bow_cd <= 0. && p.heal_time <= 0. {
            p.bow_cd = 0.32;
            effects.push((Effect::Bolt, p.pos + vec2(p.face * 12., -16.), p.face));
            self.shots.push(Shot {
                pos: p.pos - vec2(-p.face * 12., 16.),
                vel: vec2(p.face * 430., 0.),
                life: 1.6,
                damage: 22. + p.power[1] as f32 * 6.,
                hostile: false,
                kind: 0,
            });
            self.sounds.push(Sfx::Slash);
        }
        if input.grenade && p.grenade_cd <= 0. {
            p.grenade_cd = 5.;
            self.shots.push(Shot {
                pos: p.pos - vec2(0., 22.),
                vel: vec2(p.face * 180., -210.),
                life: 0.85,
                damage: 90. + p.power[1] as f32 * 12.,
                hostile: false,
                kind: 1,
            });
        }
        if input.trap && p.trap_cd <= 0. {
            if let Some(y) = surface_below(&self.level.platforms, p.pos.x, p.pos.y) {
                p.trap_cd = 8.;
                let pos = vec2(p.pos.x, y);
                effects.push((Effect::Trap, pos, 0.));
                self.traps.push(Trap {
                    pos,
                    life: 7.,
                    tick: 0.,
                });
            }
        }
        for (effect, pos, dir) in effects {
            self.effect(effect, pos, dir);
        }
        if let Some((pos, dir, reach, dmg)) = melee {
            let hitbox = Rect::new(
                if dir > 0. { pos.x } else { pos.x - reach },
                pos.y - 38.,
                reach,
                43.,
            );
            let indices: Vec<_> = self
                .level
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.hp > 0. && hitbox.overlaps(&e.rect()))
                .map(|(i, _)| i)
                .collect();
            for i in indices {
                self.hit(i, dmg, dir, self.player.tier > 1);
                if self.settings.hitstop {
                    self.hitstop = 0.035;
                }
            }
        }
        if landed_slam {
            self.effect(Effect::Slam, self.player.pos, 0.);
            self.shake = 6.;
            let indices: Vec<_> = self
                .level
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.hp > 0. && (e.pos - self.player.pos).length() < 78.)
                .map(|(i, _)| i)
                .collect();
            for i in indices {
                self.hit(i, 55., self.player.face, false);
            }
        }
        self.update_enemies(dt);
        self.update_shots(dt);
        self.update_emitters(dt);
        let mut trap_hits = vec![];
        for t in &mut self.traps {
            t.life -= dt;
            t.tick -= dt;
            if t.tick <= 0. {
                t.tick = 0.4;
                for (i, e) in self.level.enemies.iter().enumerate() {
                    if e.hp > 0. && (e.pos - t.pos).length() < 105. {
                        trap_hits.push(i);
                    }
                }
            }
        }
        self.traps.retain(|t| t.life > 0.);
        for i in trap_hits {
            self.hit(i, 18., 0., true);
        }
        if self
            .level
            .hazards
            .iter()
            .any(|h| h.overlaps(&self.player.rect()))
        {
            self.hurt(12., -self.player.face);
        }
        if self.player.pos.y > self.level.max_y + 80. {
            self.player.pos = self.last_safe_pos;
            self.player.vel = Vec2::ZERO;
            self.player.ground = true;
            self.player.jumps = 0;
            self.player.slam = false;
            self.player.drop_through = None;
            self.player.invuln = self.player.invuln.max(0.8);
            self.player.buffer = 0.;
            self.player.dodge = 0.;
            self.camera_y =
                (self.player.pos.y - 230.).clamp(self.level.min_y, self.level.max_y - 360.);
            self.notify("The updraft carries you back to the last safe ledge.");
        } else if self.player.ground
            && !self
                .level
                .hazards
                .iter()
                .any(|r| r.overlaps(&self.player.rect()))
        {
            self.last_safe_pos = self.player.pos;
        }
        if input.interact && self.screen == Screen::Playing {
            self.interact();
        }
        let target = (self.player.pos.x - 250. + self.player.face * 35.)
            .clamp(0., (self.level.width - 640.).max(0.));
        self.camera += (target - self.camera) * (dt * 6.).min(1.);
        let screen_y = self.player.pos.y - self.camera_y;
        let target_y = if screen_y < 132. {
            self.player.pos.y - 132.
        } else if screen_y > 248. {
            self.player.pos.y - 248.
        } else {
            self.camera_y
        }
        .clamp(self.level.min_y, self.level.max_y - 360.);
        self.camera_y += (target_y - self.camera_y) * (dt * 7.).min(1.);
        self.camera_y = self
            .camera_y
            .clamp(self.level.min_y, self.level.max_y - 360.);
    }
    fn update_enemies(&mut self, dt: f32) {
        let pp = self.player.pos;
        let mut attacks = vec![];
        let mut shots = vec![];
        let mut parries = vec![];
        let mut burned_out = vec![];
        for (i, e) in self.level.enemies.iter_mut().enumerate() {
            if e.hp <= 0. {
                continue;
            }
            keep_enemy_on_tier(&self.level.platforms, e);
            e.flash = (e.flash - dt).max(0.);
            e.stun = (e.stun - dt).max(0.);
            if e.burn > 0. {
                e.burn -= dt;
                if e.hp <= dt * 8. {
                    burned_out.push(i);
                } else {
                    e.hp -= dt * 8.;
                }
            }
            if e.stun > 0. {
                continue;
            }
            let dx = pp.x - e.pos.x;
            let dy = pp.y - e.pos.y;
            e.face = dx.signum();
            e.timer -= dt;
            if e.windup > 0. {
                e.windup -= dt;
                if e.windup <= 0. {
                    e.timer = match e.kind {
                        EnemyKind::Regent => 0.85,
                        EnemyKind::Brute => 1.6,
                        _ => 1.15,
                    };
                    match e.kind {
                        EnemyKind::Archer => shots.push(Shot {
                            pos: e.pos - vec2(0., 20.),
                            vel: (pp - vec2(0., 15.) - (e.pos - vec2(0., 20.))).normalize_or_zero()
                                * 170.,
                            life: 3.,
                            damage: 13.,
                            hostile: true,
                            kind: 2,
                        }),
                        EnemyKind::Regent => {
                            e.phase += 1;
                            if e.phase % 3 == 0 {
                                for n in -2..=2 {
                                    shots.push(Shot {
                                        pos: e.pos - vec2(0., 34.),
                                        vel: vec2(e.face * 170., n as f32 * 48.),
                                        life: 3.,
                                        damage: 19.,
                                        hostile: true,
                                        kind: 2,
                                    });
                                }
                            } else {
                                if dx.abs() < 112. && dy.abs() < 70. {
                                    attacks.push((i, 25., e.face));
                                }
                                e.pos.x += e.face * 55.;
                            }
                        }
                        _ => {
                            if dx.abs() < if e.kind == EnemyKind::Brute { 66. } else { 43. }
                                && dy.abs() < 37.
                            {
                                attacks.push((
                                    i,
                                    if e.kind == EnemyKind::Brute { 22. } else { 12. },
                                    e.face,
                                ));
                            }
                        }
                    }
                }
                keep_enemy_on_tier(&self.level.platforms, e);
                continue;
            }
            let range = match e.kind {
                EnemyKind::Archer => 300.,
                EnemyKind::Regent => 180.,
                EnemyKind::Brute => 58.,
                _ => 36.,
            };
            if dx.abs() > 350. && e.kind != EnemyKind::Regent && e.kind != EnemyKind::Moth {
                e.pos.x = e.home + (self.time * 0.7 + e.home).sin() * 12.;
            }
            if dx.abs() < range
                && dy.abs()
                    < if e.kind == EnemyKind::Archer {
                        200.
                    } else {
                        80.
                    }
                && e.timer <= 0.
            {
                e.windup = match e.kind {
                    EnemyKind::Brute => 0.65,
                    EnemyKind::Regent => 0.62,
                    _ => 0.4,
                };
            } else if dx.abs() < 320. && dy.abs() < 150. && dx.abs() > range * 0.8 {
                if e.kind == EnemyKind::Moth {
                    e.pos += (pp - vec2(0., 12.) - e.pos).normalize_or_zero() * 55. * dt;
                } else if dy.abs() < 45. && e.kind != EnemyKind::Archer {
                    e.pos.x += e.face * if e.kind == EnemyKind::Brute { 28. } else { 46. } * dt;
                }
            }
            if e.kind == EnemyKind::Moth && (dx.abs() >= 320. || dy.abs() >= 150.) {
                let home = vec2(
                    e.home + (self.time * 1.1 + e.home).sin() * 18.,
                    e.home_y - 14. + (self.time * 2.2 + e.home).sin() * 9.,
                );
                e.pos += (home - e.pos) * (dt * 2.).min(1.);
            }
            keep_enemy_on_tier(&self.level.platforms, e);
        }
        for i in burned_out {
            self.hit(i, self.level.enemies[i].hp, 0., false);
        }
        self.shots.extend(shots);
        for (i, damage, dir) in attacks {
            if self.player.parry > 0. && self.player.face == -dir {
                parries.push(i);
            } else {
                self.hurt(damage, dir);
            }
        }
        for i in parries {
            self.hit(i, 38., self.player.face, false);
            self.level.enemies[i].stun = 1.8;
            self.player.parry_cd = 0.;
            self.sounds.push(Sfx::Parry);
            self.label(
                self.player.pos - vec2(0., 42.),
                "PARRY".into(),
                Color::from_hex(0x85fff0),
            );
            self.effect(
                Effect::Parry,
                self.player.pos + vec2(self.player.face * 13., -20.),
                self.player.face,
            );
        }
    }
    fn update_shots(&mut self, dt: f32) {
        let mut hits = vec![];
        let mut damage = vec![];
        let mut blasts = vec![];
        let mut reflections = vec![];
        let pr = self.player.rect();
        for s in &mut self.shots {
            s.life -= dt;
            let old_bottom = s.pos.y + 4.;
            s.pos += s.vel * dt;
            if s.kind == 1 {
                s.vel.y += 500. * dt;
                if let Some(y) = crossed_surface(
                    &self.level.platforms,
                    s.pos.x,
                    4.,
                    old_bottom,
                    s.pos.y + 4.,
                    None,
                ) {
                    s.pos.y = y - 4.;
                    s.vel.y = -s.vel.y * 0.5;
                    s.vel.x *= 0.7;
                }
                if s.life <= 0. {
                    blasts.push((s.pos, s.damage));
                }
                continue;
            }
            if s.hostile {
                if pr.contains(s.pos) {
                    if self.player.parry > 0. && self.player.face == -s.vel.x.signum() {
                        s.hostile = false;
                        s.vel = -s.vel * 1.8;
                        s.damage *= 3.;
                        self.sounds.push(Sfx::Parry);
                        reflections.push((s.pos, s.vel.x.signum()));
                    } else {
                        s.life = 0.;
                        damage.push((s.damage, s.vel.x.signum()));
                    }
                }
            } else {
                for (i, e) in self.level.enemies.iter().enumerate() {
                    if e.hp > 0. && e.rect().contains(s.pos) {
                        hits.push((i, s.damage, s.vel.x.signum()));
                        s.life = 0.;
                        break;
                    }
                }
            }
        }
        self.shots.retain(|s| s.life > 0.);
        for (pos, dir) in reflections {
            self.effect(Effect::Parry, pos, dir);
        }
        for (i, d, dir) in hits {
            self.hit(i, d, dir, false);
        }
        for (d, dir) in damage {
            self.hurt(d, dir);
        }
        for (pos, d) in blasts {
            self.effect(Effect::Explosion, pos, 0.);
            self.shake = 8.;
            self.sounds.push(Sfx::Explosion);
            let ids: Vec<_> = self
                .level
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.hp > 0. && (e.pos - pos).length() < 105.)
                .map(|(i, _)| i)
                .collect();
            for i in ids {
                self.hit(i, d, 0., true);
            }
        }
    }
    fn update_emitters(&mut self, dt: f32) {
        // Fixed simulation cadence, not render cadence. Dormant and offscreen entities emit nothing.
        self.effect_clock += dt;
        if self.effect_clock < 0.05 {
            return;
        }
        self.effect_clock %= 0.05;
        let mut effects = Vec::with_capacity(20);
        for shot in self
            .shots
            .iter()
            .filter(|s| {
                s.pos.x > self.camera - 40.
                    && s.pos.x < self.camera + 680.
                    && s.pos.y > self.camera_y - 40.
                    && s.pos.y < self.camera_y + 400.
            })
            .take(12)
        {
            effects.push((
                Effect::Projectile {
                    hostile: shot.hostile,
                    grenade: shot.kind == 1,
                },
                shot.pos,
                -shot.vel.x.signum(),
            ));
        }
        for enemy in self
            .level
            .enemies
            .iter()
            .filter(|e| {
                e.hp > 0.
                    && e.burn > 0.
                    && e.pos.x > self.camera - 40.
                    && e.pos.x < self.camera + 680.
                    && e.pos.y > self.camera_y - 40.
                    && e.pos.y < self.camera_y + 400.
            })
            .take(8)
        {
            effects.push((Effect::Burn, enemy.pos - vec2(0., 16.), 0.));
        }
        let p = &self.player;
        if p.heal_time > 0. {
            effects.push((Effect::HealCast, p.pos - vec2(0., 13.), 0.));
        }
        if p.dodge > 0. {
            effects.push((Effect::DodgeTrail, p.pos - vec2(p.face * 8., 4.), -p.face));
        }
        for (effect, pos, dir) in effects {
            self.effect(effect, pos, dir);
        }
    }
    pub fn nearby(&self) -> Option<usize> {
        self.level
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| !o.used && (o.pos - self.player.pos).length() < 42.)
            .min_by(|a, b| {
                (a.1.pos - self.player.pos)
                    .length_squared()
                    .total_cmp(&(b.1.pos - self.player.pos).length_squared())
            })
            .map(|(i, _)| i)
    }
    fn interact(&mut self) {
        let Some(i) = self.nearby() else { return };
        let kind = self.level.objects[i].kind;
        let pos = self.level.objects[i].pos;
        match kind {
            ObjectKind::Chest => {
                self.player.weapon = match self.rng.range(0, 3) {
                    0 => Weapon::Sabre,
                    1 => Weapon::Glaive,
                    _ => Weapon::Hammer,
                };
                self.player.tier += 1;
                self.notify(&format!(
                    "{} +{}  /  Scorching edge",
                    self.player.weapon.name(),
                    self.player.tier
                ));
            }
            ObjectKind::Scroll => {
                self.screen = Screen::Scroll;
            }
            ObjectKind::Fountain => {
                self.player.hp = self.player.max_hp;
                self.player.flasks = 2 + self.save.flask;
                self.notify("The well remembers you. Health and flasks restored.");
            }
            ObjectKind::Forge => {
                if self.player.gold < 60 {
                    self.notify("The smith asks for 60 copper to temper your weapon.");
                    return;
                }
                self.player.gold -= 60;
                self.player.tier += 1;
                self.notify("Weapon tempered. Damage increased; attacks ignite enemies.");
            }
            ObjectKind::Lore => {
                self.notify(
                    "\"We built the sun a cage. Then wondered why it burned.\" - The Keeper",
                );
            }
            ObjectKind::Secret => {
                if !self.save.rune && self.player.kills < 8 {
                    self.notify(
                        "A sealed cache. Defeat 8 guardians, or return with the Crown Rune.",
                    );
                    return;
                }
                self.player.embers += 15;
                self.player.gold += 100;
                self.notify("Hidden cache opened: 15 embers and 100 copper.");
            }
            ObjectKind::Exit => {
                if self.level.biome == Biome::Crown {
                    if self
                        .level
                        .enemies
                        .iter()
                        .any(|e| e.kind == EnemyKind::Regent && e.hp > 0.)
                    {
                        self.notify("The Brass Regent holds the gate shut.");
                        return;
                    }
                    self.save.wins += 1;
                    self.save.embers += self.player.embers;
                    self.player.embers = 0;
                    self.save.best_kills = self.save.best_kills.max(self.player.kills);
                    self.persist();
                    self.screen = Screen::Victory;
                } else {
                    self.save.embers += self.player.embers;
                    self.player.embers = 0;
                    self.persist();
                    self.screen = Screen::Camp;
                }
            }
        }
        self.level.objects[i].used = true;
        self.sounds.push(Sfx::Loot);
        self.effect(Effect::Loot, pos - vec2(0., 15.), 0.);
    }
    pub fn upgrade(&mut self, choice: usize) {
        self.player.power[choice] += 1;
        let hp = if choice == 2 { 24. } else { 10. };
        self.player.max_hp += hp;
        self.player.hp += hp;
        self.screen = Screen::Playing;
        self.notify(
            [
                "FEROCITY rises. Melee damage increased.",
                "INGENUITY rises. Ranged and grenade damage increased.",
                "RESOLVE rises. Maximum vitality increased.",
            ][choice],
        );
    }
    pub fn buy(&mut self, choice: usize) {
        let cost = match choice {
            0 => 10 + self.save.vitality * 8,
            1 => 20 + self.save.flask * 20,
            _ => 15,
        };
        if self.save.embers < cost {
            self.notify("Not enough banked embers.");
            return;
        }
        if choice == 1 && self.save.flask >= 3 {
            self.notify("The flask is fully reinforced.");
            return;
        }
        self.save.embers -= cost;
        match choice {
            0 => {
                self.save.vitality += 1;
                self.player.max_hp += 15.;
                self.player.hp += 15.;
            }
            1 => {
                self.save.flask += 1;
                self.player.flasks += 1;
            }
            _ => {
                self.player.mutation = if self.player.mutation == 1 { 2 } else { 1 };
            }
        }
        self.persist();
        self.notify("The Keeper binds your choice.");
    }
    pub fn travel(&mut self) {
        self.stage += 1;
        let biome = if self.stage >= 2 {
            Biome::Crown
        } else if self.route == 0 {
            Biome::Garden
        } else {
            Biome::Foundry
        };
        self.level = Level::generate(self.seed + self.stage as u64 * 53, biome, self.save.wins);
        self.player.pos = self.level.spawn;
        self.player.vel = Vec2::ZERO;
        self.player.ground = true;
        self.player.jumps = 0;
        self.player.buffer = 0.;
        self.player.coyote = 0.;
        self.player.slam = false;
        self.player.drop_through = None;
        self.player.dodge = 0.;
        self.player.heal_time = 0.;
        self.last_safe_pos = self.level.spawn;
        self.player.hp = self.player.max_hp;
        self.player.flasks = 2 + self.save.flask;
        self.shots.clear();
        self.traps.clear();
        self.particles.clear();
        self.effect_clock = 0.;
        self.footstep_distance = 0.;
        self.camera = 0.;
        self.camera_y =
            (self.level.spawn.y - 248.).clamp(self.level.min_y, self.level.max_y - 360.);
        self.intro = 4.;
        self.screen = Screen::Playing;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::new(42, Save::default());
        g.practice = true;
        g.screen = Screen::Playing;
        g.level.enemies.clear();
        g
    }
    #[test]
    fn visual_effects_do_not_advance_gameplay_rng() {
        let mut a = game();
        let mut b = game();
        for _ in 0..40 {
            a.effect(Effect::Explosion, vec2(100., FLOOR), 0.);
        }
        for _ in 0..20 {
            assert_eq!(a.rng.next(), b.rng.next());
        }
    }
    #[test]
    fn impact_debris_collides_with_the_elevated_platform_under_its_source() {
        let mut g = game();
        g.level.platforms = vec![Rect::new(400., 219., 140., 9.)];
        g.effect(Effect::Slam, vec2(460., 219.), 0.);
        for _ in 0..120 {
            particles::update(&mut g.particles, STEP);
            assert!(g
                .particles
                .iter()
                .filter(|p| p.kind == particles::Kind::Shard)
                .all(|p| p.pos.y <= 219.));
        }
    }
    #[test]
    fn menus_freeze_effects_and_simulation_at_the_game_boundary() {
        let mut g = game();
        g.effect(Effect::Explosion, vec2(100., FLOOR), 0.);
        let first = g.particles[0].clone();
        for screen in [
            Screen::Paused,
            Screen::Scroll,
            Screen::Camp,
            Screen::Options,
        ] {
            g.screen = screen;
            g.tick(
                STEP,
                Input {
                    jump: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.time, 0.);
            assert_eq!(g.particles[0].pos, first.pos);
            assert_eq!(g.particles[0].life, first.life);
        }
    }
    #[test]
    fn jumping_and_landing_emit_once_but_idle_does_not_emit() {
        let mut g = game();
        for _ in 0..120 {
            g.tick(STEP, Input::default());
        }
        assert!(g.particles.is_empty());
        g.tick(
            STEP,
            Input {
                jump: true,
                jump_held: true,
                ..Default::default()
            },
        );
        assert!(g.particles.iter().any(|p| p.kind == particles::Kind::Dust));
        g.particles.clear();
        let mut landed = false;
        for _ in 0..120 {
            g.tick(
                STEP,
                Input {
                    jump_held: true,
                    ..Default::default()
                },
            );
            if g.player.ground {
                landed = true;
                assert!(g.particles.iter().any(|p| p.kind == particles::Kind::Dust));
                break;
            }
        }
        assert!(landed);
        let count = g.particles.len();
        g.tick(STEP, Input::default());
        assert_eq!(g.particles.len(), count);
        for _ in 0..120 {
            g.tick(STEP, Input::default());
        }
        assert!(g.particles.is_empty());
    }
    #[test]
    fn projectile_trails_and_reflection_emit_distinct_cues() {
        let mut g = game();
        g.shots.push(Shot {
            pos: vec2(150., 170.),
            vel: vec2(200., 0.),
            life: 1.,
            damage: 5.,
            hostile: false,
            kind: 0,
        });
        for _ in 0..8 {
            g.tick(STEP, Input::default());
        }
        assert!(g.particles.iter().any(|p| p.kind == particles::Kind::Mote));
        g.particles.clear();
        g.shots.clear();
        g.player.parry = 0.2;
        g.shots.push(Shot {
            pos: g.player.pos + vec2(4., -15.),
            vel: vec2(-80., 0.),
            life: 1.,
            damage: 10.,
            hostile: true,
            kind: 2,
        });
        g.tick(STEP, Input::default());
        assert!(g.particles.iter().any(|p| p.kind == particles::Kind::Flash));
        assert!(g.particles.iter().any(|p| p.kind == particles::Kind::Ring));
        assert_eq!(g.player.hp, 100.);
    }
    #[test]
    fn double_jump_reaches_and_lands_on_upper_platform() {
        let mut g = game();
        g.level.platforms = vec![
            Rect::new(0., FLOOR, 800., 20.),
            Rect::new(400., 219., 140., 12.),
        ];
        g.player.pos.x = 460.;
        for step in 0..180 {
            g.tick(
                STEP,
                Input {
                    jump: step == 0 || step == 29,
                    jump_held: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(g.player.pos.y, 219.);
        assert!(g.player.ground);
    }
    #[test]
    fn burn_kills_and_rewards_once() {
        let mut g = game();
        let mut e = Enemy::new(500., FLOOR, EnemyKind::Warden, 0);
        e.hp = 0.01;
        e.burn = 1.;
        g.level.enemies.push(e);
        g.tick(STEP, Input::default());
        assert_eq!(g.player.kills, 1);
        assert_eq!(g.player.embers, 3);
        g.tick(STEP, Input::default());
        assert_eq!(g.player.kills, 1);
    }
    #[test]
    fn flask_requires_uninterrupted_cast() {
        let mut g = game();
        g.player.hp = 40.;
        g.tick(
            STEP,
            Input {
                heal: true,
                ..Default::default()
            },
        );
        g.hurt(10., 1.);
        for _ in 0..120 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.player.flasks, 2);
        assert_eq!(g.player.hp, 30.);
        g.tick(
            STEP,
            Input {
                heal: true,
                ..Default::default()
            },
        );
        for _ in 0..120 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.player.flasks, 1);
        assert_eq!(g.player.hp, 90.);
    }
    #[test]
    fn reflected_projectile_changes_allegiance() {
        let mut g = game();
        g.player.parry = 0.2;
        g.shots.push(Shot {
            pos: g.player.pos - vec2(-4., 15.),
            vel: vec2(-80., 0.),
            life: 1.,
            damage: 10.,
            hostile: true,
            kind: 2,
        });
        g.tick(STEP, Input::default());
        assert!(!g.shots[0].hostile);
        assert!(g.shots[0].vel.x > 0.);
        assert_eq!(g.player.hp, 100.);
    }
    #[test]
    fn cannot_exit_after_lethal_damage_in_same_tick() {
        let mut g = game();
        g.player.pos = g
            .level
            .objects
            .iter()
            .find(|o| o.kind == ObjectKind::Exit)
            .unwrap()
            .pos;
        g.player.hp = 1.;
        g.shots.push(Shot {
            pos: g.player.pos - vec2(0., 12.),
            vel: Vec2::ZERO,
            life: 1.,
            damage: 20.,
            hostile: true,
            kind: 2,
        });
        g.tick(
            STEP,
            Input {
                interact: true,
                ..Default::default()
            },
        );
        assert_eq!(g.screen, Screen::Dead);
    }
    #[test]
    fn jump_lands_on_floor() {
        let mut g = game();
        g.tick(
            STEP,
            Input {
                jump: true,
                jump_held: true,
                ..Default::default()
            },
        );
        assert!(g.player.vel.y < 0.);
        for _ in 0..240 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.player.pos.y, FLOOR);
        assert!(g.player.ground);
    }
    #[test]
    fn dodge_is_invulnerable_and_has_cooldown() {
        let mut g = game();
        g.tick(
            STEP,
            Input {
                dodge: true,
                ..Default::default()
            },
        );
        g.hurt(50., 1.);
        assert_eq!(g.player.hp, 100.);
        for _ in 0..36 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.player.dodge, 0.);
        assert!(g.player.dodge_cd > 0.);
        g.hurt(20., 1.);
        assert_eq!(g.player.hp, 80.);
    }
    #[test]
    fn melee_only_hits_facing_range() {
        let mut g = game();
        g.level.enemies = vec![
            Enemy::new(140., FLOOR, EnemyKind::Warden, 0),
            Enemy::new(60., FLOOR, EnemyKind::Warden, 0),
        ];
        g.tick(
            STEP,
            Input {
                attack: true,
                ..Default::default()
            },
        );
        assert!(g.level.enemies[0].hp < 65.);
        assert_eq!(g.level.enemies[1].hp, 65.);
    }
    #[test]
    fn hitstop_setting_controls_the_impact_freeze() {
        for enabled in [true, false] {
            let mut g = game();
            g.settings.hitstop = enabled;
            g.level.enemies = vec![Enemy::new(140., FLOOR, EnemyKind::Warden, 0)];
            g.tick(
                STEP,
                Input {
                    attack: true,
                    ..Default::default()
                },
            );
            assert!(g.level.enemies[0].hp < 65.);
            assert_eq!(g.hitstop > 0., enabled);
        }
    }
    #[test]
    fn abandoning_needs_confirmation_and_costs_like_a_death() {
        let mut g = game();
        g.save.embers = 30;
        g.player.embers = 12;
        g.request_abandon();
        assert_eq!(g.screen, Screen::Playing, "only the pause screen abandons");
        g.screen = Screen::Paused;
        g.request_abandon();
        assert_eq!(g.screen, Screen::Paused);
        assert!(g.abandon_armed);
        g.request_abandon();
        assert_eq!(g.screen, Screen::Dead);
        assert_eq!(g.player.embers, 0);
        assert_eq!(g.save.embers, 30);
    }
    #[test]
    fn options_return_to_their_origin_and_survive_a_new_run() {
        let mut g = game();
        g.screen = Screen::Paused;
        g.abandon_armed = true;
        g.open_options();
        assert_eq!(g.screen, Screen::Options);
        assert!(!g.abandon_armed);
        g.settings.adjust(2, -1);
        g.close_options();
        assert_eq!(g.screen, Screen::Paused);
        g.start();
        assert_eq!(g.settings.shake, 9);
        g.screen = Screen::Playing;
        g.open_options();
        assert_eq!(g.screen, Screen::Playing, "options open only from menus");
    }
    fn tutor() -> Game {
        let mut g = game();
        g.teach = true;
        g.intro = 0.;
        g
    }
    fn idle(g: &mut Game, seconds: f32) {
        for _ in 0..(seconds / STEP) as u32 {
            g.tick(STEP, Input::default());
        }
    }
    #[test]
    fn each_tip_fires_once_when_its_mechanic_matters() {
        let mut g = tutor();
        g.level.platforms.retain(|r| r.y >= FLOOR);
        idle(&mut g, 0.5);
        assert!(g.hint.is_none(), "nothing relevant yet");
        g.level.enemies = vec![Enemy::new(
            g.player.pos.x + 120.,
            FLOOR,
            EnemyKind::Archer,
            0,
        )];
        g.player.hp = g.player.max_hp * 0.5;
        g.tick(STEP, Input::default());
        assert_eq!(g.hint.map(|h| h.0), Some(Hint::Strike));
        assert!(g.settings.hints_seen & Hint::Strike.bit() != 0);
        g.level.enemies.clear();
        idle(&mut g, HINT_SECONDS + 0.1);
        assert_eq!(g.hint.map(|h| h.0), Some(Hint::Heal), "queued tips follow");
        idle(&mut g, HINT_SECONDS + 0.1);
        assert!(g.hint.is_none());
        // A shown tip never returns, even after the settings round-trip a restart does.
        let stored = serde_json::to_vec(&g.settings).unwrap();
        g.settings = serde_json::from_slice(&stored).unwrap();
        g.start();
        g.intro = 0.;
        g.level.platforms.retain(|r| r.y >= FLOOR);
        g.level.enemies = vec![Enemy::new(
            g.player.pos.x + 120.,
            FLOOR,
            EnemyKind::Warden,
            0,
        )];
        g.player.hp = g.player.max_hp * 0.5;
        idle(&mut g, 1.);
        assert!(g.hint.is_none());
    }
    #[test]
    fn kills_and_hostile_bolts_queue_their_tips() {
        let mut g = tutor();
        g.level.enemies = vec![Enemy::new(400., FLOOR, EnemyKind::Moth, 0)];
        g.hit(0, 1000., 1., false);
        g.tick(STEP, Input::default());
        assert_eq!(g.hint.map(|h| h.0), Some(Hint::Tools));
        g.hint = None;
        g.shots.push(Shot {
            pos: g.player.pos + vec2(150., -12.),
            vel: vec2(-200., 0.),
            life: 2.,
            damage: 13.,
            hostile: true,
            kind: 0,
        });
        g.tick(STEP, Input::default());
        assert_eq!(g.hint.map(|h| h.0), Some(Hint::Parry));
    }
    #[test]
    fn ledges_above_and_underfoot_teach_climbing_and_dropping() {
        let mut g = tutor();
        let x = g.player.pos.x;
        g.level.platforms.retain(|r| r.y >= FLOOR);
        g.level
            .platforms
            .push(Rect::new(x - 40., FLOOR - 60., 80., 10.));
        g.tick(STEP, Input::default());
        assert_eq!(g.hint.map(|h| h.0), Some(Hint::Climb));
        g.hint = None;
        g.level
            .platforms
            .push(Rect::new(x - 40., Level::UPPER, 80., 10.));
        g.player.pos.y = Level::UPPER;
        g.tick(STEP, Input::default());
        assert_eq!(g.hint.map(|h| h.0), Some(Hint::Drop));
    }
    #[test]
    fn tips_stay_silent_when_disabled_or_practising() {
        for (teach, enabled) in [(false, true), (true, false)] {
            let mut g = tutor();
            g.teach = teach;
            g.settings.hints = enabled;
            g.level.enemies = vec![Enemy::new(
                g.player.pos.x + 120.,
                FLOOR,
                EnemyKind::Warden,
                0,
            )];
            idle(&mut g, 0.5);
            assert!(g.hint.is_none());
            assert_eq!(g.settings.hints_seen, 0);
        }
    }
    #[test]
    fn death_loses_unbanked_embers_only() {
        let mut g = game();
        g.save.embers = 30;
        g.player.embers = 12;
        g.hurt(1000., 1.);
        assert_eq!(g.screen, Screen::Dead);
        assert_eq!(g.player.embers, 0);
        assert_eq!(g.save.embers, 30);
    }
    #[test]
    fn exits_bank_and_boss_locks_gate() {
        let mut g = game();
        g.player.embers = 12;
        g.player.pos = g
            .level
            .objects
            .iter()
            .find(|o| o.kind == ObjectKind::Exit)
            .unwrap()
            .pos;
        g.interact();
        assert_eq!(g.screen, Screen::Camp);
        assert_eq!(g.save.embers, 12);
        g.stage = 1;
        g.travel();
        g.player.pos = g
            .level
            .objects
            .iter()
            .find(|o| o.kind == ObjectKind::Exit)
            .unwrap()
            .pos;
        g.interact();
        assert_eq!(g.screen, Screen::Playing);
        g.level
            .enemies
            .iter_mut()
            .find(|e| e.kind == EnemyKind::Regent)
            .unwrap()
            .hp = 0.;
        g.interact();
        assert_eq!(g.screen, Screen::Victory);
    }
    #[test]
    fn parry_stuns_attacker() {
        let mut g = game();
        let mut e = Enemy::new(g.player.pos.x + 25., FLOOR, EnemyKind::Warden, 0);
        e.windup = 0.001;
        g.level.enemies.push(e);
        g.tick(
            STEP,
            Input {
                parry: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.hp, 100.);
        assert!(g.level.enemies[0].stun > 1.5);
    }
    #[test]
    fn falling_lands_on_nearest_crossed_top_regardless_of_platform_order() {
        for reverse in [false, true] {
            let mut g = game();
            g.level.platforms = vec![
                Rect::new(0., 100., 300., 12.),
                Rect::new(0., 50., 300., 12.),
            ];
            if reverse {
                g.level.platforms.reverse();
            }
            g.player.pos = vec2(100., 0.);
            g.player.vel.y = 650.;
            g.player.ground = false;
            g.tick(0.2, Input::default());
            assert_eq!(g.player.pos.y, 50.);
            assert!(g.player.ground);
        }
        assert_eq!(
            crossed_surface(&[Rect::new(0., 100., 300., 12.)], 100., 6., 120., 80., None),
            None,
            "one-way ledges must not block ascent from below"
        );
    }
    #[test]
    fn down_jump_drops_one_ledge_without_spending_a_jump_or_starting_a_slam() {
        let mut g = game();
        g.level.platforms = vec![
            Rect::new(0., 100., 300., 12.),
            Rect::new(0., 220., 300., 12.),
        ];
        g.player.pos = vec2(100., 100.);
        g.tick(
            STEP,
            Input {
                down: true,
                jump: true,
                ..Default::default()
            },
        );
        assert!(!g.player.ground);
        assert!(g.player.pos.y > 100.);
        assert_eq!(g.player.jumps, 0);
        assert!(!g.player.slam);
        assert!(!g.sounds.iter().any(|s| matches!(s, Sfx::Jump)));
        for _ in 0..120 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.player.pos.y, 220.);
        assert!(g.player.ground);
        g.tick(
            STEP,
            Input {
                down: true,
                jump: true,
                ..Default::default()
            },
        );
        assert_eq!(
            g.player.pos.y, 220.,
            "the deepest floor cannot be dropped through"
        );
        assert!(g.player.ground);
    }
    #[test]
    fn grenade_bounces_and_trap_plants_on_the_current_upper_tier() {
        let mut g = game();
        g.level.platforms = vec![
            Rect::new(0., -50., 300., 12.),
            Rect::new(0., 622., 300., 12.),
        ];
        g.player.pos = vec2(100., -50.);
        g.shots.push(Shot {
            pos: vec2(150., -65.),
            vel: vec2(0., 400.),
            life: 1.,
            damage: 5.,
            hostile: false,
            kind: 1,
        });
        g.update_shots(0.05);
        assert_eq!(g.shots[0].pos.y, -54.);
        assert!(g.shots[0].vel.y < 0.);
        g.tick(
            STEP,
            Input {
                trap: true,
                ..Default::default()
            },
        );
        assert_eq!(g.traps[0].pos, vec2(100., -50.));
        g.player.pos.y = 622.;
        g.player.trap_cd = 0.;
        g.tick(
            STEP,
            Input {
                trap: true,
                ..Default::default()
            },
        );
        assert_eq!(g.traps[1].pos.y, 622.);
    }
    #[test]
    fn camera_tracks_upper_and_lower_tiers_but_ignores_small_vertical_motion() {
        let mut g = game();
        g.level.platforms = vec![Rect::new(0., 220., 300., 12.)];
        g.player.pos = vec2(100., 220.);
        g.camera_y = 0.;
        g.tick(STEP, Input::default());
        assert_eq!(
            g.camera_y, 0.,
            "vertical movement inside the dead zone is stable"
        );
        for (y, expected) in [(-180., g.level.min_y), (700., g.level.max_y - 360.)] {
            g.level.platforms[0].y = y;
            g.player.pos.y = y;
            for _ in 0..240 {
                g.tick(STEP, Input::default());
            }
            assert!((g.camera_y - expected).abs() < 0.01);
        }
    }
    #[test]
    fn enemies_cannot_chase_or_be_knocked_off_their_home_ledge() {
        let mut g = game();
        g.level.platforms = vec![
            Rect::new(0., FLOOR, 400., 12.),
            Rect::new(450., -50., 100., 12.),
        ];
        g.level.enemies = vec![Enemy::new(500., -50., EnemyKind::Warden, 0)];
        g.player.pos = vec2(750., -50.);
        for _ in 0..600 {
            g.update_enemies(STEP);
        }
        assert_eq!(g.level.enemies[0].pos, vec2(540., -50.));
        g.hit(0, 1., 1., false);
        assert_eq!(g.level.enemies[0].pos, vec2(540., -50.));
    }
    #[test]
    fn interaction_requires_matching_height_and_travel_resets_vertical_state() {
        let mut g = game();
        g.level.objects = vec![Object {
            pos: vec2(100., -50.),
            kind: ObjectKind::Chest,
            used: false,
        }];
        g.player.pos = vec2(100., FLOOR);
        assert_eq!(g.nearby(), None);
        g.player.pos.y = -50.;
        assert_eq!(g.nearby(), Some(0));
        g.player.slam = true;
        g.player.drop_through = Some(-50.);
        g.player.vel = vec2(100., 500.);
        g.travel();
        assert_eq!(g.player.pos, g.level.spawn);
        assert_eq!(g.player.vel, Vec2::ZERO);
        assert_eq!(g.player.drop_through, None);
        assert!(!g.player.slam);
        assert!(g.player.ground);
        assert!((g.player.pos.y - g.camera_y - 248.).abs() < 1.);
    }
    #[test]
    fn falling_outside_the_level_returns_to_the_last_safe_ledge() {
        let mut g = game();
        g.level.platforms = vec![Rect::new(0., 100., 300., 12.)];
        g.player.pos = vec2(150., 100.);
        g.tick(STEP, Input::default());
        g.player.pos = vec2(400., g.level.max_y + 90.);
        g.player.ground = false;
        g.player.vel.y = 600.;
        g.tick(STEP, Input::default());
        assert_eq!(g.player.pos, vec2(150., 100.));
        assert_eq!(g.player.vel, Vec2::ZERO);
        assert!(g.player.ground);
        assert_eq!(g.screen, Screen::Playing);
        assert!(g.notice.contains("last safe ledge"));
    }
    #[test]
    fn normal_inputs_reach_the_underground_from_the_surface() {
        let mut g = game();
        for _ in 0..300 {
            if g.player.pos.x >= 350. {
                break;
            }
            g.tick(
                STEP,
                Input {
                    axis: 1.,
                    ..Default::default()
                },
            );
        }
        assert_eq!(g.player.pos.y, FLOOR);
        g.tick(
            STEP,
            Input {
                down: true,
                jump: true,
                ..Default::default()
            },
        );
        for _ in 0..310 {
            g.tick(
                STEP,
                Input {
                    axis: 1.,
                    ..Default::default()
                },
            );
        }
        for _ in 0..90 {
            g.tick(STEP, Input::default());
        }
        assert!(
            g.player.pos.y > FLOOR + 300.,
            "expected underground, got {:?}",
            g.player.pos
        );
        assert!(g.player.ground);
        assert!(g.camera_y > 300.);
        assert_eq!(g.player.hp, g.player.max_hp);
    }
    #[test]
    fn held_single_jumps_climb_the_authored_staircase_in_every_biome() {
        for biome in [Biome::Aqueduct, Biome::Garden, Biome::Foundry, Biome::Crown] {
            let mut g = game();
            g.level = Level::generate(42, biome, 0);
            g.level.enemies.clear();
            g.player.pos = g.level.spawn;
            let targets = g.level.traversal.clone();
            for target in targets {
                let mut arrived = false;
                for _ in 0..300 {
                    let delta = target - g.player.pos;
                    if delta.x.abs() < 12. && delta.y.abs() < 1. && g.player.ground {
                        arrived = true;
                        break;
                    }
                    g.tick(
                        STEP,
                        Input {
                            axis: if delta.x.abs() > 3. {
                                delta.x.signum()
                            } else {
                                0.
                            },
                            jump: g.player.ground && delta.y < -2.,
                            jump_held: true,
                            ..Default::default()
                        },
                    );
                    assert!(
                        g.player.jumps <= 1,
                        "staircase must not require double jumps"
                    );
                }
                assert!(
                    arrived,
                    "{biome:?}: failed to reach {target:?} from {:?}",
                    g.player.pos
                );
                if target.y == Level::UPPER {
                    break;
                }
            }
            assert_eq!(g.player.pos.y, Level::UPPER);
            assert!(g.camera_y < -100.);
            assert_eq!(g.player.hp, g.player.max_hp);
        }
    }
}
