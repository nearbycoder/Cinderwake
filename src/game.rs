pub use crate::particles::Particle;
use crate::{
    controls::{Action, Prompts},
    particles::{self, Effect, VisualRng},
    save::{Checkpoint, Save, Unreadable},
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
    /// Choosing between a reliquary's weapon and the one in hand.
    Reliquary,
    /// Rebinding gameplay keys, opened from the options page.
    Controls,
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
    /// The tip, naming the player's current keys or controller buttons.
    pub fn text(self, p: &Prompts) -> String {
        let k = |a| p.action(a);
        // Mouse alternatives are only worth mentioning to keyboard players.
        let click = |button: &str| {
            if p.pad() {
                String::new()
            } else {
                format!(" or {button} click")
            }
        };
        match self {
            Self::Climb => format!(
                "{} jumps. Press it again in the air to double jump onto higher ledges.",
                k(Action::Jump)
            ),
            Self::Strike => format!(
                "{}{} strikes; a combo's second blow staggers guardians. {} dodges their strikes.",
                k(Action::Strike),
                click("left"),
                k(Action::Dodge)
            ),
            Self::Parry => format!(
                "Face a bolt and press {}{} to parry it back at the shooter.",
                k(Action::Parry),
                click("right")
            ),
            Self::Drop => format!(
                "{} + {} drops through a ledge. Press {} in mid-air to slam down.",
                k(Action::Down),
                k(Action::Jump),
                k(Action::Down)
            ),
            Self::Heal => format!(
                "{} drinks a healing flask. Taking damage interrupts the drink.",
                k(Action::Heal)
            ),
            Self::Tools => format!(
                "{} glassbolt, {} fire vessel, {} arc snare. Embers are lost on death until banked at a bellgate.",
                k(Action::Glassbolt),
                k(Action::FireVessel),
                k(Action::ArcSnare)
            ),
        }
    }
}
pub const HINT_SECONDS: f32 = 6.;
/// Seconds before the death and victory screens accept confirming.
pub const RESULT_DELAY: f32 = 1.;
/// Shortest gap between two windup tells.
pub const TELL_GAP: f32 = 0.2;
/// What ended a run.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cause {
    Strike(EnemyKind),
    Bolt(EnemyKind),
    Hazard,
    Abandoned,
}
impl Cause {
    pub fn text(self) -> &'static str {
        match self {
            Self::Strike(EnemyKind::Warden) => "Cut down by a warden",
            Self::Strike(EnemyKind::Brute) => "Crushed by a brute",
            Self::Strike(EnemyKind::Moth) => "Brought down by a clockwork moth",
            Self::Strike(EnemyKind::Archer) => "Struck down by an archer",
            Self::Strike(EnemyKind::Regent) => "Struck down by the Brass Regent",
            Self::Bolt(EnemyKind::Regent) => "Shot down by the Brass Regent",
            Self::Bolt(_) => "Shot down by an archer",
            Self::Hazard => "Caught on the spikes",
            Self::Abandoned => "The descent was abandoned",
        }
    }
}
/// The run that just ended, shown on the death and victory screens.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Recap {
    /// What ended the run; `None` for a victory.
    pub cause: Option<Cause>,
    pub biome: Biome,
    pub stage: u32,
    /// Carried embers lost on death, or banked at the final gate.
    pub embers: u32,
    /// Records this run beat.
    pub new_kills: bool,
    pub new_stage: bool,
    pub new_time: bool,
}
impl Screen {
    /// Menus that hold the world still and discard gameplay input.
    pub fn freezes_world(self) -> bool {
        matches!(
            self,
            Self::Paused
                | Self::Scroll
                | Self::Camp
                | Self::Options
                | Self::Controls
                | Self::Reliquary
        )
    }
}
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
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
    pub fn summary(self) -> &'static str {
        match self {
            Self::Sabre => "Quick, close cuts",
            Self::Glaive => "Long reach, measured swings",
            Self::Hammer => "Crushing, slow blows",
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
    /// Presses made just before their action was ready, kept until it is.
    pub queued: Queued,
    /// Whether strike and glassbolt were held last step, so a fresh press
    /// can be told apart from a hold.
    pub held: [bool; 2],
    /// Seconds left of each HUD slot's mark for a press that couldn't
    /// happen, indexed by `Slot`.
    pub refused: [f32; 7],
    /// Why the flask was refused: "EMPTY" or "FULL".
    pub flask_note: &'static str,
}
/// The HUD slots a refused press marks.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Slot {
    Strike,
    Glassbolt,
    FireVessel,
    ArcSnare,
    Flask,
    Parry,
    Dodge,
}
/// Copper the forge asks to temper a weapon.
pub const FORGE_COST: u32 = 60;
/// Guardians to fell before a sealed cache opens without the Crown Rune.
pub const CACHE_KILLS: u32 = 8;
/// How long a refused press marks its HUD slot.
pub const REFUSAL_SHOW: f32 = 0.45;
/// How long before an action is ready a press of it is kept rather than
/// dropped. Jumping keeps its own, shorter buffer (`Player::buffer`).
pub const PRESS_BUFFER: f32 = 0.15;
/// Seconds each early press has left before it's dropped.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Queued {
    pub dodge: f32,
    pub parry: f32,
    pub strike: f32,
    pub bolt: f32,
    pub grenade: f32,
    pub trap: f32,
}
impl Queued {
    fn tick(&mut self, dt: f32) {
        for t in [
            &mut self.dodge,
            &mut self.parry,
            &mut self.strike,
            &mut self.bolt,
            &mut self.grenade,
            &mut self.trap,
        ] {
            *t = (*t - dt).max(0.);
        }
    }
}
/// Keeps a press of an action that is `wait` seconds from ready, if that's
/// within `PRESS_BUFFER`. Returns false for a press too early to keep.
fn queue(timer: &mut f32, pressed: bool, wait: f32) -> bool {
    if !pressed || wait <= 0. {
        return true;
    }
    if wait > PRESS_BUFFER {
        return false;
    }
    // A little longer than the wait, so the press is still there on the
    // step its action becomes ready.
    *timer = wait + 0.02;
    true
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
            queued: Queued::default(),
            held: [false; 2],
            refused: [0.; 7],
            flask_note: "",
        }
    }
    /// How strongly `slot` is marked for a refused press, from 1 just
    /// after it to 0.
    pub fn refusal(&self, slot: Slot) -> f32 {
        (self.refused[slot as usize] / REFUSAL_SHOW).clamp(0., 1.)
    }
    /// The run mutation bought from the Keeper, if any.
    pub fn mutation_name(&self) -> Option<&'static str> {
        match self.mutation {
            1 => Some("Mending"),
            2 => Some("Swift skills"),
            _ => None,
        }
    }
    pub fn rect(&self) -> Rect {
        Rect::new(self.pos.x - 7., self.pos.y - 28., 14., 28.)
    }
    pub fn damage(&self) -> f32 {
        self.damage_with(self.weapon)
    }
    /// Per-strike damage this build would deal with `weapon` at its tier.
    pub fn damage_with(&self, weapon: Weapon) -> f32 {
        weapon.damage() * (1. + self.power[0] as f32 * 0.17 + self.tier as f32 * 0.09)
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
    /// The guardian that fired a hostile bolt, named if it ends the run.
    pub from: Option<EnemyKind>,
}
#[derive(Clone)]
pub struct Trap {
    pub pos: Vec2,
    pub life: f32,
    pub tick: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// A guardian or the Regent falls.
    Kill,
    /// A glassbolt leaves the bow.
    Bolt,
    /// A fire vessel or arc snare is thrown.
    Throw,
    /// Embers banked at a bellgate, or a Keeper purchase.
    Bank,
    /// A menu choice: a memory, a reliquary, a Keeper route, an option.
    Select,
    /// Refused: too few embers or copper, or a sealed door.
    Deny,
    /// The heavy third strike of a combo.
    Finisher,
    /// A guardian on screen starts winding up an attack.
    Tell,
}
impl Sfx {
    /// Every cue, in the order `Audio` loads their files.
    pub const ALL: [Self; 17] = [
        Self::Slash,
        Self::Hit,
        Self::Jump,
        Self::Dodge,
        Self::Parry,
        Self::Loot,
        Self::Hurt,
        Self::Explosion,
        Self::Heal,
        Self::Kill,
        Self::Bolt,
        Self::Throw,
        Self::Bank,
        Self::Select,
        Self::Deny,
        Self::Finisher,
        Self::Tell,
    ];
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
    /// Controls page: selected row (the last resets to defaults), whether the
    /// next key press rebinds it, and a note about the last change.
    pub controls_row: usize,
    pub rebinding: bool,
    pub controls_note: Option<String>,
    /// What the camera has shown of this level, for the atlas's fog of war.
    pub survey: Survey,
    /// The weapon a reliquary offers while its choice is open.
    pub offer: Option<Weapon>,
    /// The first abandon request from the pause screen only asks for confirmation.
    pub abandon_armed: bool,
    /// Desktop quitting from the title or pause screen: the first request
    /// asks for confirmation, and the second sets `quit`.
    pub quit_armed: bool,
    pub quit: bool,
    /// Contextual tips run only in normal play, never in practice or captures.
    pub teach: bool,
    /// The tip on screen and its remaining seconds.
    pub hint: Option<(Hint, f32)>,
    pending_hints: u32,
    pub intro: f32,
    pub route: usize,
    pub save_error: Option<String>,
    /// Saved files found damaged at launch, kept aside and named on the title.
    pub unreadable: Vec<Unreadable>,
    /// A saved run the title screen offers to continue.
    pub resume: Option<Checkpoint>,
    pub practice: bool,
    /// Prompts name controller buttons while a controller was used last.
    pub pad_prompts: bool,
    /// Where the mouse points, in interface coordinates, while it was the
    /// last thing used. Menus highlight the target under it.
    pub pointer: Option<Vec2>,
    /// The run paused because a controller was removed; shown until a
    /// controller returns or play resumes.
    pub pad_lost: bool,
    last_safe_pos: Vec2,
    /// The last thing that wounded the hero, named if the run ends.
    pub cause: Cause,
    /// The ended run, for the death and victory screens.
    pub recap: Option<Recap>,
    /// Seconds the death or victory screen has been open.
    pub result_time: f32,
    /// Seconds until another windup tell may sound.
    tell_cooldown: f32,
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
        let level = Level::generate(seed, Biome::Aqueduct, Threat::new(0, save.wins));
        let mut player = Player::new(&save);
        player.pos = level.spawn;
        let camera_y = (level.spawn.y - 248.).clamp(level.min_y, level.max_y - 360.);
        let last_safe_pos = level.spawn;
        let mut survey = Survey::new(&level);
        survey.reveal(Rect::new(0., camera_y, 640., 360.));
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
            controls_row: 0,
            rebinding: false,
            controls_note: None,
            survey,
            offer: None,
            abandon_armed: false,
            quit_armed: false,
            quit: false,
            pad_prompts: false,
            pointer: None,
            pad_lost: false,
            teach: false,
            hint: None,
            pending_hints: 0,
            intro: 4.,
            route: 0,
            save_error: None,
            unreadable: vec![],
            resume: None,
            practice: false,
            last_safe_pos,
            cause: Cause::Hazard,
            recap: None,
            result_time: 0.,
            tell_cooldown: 0.,
        }
    }
    /// Normal play: progress, settings, and any run to continue, from
    /// storage. A damaged progress or settings file is kept aside first.
    pub fn load(seed: u64) -> Self {
        let (save, damaged_save) = Save::load();
        let (settings, damaged_settings) = Settings::load();
        let mut g = Self::new(seed, save);
        g.settings = settings;
        g.teach = true;
        g.resume = Checkpoint::load();
        g.unreadable = damaged_save.into_iter().chain(damaged_settings).collect();
        g
    }
    pub fn persist(&mut self) {
        if !self.practice {
            if let Err(e) = self.save.store() {
                self.save_error = Some(e.to_string());
            }
        }
    }
    pub fn start(&mut self) {
        // A new descent replaces any saved run.
        self.clear_checkpoint();
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
    /// Moves the player to a standing position and snaps the camera there.
    pub fn place_player(&mut self, pos: Vec2) {
        self.player.pos = pos;
        self.player.vel = Vec2::ZERO;
        self.player.ground = true;
        self.player.face = 1.;
        self.last_safe_pos = pos;
        self.camera = (pos.x - 250. + 35.).clamp(0., (self.level.width - 640.).max(0.));
        self.camera_y = (pos.y - 248.).clamp(self.level.min_y, self.level.max_y - 360.);
        self.survey
            .reveal(Rect::new(self.camera, self.camera_y, 640., 360.));
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
    pub fn hurt(&mut self, damage: f32, dir: f32, cause: Cause) {
        let p = &mut self.player;
        if p.invuln > 0. || p.dodge > 0. || p.hp <= 0. {
            return;
        }
        p.hp = (p.hp - damage).max(0.);
        self.cause = cause;
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
        self.clear_checkpoint();
        self.screen = Screen::Dead;
        self.result_time = 0.;
        self.recap = Some(self.record_run(Some(self.cause), self.player.embers));
        self.player.embers = 0;
        self.persist();
    }
    /// Compares the run that just ended with the saved records, updates
    /// them, and returns the recap the result screen shows. A record counts
    /// as new only when it beats one already saved, and practice runs never
    /// change records.
    fn record_run(&mut self, cause: Option<Cause>, embers: u32) -> Recap {
        let mut recap = Recap {
            cause,
            biome: self.level.biome,
            stage: self.stage,
            embers,
            new_kills: false,
            new_stage: false,
            new_time: false,
        };
        if self.practice {
            return recap;
        }
        let s = &mut self.save;
        recap.new_kills = s.best_kills > 0 && self.player.kills > s.best_kills;
        s.best_kills = s.best_kills.max(self.player.kills);
        recap.new_stage = s.best_stage.is_some_and(|best| self.stage > best);
        s.best_stage = Some(s.best_stage.map_or(self.stage, |best| best.max(self.stage)));
        if cause.is_none() {
            recap.new_time = s.best_time.is_some_and(|best| self.run_time < best);
            s.best_time = Some(
                s.best_time
                    .map_or(self.run_time, |best| best.min(self.run_time)),
            );
        }
        recap
    }
    /// How fast the simulation runs this frame: the game-speed option
    /// applies in play only, so menus and result screens keep normal pace.
    pub fn sim_speed(&self) -> f32 {
        if self.screen == Screen::Playing {
            self.settings.speed_scale()
        } else {
            1.
        }
    }
    /// The result screens ignore confirming for their first second, so a
    /// button still held or mashed as the run ends doesn't skip the recap.
    pub fn result_ready(&self) -> bool {
        self.result_time >= RESULT_DELAY
    }
    /// Pause-screen abandon: the first request arms, the second ends the run
    /// with the same losses as a death.
    /// The window or browser tab lost focus: pause a run in progress so
    /// guardians don't keep attacking an unattended hero.
    pub fn focus_lost(&mut self) {
        if self.screen == Screen::Playing {
            self.screen = Screen::Paused;
            self.abandon_armed = false;
        }
    }
    /// A controller was removed: pause a run in progress, like losing
    /// focus, and say why on the pause screen.
    pub fn controller_lost(&mut self) {
        if self.screen == Screen::Playing {
            self.screen = Screen::Paused;
            self.abandon_armed = false;
            self.pad_lost = true;
        }
        // The pad that was prompting is gone, so name keys instead.
        self.pad_prompts = false;
    }
    pub fn prompts(&self) -> Prompts<'_> {
        Prompts::new(&self.settings.keys, self.pad_prompts)
    }
    /// Asks to close the desktop game. Quitting mid-run is the same as
    /// closing the window: the run's last checkpoint stays.
    pub fn request_quit(&mut self) {
        if !matches!(self.screen, Screen::Title | Screen::Paused) {
            return;
        }
        self.abandon_armed = false;
        if self.quit_armed {
            self.quit = true;
        } else {
            self.quit_armed = true;
        }
    }
    /// Whether quitting now leaves a run to continue: checkpoints are saved
    /// on arriving in the second and third biomes and at the Keeper, never
    /// in the opening biome or in practice.
    pub fn run_is_saved(&self) -> bool {
        !self.practice && self.stage > 0
    }
    pub fn request_abandon(&mut self) {
        if self.screen != Screen::Paused {
            return;
        }
        self.quit_armed = false;
        if self.abandon_armed {
            self.abandon_armed = false;
            self.player.hp = 0.;
            self.cause = Cause::Abandoned;
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
    pub fn open_controls(&mut self) {
        if self.screen == Screen::Options {
            self.screen = Screen::Controls;
            self.controls_row = 0;
            self.rebinding = false;
            self.controls_note = None;
        }
    }
    pub fn close_controls(&mut self) {
        if self.screen == Screen::Controls {
            self.screen = Screen::Options;
            self.rebinding = false;
            self.persist_settings();
        }
    }
    /// Applies a key pressed while waiting to rebind the selected action.
    pub fn rebind(&mut self, key: KeyCode) {
        let Some(&action) = Action::ALL.get(self.controls_row) else {
            return;
        };
        self.controls_note = Some(match self.settings.keys.bind(action, key) {
            Err(()) => {
                self.sounds.push(Sfx::Deny);
                "That key is reserved for menus. Try another.".into()
            }
            Ok(moved) => {
                self.rebinding = false;
                self.sounds.push(Sfx::Select);
                match moved {
                    Some(other) => format!(
                        "{} is now {}; {} moved to {}.",
                        action.label(),
                        self.settings.keys.label(action),
                        other.label(),
                        self.settings.keys.label(other)
                    ),
                    None => format!(
                        "{} is now {}.",
                        action.label(),
                        self.settings.keys.label(action)
                    ),
                }
            }
        });
    }
    pub fn reset_controls(&mut self) {
        self.settings.keys = Default::default();
        self.rebinding = false;
        self.controls_note = Some("Default keys restored.".into());
    }
    pub fn persist_settings(&mut self) {
        if !self.practice {
            if let Err(e) = self.settings.store() {
                self.save_error = Some(e.to_string());
            }
        }
    }
    /// Guardians (wardens, archers, brutes) have poise: light hits (ordinary
    /// strikes and glassbolts) wound them without a flinch or shove, so a held
    /// attack can no longer keep one permanently staggered and out of reach.
    /// Heavy hits (combo finishers, slams, fire, snares, parries, reflected
    /// bolts) stagger and push anything. Moths and the Regent flinch as before.
    pub fn hit(&mut self, index: usize, damage: f32, dir: f32, burn: bool, heavy: bool) {
        let e = &mut self.level.enemies[index];
        if e.hp <= 0. {
            return;
        }
        let guardian = matches!(
            e.kind,
            EnemyKind::Warden | EnemyKind::Archer | EnemyKind::Brute
        );
        let staggers = heavy || !guardian;
        // A steel number warns that a telegraphed strike is still coming.
        let committed = !staggers && e.windup > 0.;
        e.hp -= damage;
        e.flash = 0.12;
        if burn {
            e.burn = 2.;
        }
        if staggers {
            e.stun = if e.kind == EnemyKind::Regent {
                0.06
            } else {
                0.22
            };
            e.pos.x = (e.pos.x + dir * 8.).clamp(15., self.level.width - 15.);
        }
        keep_enemy_on_tier(&self.level.platforms, e);
        let pos = e.pos - vec2(0., 20.);
        let dead = e.hp <= 0.;
        let boss = e.kind == EnemyKind::Regent;
        self.label(
            pos,
            format!("{}", damage as u32),
            Color::from_hex(if committed { 0xa9c1d6 } else { 0xffdb9b }),
        );
        self.effect(if dead { Effect::Death } else { Effect::Hit }, pos, dir);
        self.sounds.push(if dead { Sfx::Kill } else { Sfx::Hit });
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
        if self.screen.freezes_world() {
            // Menus discard pending presses, early ones included.
            self.player.queued = Queued::default();
            return;
        }
        if self.screen == Screen::Playing {
            self.pad_lost = false;
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
        if matches!(self.screen, Screen::Dead | Screen::Victory) {
            self.result_time += dt;
        }
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
        // Presses made a moment before their action is ready are kept until
        // it is. Strike and glassbolt repeat while held, so only a fresh
        // press of either is kept, and releasing never adds a swing.
        // Presses too early to keep mark their slot on the HUD instead.
        p.queued.tick(dt);
        for t in &mut p.refused {
            *t = (*t - dt).max(0.);
        }
        let rate = if p.mutation == 2 { 1.35 } else { 1. };
        let fresh = [input.attack && !p.held[0], input.bow && !p.held[1]];
        p.held = [input.attack, input.bow];
        let q = &mut p.queued;
        let kept = [
            (
                Slot::Dodge,
                queue(&mut q.dodge, input.dodge, p.dodge_cd.max(p.heal_time)),
            ),
            (Slot::Parry, queue(&mut q.parry, input.parry, p.parry_cd)),
            (
                Slot::Strike,
                queue(
                    &mut q.strike,
                    fresh[0],
                    p.attack_cd.max(p.dodge).max(p.heal_time),
                ),
            ),
            (
                Slot::Glassbolt,
                queue(&mut q.bolt, fresh[1], p.bow_cd.max(p.heal_time)),
            ),
            (
                Slot::FireVessel,
                queue(&mut q.grenade, input.grenade, p.grenade_cd / rate),
            ),
            (
                Slot::ArcSnare,
                queue(&mut q.trap, input.trap, p.trap_cd / rate),
            ),
        ];
        for (slot, kept) in kept {
            if !kept {
                p.refused[slot as usize] = REFUSAL_SHOW;
            }
        }
        if input.heal && p.heal_time <= 0. && (p.flasks == 0 || p.hp >= p.max_hp) {
            p.refused[Slot::Flask as usize] = REFUSAL_SHOW;
            p.flask_note = if p.flasks == 0 { "EMPTY" } else { "FULL" };
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
        if (input.dodge || p.queued.dodge > 0.) && p.dodge_cd <= 0. && p.heal_time <= 0. {
            p.queued.dodge = 0.;
            p.dodge = 0.23;
            p.dodge_cd = 0.65;
            p.invuln = 0.23;
            p.slam = false;
            self.sounds.push(Sfx::Dodge);
            effects.push((Effect::Dodge, p.pos, -p.face));
        }
        if (input.parry || p.queued.parry > 0.) && p.parry_cd <= 0. {
            p.queued.parry = 0.;
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
        if (input.attack || p.queued.strike > 0.)
            && p.attack_cd <= 0.
            && p.dodge <= 0.
            && p.heal_time <= 0.
        {
            p.queued.strike = 0.;
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
                p.combo == 2,
            ));
            self.sounds.push(if p.combo == 2 {
                Sfx::Finisher
            } else {
                Sfx::Slash
            });
        }
        if (input.bow || p.queued.bolt > 0.) && p.bow_cd <= 0. && p.heal_time <= 0. {
            p.queued.bolt = 0.;
            p.bow_cd = 0.32;
            effects.push((Effect::Bolt, p.pos + vec2(p.face * 12., -16.), p.face));
            self.shots.push(Shot {
                pos: p.pos - vec2(-p.face * 12., 16.),
                vel: vec2(p.face * 430., 0.),
                life: 1.6,
                damage: 22. + p.power[1] as f32 * 6.,
                hostile: false,
                kind: 0,
                from: None,
            });
            self.sounds.push(Sfx::Bolt);
        }
        if (input.grenade || p.queued.grenade > 0.) && p.grenade_cd <= 0. {
            p.queued.grenade = 0.;
            p.grenade_cd = 5.;
            self.sounds.push(Sfx::Throw);
            self.shots.push(Shot {
                pos: p.pos - vec2(0., 22.),
                vel: vec2(p.face * 180., -210.),
                life: 0.85,
                damage: 90. + p.power[1] as f32 * 12.,
                hostile: false,
                kind: 1,
                from: None,
            });
        }
        if (input.trap || p.queued.trap > 0.) && p.trap_cd <= 0. {
            if let Some(y) = surface_below(&self.level.platforms, p.pos.x, p.pos.y) {
                p.queued.trap = 0.;
                p.trap_cd = 8.;
                self.sounds.push(Sfx::Throw);
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
        if let Some((pos, dir, reach, dmg, finisher)) = melee {
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
                self.hit(i, dmg, dir, self.player.tier > 1, finisher);
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
                self.hit(i, 55., self.player.face, false, true);
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
            self.hit(i, 18., 0., true, true);
        }
        if self
            .level
            .hazards
            .iter()
            .any(|h| h.overlaps(&self.player.rect()))
        {
            self.hurt(12., -self.player.face, Cause::Hazard);
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
            self.player.queued = Queued::default();
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
        self.survey
            .reveal(Rect::new(self.camera, self.camera_y, 640., 360.));
    }
    fn update_enemies(&mut self, dt: f32) {
        self.tell_cooldown = (self.tell_cooldown - dt).max(0.);
        let pp = self.player.pos;
        let mut attacks = vec![];
        let mut shots = vec![];
        let mut parries = vec![];
        let mut burned_out = vec![];
        let mut tells = vec![];
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
            e.timer -= dt;
            let dx = pp.x - e.pos.x;
            let dy = pp.y - e.pos.y;
            e.face = dx.signum();
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
                            damage: EnemyKind::Archer.hit_damage() * e.power,
                            hostile: true,
                            kind: 2,
                            from: Some(EnemyKind::Archer),
                        }),
                        EnemyKind::Regent => {
                            e.phase += 1;
                            if e.phase % 3 == 0 {
                                for n in -2..=2 {
                                    shots.push(Shot {
                                        pos: e.pos - vec2(0., 34.),
                                        vel: vec2(e.face * 170., n as f32 * 48.),
                                        life: 3.,
                                        damage: 19. * e.power,
                                        hostile: true,
                                        kind: 2,
                                        from: Some(EnemyKind::Regent),
                                    });
                                }
                            } else {
                                let reach = e.kind.strike_reach();
                                if dx.abs() < reach.x && dy.abs() < reach.y {
                                    attacks.push((i, e.kind.hit_damage() * e.power, e.face));
                                }
                                e.pos.x += e.face * 55.;
                            }
                        }
                        _ => {
                            let reach = e.kind.strike_reach();
                            if dx.abs() < reach.x && dy.abs() < reach.y {
                                attacks.push((i, e.kind.hit_damage() * e.power, e.face));
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
                e.windup = e.kind.windup();
                tells.push(e.pos);
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
            self.hit(i, self.level.enemies[i].hp, 0., false, true);
        }
        // A tell for windups the player can see, at most every
        // `TELL_GAP` seconds so a crowd doesn't become a drone.
        let view = Rect::new(self.camera, self.camera_y, 640., 360.);
        if self.tell_cooldown <= 0. && tells.iter().any(|p| view.contains(*p)) {
            self.sounds.push(Sfx::Tell);
            self.tell_cooldown = TELL_GAP;
        }
        self.shots.extend(shots);
        for (i, damage, dir) in attacks {
            if self.player.parry > 0. && self.player.face == -dir {
                parries.push(i);
            } else {
                self.hurt(damage, dir, Cause::Strike(self.level.enemies[i].kind));
            }
        }
        for i in parries {
            self.hit(i, 38., self.player.face, false, true);
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
                        damage.push((s.damage, s.vel.x.signum(), s.from));
                    }
                }
            } else {
                for (i, e) in self.level.enemies.iter().enumerate() {
                    if e.hp > 0. && e.rect().contains(s.pos) {
                        // Glassbolts are light; reflected enemy bolts land heavy.
                        hits.push((i, s.damage, s.vel.x.signum(), s.kind != 0));
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
        for (i, d, dir, heavy) in hits {
            self.hit(i, d, dir, false, heavy);
        }
        for (d, dir, from) in damage {
            self.hurt(d, dir, from.map_or(Cause::Hazard, Cause::Bolt));
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
                self.hit(i, d, 0., true, true);
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
    /// The prompt for interacting with object `i`, and whether interacting
    /// will work now. Objects that need something say what, and how much of
    /// it the hero has; the bellgate says what it banks.
    pub fn object_prompt(&self, i: usize) -> (String, bool) {
        let p = &self.player;
        let banks = |action: &str| match p.embers {
            0 => action.to_string(),
            n => format!("{action} / BANK {n} EMBERS"),
        };
        match self.level.objects[i].kind {
            ObjectKind::Exit if self.regent_holds_gate() => {
                ("HELD SHUT BY THE BRASS REGENT".into(), false)
            }
            ObjectKind::Exit => (banks("RING THE BELLGATE"), true),
            ObjectKind::Scroll => ("CLAIM A MEMORY".into(), true),
            ObjectKind::Chest => ("OPEN RELIQUARY".into(), true),
            ObjectKind::Fountain => ("DRINK FROM THE WELL".into(), true),
            ObjectKind::Forge if p.gold < FORGE_COST => (
                format!("TEMPER WEAPON / {} OF {FORGE_COST} COPPER", p.gold),
                false,
            ),
            ObjectKind::Forge => (format!("TEMPER WEAPON / {FORGE_COST} COPPER"), true),
            ObjectKind::Lore => ("READ THE INSCRIPTION".into(), true),
            ObjectKind::Secret if p.kills >= CACHE_KILLS => ("BREAK THE SEAL".into(), true),
            ObjectKind::Secret if self.save.rune => {
                ("BREAK THE SEAL WITH THE CROWN RUNE".into(), true)
            }
            ObjectKind::Secret => (
                format!("SEALED / {} OF {CACHE_KILLS} GUARDIANS FELLED", p.kills),
                false,
            ),
        }
    }
    /// Whether this is the Crown's gate with the Regent still standing.
    fn regent_holds_gate(&self) -> bool {
        self.level.biome == Biome::Crown
            && self
                .level
                .enemies
                .iter()
                .any(|e| e.kind == EnemyKind::Regent && e.hp > 0.)
    }
    fn interact(&mut self) {
        let Some(i) = self.nearby() else { return };
        let kind = self.level.objects[i].kind;
        let pos = self.level.objects[i].pos;
        match kind {
            ObjectKind::Chest => {
                let found = match self.rng.range(0, 3) {
                    0 => Weapon::Sabre,
                    1 => Weapon::Glaive,
                    _ => Weapon::Hammer,
                };
                // The tier rises either way; a different weapon is a choice.
                self.player.tier += 1;
                if found == self.player.weapon {
                    self.notify_weapon();
                } else {
                    self.offer = Some(found);
                    self.screen = Screen::Reliquary;
                }
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
                if self.player.gold < FORGE_COST {
                    self.notify("The smith asks for 60 copper to temper your weapon.");
                    self.sounds.push(Sfx::Deny);
                    return;
                }
                self.player.gold -= FORGE_COST;
                self.player.tier += 1;
                self.notify("Weapon tempered. Damage increased; attacks ignite enemies.");
            }
            ObjectKind::Lore => {
                self.notify(
                    "\"We built the sun a cage. Then wondered why it burned.\" - The Keeper",
                );
            }
            ObjectKind::Secret => {
                if !self.save.rune && self.player.kills < CACHE_KILLS {
                    self.notify(
                        "A sealed cache. Defeat 8 guardians, or return with the Crown Rune.",
                    );
                    self.sounds.push(Sfx::Deny);
                    return;
                }
                self.player.embers += 15;
                self.player.gold += 100;
                self.notify("Hidden cache opened: 15 embers and 100 copper.");
            }
            ObjectKind::Exit => {
                if self.level.biome == Biome::Crown {
                    if self.regent_holds_gate() {
                        self.notify("The Brass Regent holds the gate shut.");
                        self.sounds.push(Sfx::Deny);
                        return;
                    }
                    self.save.wins += 1;
                    self.save.embers += self.player.embers;
                    self.recap = Some(self.record_run(None, self.player.embers));
                    self.player.embers = 0;
                    self.persist();
                    self.clear_checkpoint();
                    self.screen = Screen::Victory;
                    self.result_time = 0.;
                } else {
                    self.save.embers += self.player.embers;
                    self.player.embers = 0;
                    self.persist();
                    self.screen = Screen::Camp;
                    self.save_checkpoint();
                }
            }
        }
        self.level.objects[i].used = true;
        self.sounds.push(if kind == ObjectKind::Exit {
            Sfx::Bank
        } else {
            Sfx::Loot
        });
        self.effect(Effect::Loot, pos - vec2(0., 15.), 0.);
    }
    fn notify_weapon(&mut self) {
        self.notify(&format!(
            "{} +{}  /  Scorching edge",
            self.player.weapon.name(),
            self.player.tier
        ));
    }
    /// Resolves an open reliquary: take the offered weapon or keep the
    /// current one, both at the raised tier.
    pub fn choose_weapon(&mut self, take: bool) {
        let Some(found) = self.offer.take() else {
            return;
        };
        if take {
            self.player.weapon = found;
        }
        self.sounds.push(Sfx::Select);
        self.screen = Screen::Playing;
        self.notify_weapon();
    }
    pub fn upgrade(&mut self, choice: usize) {
        self.player.power[choice] += 1;
        let hp = if choice == 2 { 24. } else { 10. };
        self.player.max_hp += hp;
        self.player.hp += hp;
        self.sounds.push(Sfx::Select);
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
            self.sounds.push(Sfx::Deny);
            return;
        }
        if choice == 1 && self.save.flask >= 3 {
            self.notify("The flask is fully reinforced.");
            self.sounds.push(Sfx::Deny);
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
        if self.screen == Screen::Camp {
            // Keep the saved run in step with what was just bought.
            self.save_checkpoint();
        }
        self.sounds.push(Sfx::Bank);
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
        self.enter(biome);
        self.save_checkpoint();
    }
    /// The current run as a checkpoint.
    pub fn checkpoint(&self) -> Checkpoint {
        let p = &self.player;
        Checkpoint {
            seed: self.seed,
            stage: self.stage,
            biome: self.level.biome,
            at_keeper: self.screen == Screen::Camp,
            weapon: p.weapon,
            tier: p.tier,
            power: p.power,
            gold: p.gold,
            kills: p.kills,
            mutation: p.mutation,
            max_hp: p.max_hp,
            run_time: self.run_time,
        }
    }
    fn save_checkpoint(&mut self) {
        if !self.practice {
            if let Err(e) = self.checkpoint().store() {
                self.save_error = Some(e.to_string());
            }
        }
    }
    fn clear_checkpoint(&mut self) {
        self.resume = None;
        if !self.practice {
            if let Err(e) = Checkpoint::clear() {
                self.save_error = Some(e.to_string());
            }
        }
    }
    /// Restores the saved run at the start of its biome, or at the Keeper.
    /// Embers carried past the last bellgate were never banked, so they're
    /// gone, as on death. Continuing doesn't count as a new descent.
    pub fn continue_run(&mut self) {
        let Some(c) = self.resume.take() else {
            return;
        };
        let (save, settings) = (self.save.clone(), self.settings.clone());
        let (practice, teach) = (self.practice, self.teach);
        *self = Self::new(c.seed, save);
        self.settings = settings;
        self.practice = practice;
        self.teach = teach;
        self.stage = c.stage;
        self.enter(c.biome);
        let p = &mut self.player;
        p.weapon = c.weapon;
        p.tier = c.tier;
        p.power = c.power;
        p.gold = c.gold;
        p.kills = c.kills;
        p.mutation = c.mutation;
        p.max_hp = c.max_hp;
        p.hp = c.max_hp;
        self.run_time = c.run_time;
        if c.at_keeper {
            self.place_player(
                self.level
                    .objects
                    .iter()
                    .find(|o| o.kind == ObjectKind::Exit)
                    .map_or(self.level.spawn, |o| o.pos),
            );
            self.screen = Screen::Camp;
            self.intro = 0.;
        } else {
            self.notify("The descent continues. Embers carried since the last bellgate are lost.");
        }
    }
    /// Generates this stage's level in `biome` and arrives at its start.
    fn enter(&mut self, biome: Biome) {
        self.level = Level::generate(
            self.seed + self.stage as u64 * 53,
            biome,
            Threat::new(self.stage, self.save.wins),
        );
        self.player.pos = self.level.spawn;
        self.player.vel = Vec2::ZERO;
        self.player.ground = true;
        self.player.jumps = 0;
        self.player.buffer = 0.;
        self.player.queued = Queued::default();
        self.player.refused = [0.; 7];
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
        self.survey = Survey::new(&self.level);
        self.survey.reveal(Rect::new(0., self.camera_y, 640., 360.));
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
    /// A guardian beside the hero, ready to start winding up.
    fn guardian(g: &mut Game, kind: EnemyKind, dx: f32) -> usize {
        let mut e = Enemy::new(g.player.pos.x + dx, g.player.pos.y, kind, Threat::BASE);
        e.timer = 0.;
        g.level.enemies.push(e);
        g.level.enemies.len() - 1
    }
    fn tells(g: &Game) -> usize {
        g.sounds.iter().filter(|s| **s == Sfx::Tell).count()
    }
    #[test]
    fn windups_sound_a_tell_once_and_only_on_screen() {
        let mut g = game();
        g.place_player(vec2(900., FLOOR));
        guardian(&mut g, EnemyKind::Warden, 25.);
        g.tick(STEP, Input::default());
        assert!(g.level.enemies[0].windup > 0.);
        assert_eq!(tells(&g), 1);
        for _ in 0..60 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(tells(&g), 1, "one tell per windup");

        // Two guardians starting together, or within the gap, make one tell;
        // a later windup makes another.
        let mut g = game();
        g.place_player(vec2(900., FLOOR));
        guardian(&mut g, EnemyKind::Warden, 25.);
        guardian(&mut g, EnemyKind::Warden, -25.);
        let late = guardian(&mut g, EnemyKind::Brute, 40.);
        g.level.enemies[late].timer = TELL_GAP / 2.;
        for _ in 0..30 {
            g.tick(STEP, Input::default());
        }
        assert!(g
            .level
            .enemies
            .iter()
            .all(|e| e.windup > 0. || e.timer > 0.));
        assert_eq!(tells(&g), 1, "windups within the gap share a tell");
        let mut g = game();
        g.place_player(vec2(900., FLOOR));
        guardian(&mut g, EnemyKind::Warden, 25.);
        let late = guardian(&mut g, EnemyKind::Brute, 40.);
        g.level.enemies[late].timer = TELL_GAP + 0.05;
        for _ in 0..60 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(tells(&g), 2);

        // An archer drawing on the hero from beyond the left edge is silent.
        let mut g = game();
        g.place_player(vec2(900., FLOOR));
        let archer = guardian(&mut g, EnemyKind::Archer, -290.);
        g.tick(STEP, Input::default());
        assert!(g.level.enemies[archer].windup > 0.);
        assert!(g.level.enemies[archer].pos.x < g.camera);
        assert_eq!(tells(&g), 0);
    }
    #[test]
    fn strikes_land_only_inside_the_warned_reach() {
        for kind in [EnemyKind::Warden, EnemyKind::Brute, EnemyKind::Regent] {
            let reach = kind.strike_reach();
            for (dx, dy, lands) in [
                (reach.x - 2., 0., true),
                (reach.x + 2., 0., false),
                (-(reach.x - 2.), 0., true),
                (0., reach.y - 2., true),
                (0., reach.y + 2., false),
            ] {
                let mut g = game();
                g.place_player(vec2(900., FLOOR));
                let i = guardian(&mut g, kind, dx);
                // Guardians keep to their ledge, so the hero rises instead.
                g.player.pos.y -= dy;
                g.player.ground = false;
                let e = &mut g.level.enemies[i];
                e.windup = STEP / 2.;
                // The Regent's every third attack is a volley instead.
                e.phase = 0;
                let hp = g.player.hp;
                g.tick(STEP, Input::default());
                assert_eq!(g.player.hp < hp, lands, "{kind:?} at {dx}, {dy}");
            }
        }
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
            from: None,
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
            from: None,
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
        let mut e = Enemy::new(500., FLOOR, EnemyKind::Warden, Threat::BASE);
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
        g.hurt(10., 1., Cause::Hazard);
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
            from: None,
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
            from: None,
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
        g.hurt(50., 1., Cause::Hazard);
        assert_eq!(g.player.hp, 100.);
        for _ in 0..36 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.player.dodge, 0.);
        assert!(g.player.dodge_cd > 0.);
        g.hurt(20., 1., Cause::Hazard);
        assert_eq!(g.player.hp, 80.);
    }
    #[test]
    fn melee_only_hits_facing_range() {
        let mut g = game();
        g.level.enemies = vec![
            Enemy::new(140., FLOOR, EnemyKind::Warden, Threat::BASE),
            Enemy::new(60., FLOOR, EnemyKind::Warden, Threat::BASE),
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
            g.level.enemies = vec![Enemy::new(140., FLOOR, EnemyKind::Warden, Threat::BASE)];
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
            Threat::BASE,
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
            Threat::BASE,
        )];
        g.player.hp = g.player.max_hp * 0.5;
        idle(&mut g, 1.);
        assert!(g.hint.is_none());
    }
    #[test]
    fn kills_and_hostile_bolts_queue_their_tips() {
        let mut g = tutor();
        g.level.enemies = vec![Enemy::new(400., FLOOR, EnemyKind::Moth, Threat::BASE)];
        g.hit(0, 1000., 1., false, true);
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
            from: None,
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
                Threat::BASE,
            )];
            idle(&mut g, 0.5);
            assert!(g.hint.is_none());
            assert_eq!(g.settings.hints_seen, 0);
        }
    }
    /// Expected build entering a stage of a run: per biome, two reliquaries and
    /// a forge (+3 tiers) and two memories (one Ferocity, one Resolve).
    fn stage_build(stage: u32, weapon: Weapon) -> Player {
        let mut p = Player::new(&Save::default());
        p.weapon = weapon;
        p.tier = 1 + 3 * stage;
        p.power = [1 + stage, 1, 1 + stage];
        p.max_hp = 100. + 34. * stage as f32;
        p.hp = p.max_hp;
        p
    }
    /// A real simulated duel: the build holds attack against one enemy. Returns
    /// seconds until the enemy falls and the fraction of vitality lost.
    fn duel(stage: u32, threat: Threat, weapon: Weapon, kind: EnemyKind) -> (f32, f32) {
        duel_with(stage, threat, weapon, kind, false)
    }
    /// With `dodge_windups`, the build also dodges as each telegraphed strike
    /// is about to land, then turns back to face the enemy.
    fn duel_with(
        stage: u32,
        threat: Threat,
        weapon: Weapon,
        kind: EnemyKind,
        dodge_windups: bool,
    ) -> (f32, f32) {
        let mut g = game();
        let spawn = g.player.pos;
        g.player = stage_build(stage, weapon);
        g.player.pos = spawn;
        g.level.enemies = vec![Enemy::new(spawn.x + 34., FLOOR, kind, threat)];
        // An approaching guardian's attack timer has usually already run down.
        g.level.enemies[0].timer = 0.;
        let mut t = 0.;
        while t < 30. && g.level.enemies[0].hp > 0. && g.player.hp > 0. {
            let e = &g.level.enemies[0];
            let toward = (e.pos.x - g.player.pos.x).signum();
            let striking = e.windup > 0. && e.windup <= 0.2;
            g.tick(
                STEP,
                Input {
                    attack: true,
                    dodge: dodge_windups && striking,
                    axis: if dodge_windups && g.player.face != toward && g.player.dodge <= 0. {
                        toward
                    } else {
                        0.
                    },
                    ..Default::default()
                },
            );
            t += STEP;
        }
        assert!(
            g.level.enemies[0].hp <= 0. || g.player.hp <= 0.,
            "duel stalled"
        );
        (t, 1. - g.player.hp / g.player.max_hp)
    }
    /// Strikes an unkillable guardian lands in five seconds against the
    /// opening build holding attack (optionally dodging telegraphs).
    fn strikes_against_held_attack(weapon: Weapon, kind: EnemyKind, dodge: bool) -> u32 {
        let mut g = game();
        let spawn = g.player.pos;
        g.player = stage_build(0, weapon);
        g.player.pos = spawn;
        g.player.max_hp = 1e6;
        g.player.hp = 1e6;
        g.level.enemies = vec![Enemy::new(spawn.x + 34., FLOOR, kind, Threat::BASE)];
        g.level.enemies[0].timer = 0.;
        g.level.enemies[0].hp = 1e7;
        let mut strikes = 0;
        for _ in 0..(5. / STEP) as u32 {
            let e = &g.level.enemies[0];
            let toward = (e.pos.x - g.player.pos.x).signum();
            let before = g.player.hp;
            g.tick(
                STEP,
                Input {
                    attack: true,
                    dodge: dodge && e.windup > 0. && e.windup <= 0.2,
                    axis: if dodge && g.player.face != toward && g.player.dodge <= 0. {
                        toward
                    } else {
                        0.
                    },
                    ..Default::default()
                },
            );
            strikes += u32::from(g.player.hp < before);
            g.player.invuln = 0.;
        }
        strikes
    }
    /// The cooldown each early-pressable action starts when it happens, an
    /// input pressing it, and its HUD slot.
    #[allow(clippy::type_complexity)]
    fn early_actions() -> Vec<(&'static str, fn(&mut Player) -> &mut f32, Input, Slot)> {
        let mut presses = [Input::default(); 6];
        presses[0].dodge = true;
        presses[1].parry = true;
        presses[2].attack = true;
        presses[3].bow = true;
        presses[4].grenade = true;
        presses[5].trap = true;
        let actions: [(&str, fn(&mut Player) -> &mut f32, Slot); 6] = [
            ("dodge", |p| &mut p.dodge_cd, Slot::Dodge),
            ("parry", |p| &mut p.parry_cd, Slot::Parry),
            ("strike", |p| &mut p.attack_cd, Slot::Strike),
            ("glassbolt", |p| &mut p.bow_cd, Slot::Glassbolt),
            ("fire vessel", |p| &mut p.grenade_cd, Slot::FireVessel),
            ("arc snare", |p| &mut p.trap_cd, Slot::ArcSnare),
        ];
        actions
            .into_iter()
            .zip(presses)
            .map(|((name, cd, slot), press)| (name, cd, press, slot))
            .collect()
    }
    /// Presses once, `early` seconds before the action is ready, and returns
    /// the simulated times (from the press) at which it happened.
    fn early_press(cd: fn(&mut Player) -> &mut f32, press: Input, early: f32) -> Vec<f32> {
        let mut g = game();
        g.level.hazards.clear();
        g.place_player(vec2(900., FLOOR));
        *cd(&mut g.player) = early;
        let mut times = vec![];
        for step in 0..120 {
            let before = *cd(&mut g.player);
            g.tick(STEP, if step == 0 { press } else { Input::default() });
            if *cd(&mut g.player) > before + STEP {
                times.push(step as f32 * STEP);
            }
        }
        times
    }
    #[test]
    fn presses_made_a_moment_early_happen_once_ready() {
        for (name, cd, press, _) in early_actions() {
            let times = early_press(cd, press, 0.1);
            assert_eq!(times.len(), 1, "{name} pressed 0.1 s early: {times:?}");
            assert!(
                (0.09..0.11).contains(&times[0]),
                "{name} happens as it becomes ready, not later: {times:?}"
            );
            assert_eq!(early_press(cd, press, 0.).len(), 1, "{name} when ready");
            assert!(
                early_press(cd, press, 0.3).is_empty(),
                "{name} pressed 0.3 s early is still dropped"
            );
        }
    }
    /// A game with the hero on safe ground and nothing about.
    fn quiet() -> Game {
        let mut g = game();
        g.level.hazards.clear();
        g.place_player(vec2(900., FLOOR));
        g
    }
    fn marked(g: &Game) -> Vec<Slot> {
        [
            Slot::Strike,
            Slot::Glassbolt,
            Slot::FireVessel,
            Slot::ArcSnare,
            Slot::Flask,
            Slot::Parry,
            Slot::Dodge,
        ]
        .into_iter()
        .filter(|s| g.player.refusal(*s) > 0.)
        .collect()
    }
    #[test]
    fn presses_too_early_to_keep_mark_only_their_own_slot() {
        for (name, cd, press, slot) in early_actions() {
            let mut g = quiet();
            *cd(&mut g.player) = 0.3;
            g.tick(STEP, press);
            assert_eq!(marked(&g), vec![slot], "{name} pressed 0.3 s early");
            assert!(g.player.refusal(slot) > 0.95, "marked at full strength");
            for _ in 0..(REFUSAL_SHOW / STEP) as usize + 2 {
                g.tick(STEP, Input::default());
            }
            assert!(marked(&g).is_empty(), "{name}'s mark fades");
            // A press that's kept and then happens marks nothing.
            let mut g = quiet();
            *cd(&mut g.player) = 0.1;
            g.tick(STEP, press);
            assert!(marked(&g).is_empty(), "{name} pressed 0.1 s early is kept");
        }
        // Holding strike through its recovery isn't a refused press.
        let mut g = quiet();
        let hold = Input {
            attack: true,
            ..Default::default()
        };
        for _ in 0..90 {
            g.tick(STEP, hold);
        }
        assert!(marked(&g).is_empty(), "a held strike marks nothing");
    }
    #[test]
    fn the_flask_says_why_it_was_refused() {
        let heal = Input {
            heal: true,
            ..Default::default()
        };
        let mut g = quiet();
        g.tick(STEP, heal);
        assert_eq!(
            (marked(&g), g.player.flask_note),
            (vec![Slot::Flask], "FULL")
        );
        let mut g = quiet();
        g.player.hp = 40.;
        g.player.flasks = 0;
        g.tick(STEP, heal);
        assert_eq!(
            (marked(&g), g.player.flask_note),
            (vec![Slot::Flask], "EMPTY")
        );
        let mut g = quiet();
        g.player.hp = 40.;
        g.tick(STEP, heal);
        assert!(
            marked(&g).is_empty() && g.player.heal_time > 0.,
            "drinking is no refusal"
        );
    }
    #[test]
    fn held_strikes_repeat_as_before_and_releasing_adds_none() {
        let mut g = game();
        g.level.hazards.clear();
        g.place_player(vec2(900., FLOOR));
        let hold = Input {
            attack: true,
            ..Default::default()
        };
        let mut swings = vec![];
        for step in 0..240 {
            let before = g.player.attack_cd;
            // Held through five swings and released 25 steps after the last,
            // 0.07 s before the next would come, then left alone.
            let held = step < 157;
            g.tick(STEP, if held { hold } else { Input::default() });
            if g.player.attack_cd > before {
                swings.push(step);
            }
        }
        // The sabre's 0.27 s delay is 33 steps (32.4, rounded up by the
        // timer reaching zero on the next step).
        let gaps: Vec<_> = swings.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.iter().all(|&gap| gap == 33), "even swings: {gaps:?}");
        assert_eq!(swings.len(), 5, "no swing after release: {swings:?}");
    }
    #[test]
    fn a_strike_tapped_late_in_a_dodge_lands_as_it_ends() {
        let mut g = game();
        g.level.hazards.clear();
        g.place_player(vec2(900., FLOOR));
        g.tick(
            STEP,
            Input {
                dodge: true,
                ..Default::default()
            },
        );
        assert!(g.player.dodge > 0.);
        while g.player.dodge > 0.1 {
            g.tick(STEP, Input::default());
        }
        // A tap: held for two steps, then released.
        for _ in 0..2 {
            g.tick(
                STEP,
                Input {
                    attack: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(g.player.attack_cd, 0., "no swing while dodging");
        let mut steps = 0;
        while g.player.attack_cd == 0. && steps < 60 {
            g.tick(STEP, Input::default());
            steps += 1;
        }
        assert!(g.player.dodge <= 0. && g.player.attack_cd > 0.);
        assert!(
            steps <= 12,
            "swung {steps} steps after the tap, as the dodge ended"
        );
    }
    #[test]
    fn menus_discard_early_presses() {
        let mut g = game();
        g.level.hazards.clear();
        g.place_player(vec2(900., FLOOR));
        g.player.dodge_cd = 0.1;
        g.tick(
            STEP,
            Input {
                dodge: true,
                ..Default::default()
            },
        );
        g.screen = Screen::Paused;
        g.tick(STEP, Input::default());
        g.screen = Screen::Playing;
        for _ in 0..60 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(
            g.player.dodge_cd, 0.,
            "the dodge pressed before pausing is gone"
        );
    }
    #[test]
    fn held_attack_no_longer_stun_locks_guardians() {
        // Before guardians had poise, every light hit stunned and shoved them
        // out of reach: holding attack let wardens and brutes land 0 strikes
        // in five seconds (1 against the slow hammer).
        for weapon in [Weapon::Sabre, Weapon::Glaive, Weapon::Hammer] {
            for kind in [EnemyKind::Warden, EnemyKind::Brute, EnemyKind::Archer] {
                let held = strikes_against_held_attack(weapon, kind, false);
                let dodging = strikes_against_held_attack(weapon, kind, true);
                eprintln!("{weapon:?} vs {kind:?}: {held} strikes held, {dodging} dodging");
                assert!(held >= 2, "{kind:?} stun-locked by a held {weapon:?}");
                // Bolts are answered with a parry rather than a dodge.
                if kind != EnemyKind::Archer {
                    assert!(dodging < held, "dodging the telegraph must help");
                }
            }
        }
    }
    #[test]
    fn guardians_flinch_only_from_heavy_hits() {
        let mut g = game();
        g.level.enemies = vec![
            Enemy::new(300., FLOOR, EnemyKind::Warden, Threat::BASE),
            Enemy::new(500., FLOOR - 30., EnemyKind::Moth, Threat::BASE),
        ];
        g.hit(0, 5., 1., false, false);
        let warden = &g.level.enemies[0];
        assert!(warden.hp < 65. && warden.stun == 0. && warden.pos.x == 300.);
        g.hit(0, 5., 1., false, true);
        let warden = &g.level.enemies[0];
        assert!(warden.stun > 0. && warden.pos.x > 300.);
        g.hit(1, 5., 1., false, false);
        assert!(g.level.enemies[1].stun > 0., "moths still flinch");
    }
    /// Mean kill time and vitality lost across weapons and melee guardians.
    fn stage_pressure(stage: u32, threat: Threat) -> (f32, f32) {
        let mut totals = (0., 0.);
        let mut n = 0.;
        for weapon in [Weapon::Sabre, Weapon::Glaive, Weapon::Hammer] {
            for kind in [EnemyKind::Warden, EnemyKind::Brute] {
                let (time, lost) = duel(stage, threat, weapon, kind);
                totals.0 += time;
                totals.1 += lost;
                n += 1.;
            }
        }
        (totals.0 / n, totals.1 / n)
    }
    #[test]
    fn difficulty_keeps_pace_with_a_runs_growing_build() {
        // Kill time against melee guardians (holding attack stun-locks them, so
        // this measures the health race) must not fall as the build grows.
        let opening = stage_pressure(0, Threat::BASE).0;
        for stage in 1..=2 {
            let flat = stage_pressure(stage, Threat::BASE).0;
            let scaled = stage_pressure(stage, Threat::new(stage, 0)).0;
            eprintln!("stage {stage}: kill {flat:.2}s unscaled, {scaled:.2}s scaled (stage 0: {opening:.2}s)");
            assert!(
                flat < opening * 0.8,
                "without stage scaling later stages get easier"
            );
            assert!(
                scaled >= opening * 0.95,
                "stage {stage} kills faster than the opening"
            );
        }
        // Each guardian strike should cost a comparable share of vitality.
        for kind in [EnemyKind::Warden, EnemyKind::Archer, EnemyKind::Brute] {
            let share = |stage| {
                kind.hit_damage() * Threat::new(stage, 0).damage(kind)
                    / stage_build(stage, Weapon::Sabre).max_hp
            };
            for stage in 1..=2 {
                assert!(
                    share(stage) >= share(0) * 0.9,
                    "{kind:?} strikes soften at stage {stage}"
                );
            }
        }
        // Face-tanking the Regent with the expected Crown build is a real fight
        // but survivable; dodging and parrying do better.
        for weapon in [Weapon::Sabre, Weapon::Glaive, Weapon::Hammer] {
            let (flat_time, _) = duel(2, Threat::BASE, weapon, EnemyKind::Regent);
            let (time, lost) = duel(2, Threat::new(2, 0), weapon, EnemyKind::Regent);
            eprintln!(
                "Regent vs {weapon:?}: {time:.2}s, {:.0}% vitality lost (unscaled {flat_time:.2}s)",
                lost * 100.
            );
            assert!(time > flat_time * 1.3 && lost > 0.4 && lost < 0.9);
        }
    }
    #[test]
    fn threat_scales_with_stage_and_victories() {
        let warden = |t: Threat| Enemy::new(0., FLOOR, EnemyKind::Warden, t);
        assert_eq!(warden(Threat::BASE).max_hp, 65.);
        assert_eq!(warden(Threat::BASE).power, 1.);
        assert!((warden(Threat::new(2, 0)).max_hp - 65. * 1.9).abs() < 1e-3);
        assert!(
            (warden(Threat::new(9, 0)).power - 1.6).abs() < 1e-6,
            "stages cap at the Crown"
        );
        assert!((warden(Threat::new(0, 1)).max_hp - 65. * 1.12).abs() < 1e-3);
        let regent = Enemy::new(0., FLOOR, EnemyKind::Regent, Threat::new(2, 0));
        assert!((regent.max_hp - 1050. * 1.6).abs() < 1e-2);
        assert!((regent.power - 1.1).abs() < 1e-6);
        // Travelling between bellgates generates each biome at its stage.
        let mut g = game();
        g.travel();
        assert!(g.level.enemies.iter().all(|e| (e.power - 1.3).abs() < 1e-6));
        g.travel();
        assert_eq!(g.level.biome, Biome::Crown);
        let regent = g.level.enemies.iter().find(|e| e.kind == EnemyKind::Regent);
        assert!((regent.unwrap().max_hp - 1050. * 1.6).abs() < 1e-2);
        assert_eq!(
            g.level.enemies.len(),
            5,
            "Regent, two gallery guards, a moth, a warden"
        );
    }
    /// Opens the level's first reliquary holding `weapon`, with a fixed seed.
    fn open_reliquary(weapon: Weapon) -> Game {
        let mut g = game();
        g.player.weapon = weapon;
        let chest = g
            .level
            .objects
            .iter()
            .find(|o| o.kind == ObjectKind::Chest)
            .unwrap()
            .pos;
        g.player.pos = chest;
        g.tick(
            STEP,
            Input {
                interact: true,
                ..Default::default()
            },
        );
        g
    }
    #[test]
    fn reliquaries_offer_a_choice_unless_they_hold_your_weapon() {
        let weapons = [Weapon::Sabre, Weapon::Glaive, Weapon::Hammer];
        let opened: Vec<_> = weapons.iter().map(|w| open_reliquary(*w)).collect();
        // The same seed rolls the same weapon: one build already holds it.
        let direct: Vec<_> = opened.iter().filter(|g| g.offer.is_none()).collect();
        assert_eq!(direct.len(), 1);
        assert_eq!(direct[0].screen, Screen::Playing);
        assert_eq!(direct[0].player.tier, 2);
        for (held, g) in weapons.iter().zip(&opened) {
            let Some(found) = g.offer else { continue };
            assert_ne!(found, *held);
            assert_eq!(g.screen, Screen::Reliquary);
            assert_eq!(g.player.tier, 2, "the tier rises before the choice");
            for take in [true, false] {
                let mut g = open_reliquary(*held);
                let before = g.player.pos;
                g.tick(STEP, Input::default());
                assert_eq!(g.player.pos, before, "the choice holds the world still");
                g.choose_weapon(take);
                assert_eq!(g.player.weapon, if take { found } else { *held });
                assert_eq!(g.player.tier, 2);
                assert_eq!(g.screen, Screen::Playing);
                assert!(g.offer.is_none());
            }
        }
    }
    #[test]
    fn exploring_reveals_the_atlas_and_each_biome_starts_dark() {
        let mut g = game();
        let far = vec2(1500., FLOOR);
        assert!(g.survey.seen(g.player.pos));
        assert!(!g.survey.seen(far));
        g.player.pos = far;
        for _ in 0..240 {
            g.tick(STEP, Input::default());
        }
        assert!(g.survey.seen(far), "the camera followed and revealed it");
        let explored = g.survey.fraction();
        g.travel();
        assert!(g.survey.fraction() < explored);
        assert!(g.survey.seen(g.level.spawn));
    }
    #[test]
    fn quitting_asks_first_and_only_from_the_title_or_pause() {
        let mut g = game();
        g.screen = Screen::Title;
        g.request_quit();
        assert!(g.quit_armed && !g.quit);
        g.request_quit();
        assert!(g.quit);
        let mut g = game();
        g.start();
        g.request_quit();
        assert!(!g.quit_armed, "not during play");
        g.screen = Screen::Paused;
        g.request_quit();
        g.request_abandon();
        assert!(
            g.abandon_armed && !g.quit_armed,
            "the other warning replaces it"
        );
        g.request_quit();
        assert!(g.quit_armed && !g.abandon_armed);
        g.request_quit();
        assert!(g.quit);
        assert_eq!(g.screen, Screen::Paused, "the run isn't ended or saved");
        assert!(!g.run_is_saved(), "practice runs never save");
        g.practice = false;
        assert!(!g.run_is_saved(), "the opening biome has no checkpoint");
        g.travel();
        assert!(g.run_is_saved());
    }
    #[test]
    fn losing_focus_pauses_only_a_run_in_progress() {
        let mut g = game();
        g.start();
        g.focus_lost();
        assert_eq!(g.screen, Screen::Paused);
        g.focus_lost();
        assert_eq!(g.screen, Screen::Paused, "stays paused");
        for screen in [
            Screen::Title,
            Screen::Scroll,
            Screen::Camp,
            Screen::Options,
            Screen::Dead,
        ] {
            g.screen = screen;
            g.focus_lost();
            assert_eq!(g.screen, screen, "menus already hold the world still");
        }
    }
    #[test]
    fn controls_page_rebinds_swaps_and_restores() {
        let mut g = game();
        g.screen = Screen::Paused;
        g.open_controls();
        assert_eq!(
            g.screen,
            Screen::Paused,
            "controls open from the options page"
        );
        g.open_options();
        g.open_controls();
        assert_eq!(g.screen, Screen::Controls);
        g.controls_row = 2; // Jump
        g.rebinding = true;
        g.rebind(KeyCode::Escape);
        assert!(g.rebinding, "a reserved key keeps listening");
        assert!(g.controls_note.as_deref().unwrap().contains("reserved"));
        g.rebind(KeyCode::K);
        assert!(!g.rebinding);
        assert_eq!(g.settings.keys.keys(Action::Jump), &[KeyCode::K]);
        assert_eq!(g.settings.keys.keys(Action::Glassbolt), &[KeyCode::Space]);
        assert!(g
            .controls_note
            .as_deref()
            .unwrap()
            .contains("Glassbolt moved to SPACE"));
        assert!(Hint::Climb.text(&g.prompts()).starts_with("K jumps"));
        g.pad_prompts = true;
        assert!(Hint::Climb.text(&g.prompts()).starts_with("A jumps"));
        assert!(!Hint::Parry.text(&g.prompts()).contains("click"));
        g.pad_prompts = false;
        assert!(Hint::Parry.text(&g.prompts()).contains("right click"));
        g.close_controls();
        assert_eq!(g.screen, Screen::Options);
        g.close_options();
        g.start();
        assert_eq!(
            g.settings.keys.short(Action::Jump),
            "K",
            "bindings survive a new run"
        );
        g.screen = Screen::Options;
        g.open_controls();
        g.reset_controls();
        assert_eq!(g.settings.keys, crate::controls::Bindings::default());
    }
    #[test]
    fn actions_menus_and_refusals_queue_their_own_cues() {
        let heard = |g: &mut Game| std::mem::take(&mut g.sounds);
        let press = |g: &mut Game, input: Input| {
            g.tick(STEP, input);
        };
        let mut g = game();
        g.place_player(g.level.spawn);
        g.sounds.clear();
        press(
            &mut g,
            Input {
                bow: true,
                ..Default::default()
            },
        );
        assert_eq!(heard(&mut g), vec![Sfx::Bolt]);
        press(
            &mut g,
            Input {
                grenade: true,
                ..Default::default()
            },
        );
        assert_eq!(heard(&mut g), vec![Sfx::Throw]);
        press(
            &mut g,
            Input {
                trap: true,
                ..Default::default()
            },
        );
        assert_eq!(heard(&mut g), vec![Sfx::Throw]);

        // Held attack: the third swing of each combo is the heavy finisher.
        let mut swings = vec![];
        for _ in 0..240 {
            press(
                &mut g,
                Input {
                    attack: true,
                    ..Default::default()
                },
            );
            swings.extend(
                heard(&mut g)
                    .into_iter()
                    .filter(|s| matches!(s, Sfx::Slash | Sfx::Finisher)),
            );
        }
        assert_eq!(swings[..3], [Sfx::Slash, Sfx::Finisher, Sfx::Slash]);

        // A wound sounds a hit; a killing blow sounds a kill instead.
        let pos = g.player.pos;
        g.level.enemies = vec![Enemy::new(
            pos.x + 20.,
            pos.y,
            EnemyKind::Warden,
            Threat::BASE,
        )];
        g.hit(0, 5., 1., false, false);
        assert_eq!(heard(&mut g), vec![Sfx::Hit]);
        g.hit(0, 500., 1., false, false);
        assert_eq!(heard(&mut g), vec![Sfx::Kill]);

        // Refusals and banking.
        let at = |g: &mut Game, kind: ObjectKind| {
            let pos = g.level.objects.iter().find(|o| o.kind == kind).unwrap().pos;
            g.place_player(pos - vec2(20., 0.));
            g.sounds.clear();
            g.tick(
                STEP,
                Input {
                    interact: true,
                    ..Default::default()
                },
            );
        };
        g.player.gold = 0;
        at(&mut g, ObjectKind::Forge);
        assert_eq!(heard(&mut g), vec![Sfx::Deny]);
        g.player.kills = 0;
        at(&mut g, ObjectKind::Secret);
        assert_eq!(heard(&mut g), vec![Sfx::Deny]);
        at(&mut g, ObjectKind::Exit);
        assert_eq!(heard(&mut g), vec![Sfx::Bank]);
        assert_eq!(g.screen, Screen::Camp);
        g.save.embers = 0;
        g.buy(0);
        assert_eq!(heard(&mut g), vec![Sfx::Deny]);
        g.save.embers = 100;
        g.buy(0);
        assert_eq!(heard(&mut g), vec![Sfx::Bank]);

        // Menu choices.
        g.screen = Screen::Scroll;
        g.upgrade(0);
        assert_eq!(heard(&mut g), vec![Sfx::Select]);
        g.offer = Some(Weapon::Hammer);
        g.screen = Screen::Reliquary;
        g.choose_weapon(false);
        assert_eq!(heard(&mut g), vec![Sfx::Select]);
    }
    /// A normal (non-practice) run; tests store files in memory.
    fn saved_run() -> Game {
        let mut g = Game::new(42, Save::default());
        g.start();
        g.level.enemies.clear();
        g
    }
    fn use_exit(g: &mut Game) {
        let exit = g.level.objects.iter().find(|o| o.kind == ObjectKind::Exit);
        g.place_player(exit.unwrap().pos - vec2(10., 0.));
        g.tick(
            STEP,
            Input {
                interact: true,
                ..Default::default()
            },
        );
    }
    fn stored_checkpoint() -> Option<Checkpoint> {
        Checkpoint::load()
    }

    #[test]
    fn runs_continue_from_their_last_biome_or_keeper() {
        let mut g = saved_run();
        assert!(
            stored_checkpoint().is_none(),
            "nothing to continue in the first biome"
        );
        let p = &mut g.player;
        (p.weapon, p.tier, p.power, p.gold, p.kills, p.mutation) =
            (Weapon::Glaive, 3, [2, 1, 3], 77, 9, 2);
        p.max_hp = 158.;
        p.embers = 6;
        g.run_time = 201.;
        use_exit(&mut g);
        assert_eq!(g.screen, Screen::Camp);
        let at_keeper = stored_checkpoint().expect("saved at the Keeper");
        assert!(at_keeper.at_keeper);
        assert_eq!((at_keeper.stage, at_keeper.biome), (0, Biome::Aqueduct));

        // Buying from the Keeper updates the saved run.
        g.save.embers = 100;
        g.buy(0);
        assert_eq!(stored_checkpoint().unwrap().max_hp, 173.);

        g.route = 1;
        g.travel();
        let arrived = stored_checkpoint().expect("saved on arrival");
        assert!(!arrived.at_keeper);
        assert_eq!((arrived.stage, arrived.biome), (1, Biome::Foundry));
        let level = g.level.clone();
        g.player.embers = 11; // carried, not banked
        g.player.hp = 20.;
        let runs = g.save.runs;

        // A later launch offers the run and restores it.
        let mut next = Game::new(9999, g.save.clone());
        next.resume = stored_checkpoint();
        next.continue_run();
        assert_eq!(next.screen, Screen::Playing);
        assert_eq!((next.stage, next.level.biome), (1, Biome::Foundry));
        assert_eq!(next.level.platforms, level.platforms);
        assert_eq!(
            next.level
                .enemies
                .iter()
                .map(|e| (e.pos, e.kind, e.max_hp))
                .collect::<Vec<_>>(),
            level
                .enemies
                .iter()
                .map(|e| (e.pos, e.kind, e.max_hp))
                .collect::<Vec<_>>()
        );
        let p = &next.player;
        assert_eq!(
            (p.weapon, p.tier, p.power, p.gold, p.kills, p.mutation),
            (Weapon::Glaive, 3, [2, 1, 3], 77, 9, 2)
        );
        assert_eq!((p.max_hp, p.hp, p.embers), (173., 173., 0));
        assert_eq!(p.pos, next.level.spawn);
        assert!((next.run_time - 201.).abs() < 0.1, "{}", next.run_time);
        assert_eq!(next.save.runs, runs, "continuing isn't a new descent");
        assert!(next.resume.is_none());

        // Continuing at the Keeper returns to the Keeper, ready to travel.
        let mut keeper = Game::new(1, Save::default());
        keeper.resume = Some(at_keeper);
        keeper.continue_run();
        assert_eq!(keeper.screen, Screen::Camp);
        keeper.route = 0;
        keeper.travel();
        assert_eq!((keeper.stage, keeper.level.biome), (1, Biome::Garden));
    }

    #[test]
    fn ending_or_replacing_a_run_deletes_its_checkpoint() {
        let reach_keeper = || {
            let mut g = saved_run();
            use_exit(&mut g);
            assert!(stored_checkpoint().is_some());
            g
        };
        // Death.
        let mut g = reach_keeper();
        g.travel();
        g.hurt(10_000., 1., Cause::Hazard);
        assert_eq!(g.screen, Screen::Dead);
        assert!(stored_checkpoint().is_none());
        // Abandoning from the pause screen.
        let mut g = reach_keeper();
        g.travel();
        g.screen = Screen::Paused;
        g.request_abandon();
        g.request_abandon();
        assert!(stored_checkpoint().is_none());
        // Starting a new descent from the title instead of continuing.
        let mut g = reach_keeper();
        g.resume = stored_checkpoint();
        g.screen = Screen::Title;
        g.start();
        assert!(stored_checkpoint().is_none() && g.resume.is_none());
        // Victory.
        let mut g = reach_keeper();
        g.travel();
        g.screen = Screen::Camp;
        g.travel();
        assert_eq!(g.level.biome, Biome::Crown);
        g.level.enemies.clear();
        use_exit(&mut g);
        assert_eq!(g.screen, Screen::Victory);
        assert!(stored_checkpoint().is_none());
    }

    #[test]
    fn damaged_saves_are_kept_aside_before_anything_overwrites_them() {
        use crate::storage;
        let damaged = b"{\"embers\": 412, \"vitality\": 3,".to_vec();
        storage::write(Save::FILE, &damaged).unwrap();
        storage::write(Settings::FILE, b"\x00\x01 not json").unwrap();
        let mut g = Game::load(9);
        assert_eq!(g.save.embers, 0, "play carries on with defaults");
        assert_eq!(g.settings, Settings::default());
        assert_eq!(
            g.unreadable,
            vec![
                Unreadable {
                    file: Save::FILE,
                    kept: Ok("progress.unreadable.json".into())
                },
                Unreadable {
                    file: Settings::FILE,
                    kept: Ok("settings.unreadable.json".into())
                },
            ]
        );
        // Starting a run writes progress straight away; the copy survives it.
        g.start();
        let fresh: Save = serde_json::from_slice(&storage::read(Save::FILE).unwrap()).unwrap();
        assert_eq!(fresh.runs, 1);
        assert_eq!(storage::read("progress.unreadable.json").unwrap(), damaged);

        // Settings are written only when changed, so the same damaged file
        // is found again next launch; it's named again but not copied twice.
        // A different damaged file gets its own copy, and the first stays.
        storage::write(Save::FILE, b"[1, 2").unwrap();
        let g = Game::load(10);
        assert_eq!(
            g.unreadable
                .iter()
                .map(|u| u.kept.clone())
                .collect::<Vec<_>>(),
            vec![
                Ok("progress.unreadable-2.json".into()),
                Ok("settings.unreadable.json".into())
            ]
        );
        assert_eq!(storage::read("progress.unreadable.json").unwrap(), damaged);
        assert_eq!(
            storage::read("progress.unreadable-2.json").unwrap(),
            b"[1, 2"
        );

        // Readable, empty, and missing files make no copies.
        storage::write(Save::FILE, b"{\"embers\": 5}").unwrap();
        storage::write(Settings::FILE, b"  ").unwrap();
        let g = Game::load(11);
        assert!(g.unreadable.is_empty());
        assert_eq!(g.save.embers, 5);
        storage::remove(Save::FILE).unwrap();
        storage::remove(Settings::FILE).unwrap();
        assert!(Game::load(12).unreadable.is_empty());
        assert!(storage::read("settings.unreadable-2.json").is_none());
    }

    #[test]
    fn removing_a_controller_pauses_play_only() {
        let mut g = game();
        g.pad_prompts = true;
        g.controller_lost();
        assert_eq!(g.screen, Screen::Paused);
        assert!(g.pad_lost && !g.pad_prompts);
        g.tick(STEP, Input::default());
        assert!(g.pad_lost, "paused, so the note stays");
        g.screen = Screen::Playing;
        g.tick(STEP, Input::default());
        assert!(!g.pad_lost, "resuming clears it");
        for screen in [Screen::Title, Screen::Scroll, Screen::Camp, Screen::Dead] {
            g.screen = screen;
            g.controller_lost();
            assert_eq!((g.screen, g.pad_lost), (screen, false), "{screen:?}");
        }
    }

    #[test]
    fn practice_runs_never_save_a_checkpoint() {
        let mut g = game();
        use_exit(&mut g);
        g.travel();
        assert_eq!(g.stage, 1);
        assert!(stored_checkpoint().is_none());
    }
    /// Ticks until the hero is wounded and returns what was blamed.
    fn first_wound(mut g: Game) -> Cause {
        g.player.hp = 1000.;
        g.player.max_hp = 1000.;
        for _ in 0..360 {
            g.tick(STEP, Input::default());
            if g.player.hp < 1000. {
                return g.cause;
            }
        }
        panic!("nothing wounded the hero");
    }
    #[test]
    fn each_damage_source_is_named_as_the_cause() {
        let beside = |kind, phase| {
            let mut g = game();
            let mut e = Enemy::new(g.player.pos.x + 25., FLOOR, kind, Threat::BASE);
            if kind == EnemyKind::Moth {
                e.pos.y = g.player.pos.y - 14.;
                e.home_y = e.pos.y + 14.;
            }
            e.windup = 0.001;
            e.phase = phase;
            g.level.enemies.push(e);
            g
        };
        for kind in [EnemyKind::Warden, EnemyKind::Brute, EnemyKind::Moth] {
            assert_eq!(first_wound(beside(kind, 0)), Cause::Strike(kind));
        }
        assert_eq!(
            first_wound(beside(EnemyKind::Archer, 0)),
            Cause::Bolt(EnemyKind::Archer)
        );
        assert_eq!(
            first_wound(beside(EnemyKind::Regent, 0)),
            Cause::Strike(EnemyKind::Regent)
        );
        // Every third Regent attack is a fan of bolts.
        assert_eq!(
            first_wound(beside(EnemyKind::Regent, 2)),
            Cause::Bolt(EnemyKind::Regent)
        );
        let mut g = game();
        g.level.hazards = vec![g.player.rect()];
        assert_eq!(first_wound(g), Cause::Hazard);

        let mut g = beside(EnemyKind::Brute, 0);
        g.player.hp = 1.;
        for _ in 0..360 {
            g.tick(STEP, Input::default());
        }
        assert_eq!(g.screen, Screen::Dead);
        let recap = g.recap.expect("death leaves a recap");
        assert_eq!(recap.cause, Some(Cause::Strike(EnemyKind::Brute)));
        assert_eq!((recap.biome, recap.stage), (Biome::Aqueduct, 0));
        let mut g = game();
        g.player.embers = 9;
        g.screen = Screen::Paused;
        g.request_abandon();
        g.request_abandon();
        let recap = g.recap.unwrap();
        assert_eq!((recap.cause, recap.embers), (Some(Cause::Abandoned), 9));
    }
    #[test]
    fn records_count_as_new_only_when_a_saved_one_is_beaten() {
        let ended = |save: Save, kills: u32, stage: u32, win: Option<f32>| {
            let mut g = Game::new(42, save);
            g.start();
            g.level.enemies.clear();
            g.player.kills = kills;
            if stage > 0 || win.is_some() {
                g.stage = 1;
                g.travel();
                g.stage = stage.max(1);
            }
            if let Some(time) = win {
                g.screen = Screen::Camp;
                g.stage = 1;
                g.travel();
                g.level.enemies.clear();
                g.run_time = time;
                use_exit(&mut g);
                assert_eq!(g.screen, Screen::Victory);
            } else {
                g.hurt(10_000., 1., Cause::Hazard);
            }
            g
        };
        // A save from before records were kept: nothing is marked new.
        let old = Save {
            best_kills: 0,
            runs: 30,
            ..Default::default()
        };
        let g = ended(old, 4, 1, None);
        let r = g.recap.unwrap();
        assert!(!r.new_kills && !r.new_stage && !r.new_time);
        assert_eq!(
            (g.save.best_kills, g.save.best_stage, g.save.best_time),
            (4, Some(1), None)
        );
        // Beating saved records marks them; matching or falling short doesn't.
        let kept = Save {
            best_kills: 5,
            best_stage: Some(0),
            best_time: Some(600.),
            ..Default::default()
        };
        let r = ended(kept.clone(), 7, 1, None).recap.unwrap();
        assert!(r.new_kills && r.new_stage && !r.new_time);
        let r = ended(kept.clone(), 5, 0, None).recap.unwrap();
        assert!(!r.new_kills && !r.new_stage);
        let g = ended(kept.clone(), 0, 2, Some(500.));
        let r = g.recap.unwrap();
        assert!(r.new_time && r.new_stage && r.cause.is_none());
        // Using the gate takes one simulation step.
        assert!(g.save.best_time.is_some_and(|t| (t - 500.).abs() < 0.1));
        assert_eq!(g.save.best_stage, Some(2));
        let g = ended(kept.clone(), 0, 2, Some(700.));
        assert!(!g.recap.unwrap().new_time);
        assert_eq!(
            g.save.best_time,
            Some(600.),
            "a slower win keeps the record"
        );
        // The first victory sets the time without calling it a record.
        let g = ended(Save::default(), 0, 2, Some(900.));
        assert!(!g.recap.unwrap().new_time);
        assert!(g.save.best_time.is_some_and(|t| (t - 900.).abs() < 0.1));
        // Practice runs leave records alone.
        let mut g = Game::new(42, kept.clone());
        g.practice = true;
        g.screen = Screen::Playing;
        g.player.kills = 50;
        g.hurt(10_000., 1., Cause::Hazard);
        assert!(!g.recap.unwrap().new_kills);
        assert_eq!(g.save.best_kills, 5);
    }
    #[test]
    fn result_screens_accept_confirming_after_a_second() {
        let mut g = game();
        g.hurt(10_000., 1., Cause::Hazard);
        assert!(!g.result_ready());
        for _ in 0..115 {
            g.tick(STEP, Input::default());
        }
        assert!(!g.result_ready());
        for _ in 0..10 {
            g.tick(STEP, Input::default());
        }
        assert!(g.result_ready());
    }
    #[test]
    fn death_loses_unbanked_embers_only() {
        let mut g = game();
        g.save.embers = 30;
        g.player.embers = 12;
        g.hurt(1000., 1., Cause::Hazard);
        assert_eq!(g.screen, Screen::Dead);
        assert_eq!(g.player.embers, 0);
        assert_eq!(g.save.embers, 30);
    }
    /// A practice game with the hero standing at a lone object of `kind`.
    fn beside(kind: ObjectKind) -> Game {
        let mut g = game();
        g.level.objects = vec![Object {
            pos: vec2(900., FLOOR),
            kind,
            used: false,
        }];
        g.place_player(vec2(900., FLOOR));
        assert_eq!(g.nearby(), Some(0));
        g
    }
    #[test]
    fn prompts_say_what_an_object_needs_and_gives() {
        let prompt = |g: &Game| g.object_prompt(0);
        let mut g = beside(ObjectKind::Forge);
        g.player.gold = 35;
        assert_eq!(
            prompt(&g),
            ("TEMPER WEAPON / 35 OF 60 COPPER".into(), false)
        );
        g.player.gold = 60;
        assert_eq!(prompt(&g), ("TEMPER WEAPON / 60 COPPER".into(), true));
        let mut g = beside(ObjectKind::Secret);
        g.player.kills = 3;
        assert_eq!(
            prompt(&g),
            ("SEALED / 3 OF 8 GUARDIANS FELLED".into(), false)
        );
        g.save.rune = true;
        assert_eq!(
            prompt(&g),
            ("BREAK THE SEAL WITH THE CROWN RUNE".into(), true)
        );
        g.player.kills = 8;
        assert_eq!(prompt(&g), ("BREAK THE SEAL".into(), true));
        let mut g = beside(ObjectKind::Exit);
        assert_eq!(prompt(&g), ("RING THE BELLGATE".into(), true));
        g.player.embers = 18;
        assert_eq!(
            prompt(&g),
            ("RING THE BELLGATE / BANK 18 EMBERS".into(), true)
        );
        g.level.biome = Biome::Crown;
        g.level
            .enemies
            .push(Enemy::new(1200., FLOOR, EnemyKind::Regent, Threat::BASE));
        assert_eq!(prompt(&g), ("HELD SHUT BY THE BRASS REGENT".into(), false));
        g.level.enemies[0].hp = 0.;
        assert!(prompt(&g).1, "the gate opens once the Regent falls");
        for (kind, text) in [
            (ObjectKind::Scroll, "CLAIM A MEMORY"),
            (ObjectKind::Chest, "OPEN RELIQUARY"),
            (ObjectKind::Fountain, "DRINK FROM THE WELL"),
            (ObjectKind::Lore, "READ THE INSCRIPTION"),
        ] {
            assert_eq!(prompt(&beside(kind)), (text.into(), true), "unchanged");
        }
    }
    #[test]
    fn a_dimmed_prompt_means_interacting_is_refused() {
        // Whatever the prompt says, pressing interact does what it did:
        // a refusal leaves the object, and a ready prompt uses it.
        for (kind, gold, kills) in [
            (ObjectKind::Forge, 35, 0),
            (ObjectKind::Forge, 60, 0),
            (ObjectKind::Secret, 0, 3),
            (ObjectKind::Secret, 0, 8),
        ] {
            let mut g = beside(kind);
            g.player.gold = gold;
            g.player.kills = kills;
            let ready = g.object_prompt(0).1;
            g.interact();
            assert_eq!(g.level.objects[0].used, ready, "{kind:?} {gold} {kills}");
            assert_eq!(g.sounds.contains(&Sfx::Deny), !ready);
        }
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
        let mut e = Enemy::new(g.player.pos.x + 25., FLOOR, EnemyKind::Warden, Threat::BASE);
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
            from: None,
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
        g.level.enemies = vec![Enemy::new(500., -50., EnemyKind::Warden, Threat::BASE)];
        g.player.pos = vec2(750., -50.);
        for _ in 0..600 {
            g.update_enemies(STEP);
        }
        assert_eq!(g.level.enemies[0].pos, vec2(540., -50.));
        g.hit(0, 1., 1., false, true);
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
            g.level = Level::generate(42, biome, Threat::BASE);
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
