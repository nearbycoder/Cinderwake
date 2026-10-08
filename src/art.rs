//! ImageGen character and environment atlases with runtime animation and composition.
use crate::{
    animation::{Animator, MotionPose},
    atlas::Atlas,
    game::{Game, Screen},
    world::{Enemy, EnemyKind},
};
use macroquad::prelude::*;
use std::collections::HashMap;

/// Enemy clips are selected from actual movement and attack edges. Distance
/// from a spawn point says nothing about whether an enemy is walking now.
struct EnemyAnimator {
    position: Vec2,
    was_winding: bool,
    attack_age: Option<f32>,
    elapsed: f32,
    stride: f32,
    frame: usize,
    pose: MotionPose,
}
impl EnemyAnimator {
    fn new(e: &Enemy) -> Self {
        Self {
            position: e.pos,
            was_winding: e.windup > 0.,
            attack_age: None,
            elapsed: 0.,
            stride: 0.,
            frame: if e.windup > 0. { 5 } else { 0 },
            pose: MotionPose::default(),
        }
    }
    fn update(&mut self, e: &Enemy, dt: f32) {
        let distance = (e.pos - self.position).length();
        self.position = e.pos;
        if e.stun > 0. {
            // Recovery doubles as a recoil pose; frozen wingbeats make a
            // stunned moth just as readable as a grounded enemy.
            if e.kind != EnemyKind::Moth {
                self.frame = 7;
            }
            self.pose.lean = -e.face * 2. * (e.flash / 0.12).clamp(0., 1.);
            return;
        }
        if dt <= 0. {
            return;
        }
        self.elapsed += dt;
        if self.was_winding && e.windup <= 0. {
            self.attack_age = Some(0.);
        } else if let Some(age) = &mut self.attack_age {
            *age += dt;
        }
        self.was_winding = e.windup > 0.;
        let moving = distance / dt > 1.;
        if moving {
            self.stride += distance;
        } else {
            self.stride = 0.;
        }
        self.frame = if e.kind == EnemyKind::Moth {
            (self.elapsed * 12.) as usize % 8
        } else if e.windup > 0. {
            if e.kind == EnemyKind::Regent && e.windup > 0.3 {
                4
            } else {
                5
            }
        } else if self.attack_age.is_some_and(|age| age < 0.12) {
            6
        } else if self.attack_age.is_some_and(|age| age < 0.28) {
            7
        } else if moving {
            let frames = if e.kind == EnemyKind::Regent { 2 } else { 3 };
            2 + (self.stride / 6.) as usize % frames
        } else {
            (self.elapsed * 3.) as usize % 2
        };
        self.pose = MotionPose::default();
        if e.kind != EnemyKind::Moth {
            self.pose.lean = if e.windup > 0. {
                -e.face * 1.5 * (1. - e.windup / 0.65).clamp(0., 1.)
            } else if let Some(age) = self.attack_age.filter(|age| *age < 0.28) {
                e.face * 2.5 * (1. - age / 0.28).powi(2)
            } else if moving {
                e.face * 0.75
            } else {
                0.
            };
        }
    }
}

#[derive(Clone, Copy)]
struct Afterimage {
    position: Vec2,
    face: f32,
    frame: usize,
    pose: MotionPose,
    age: f32,
}

/// These are sampled poses in world space, so camera motion and changing roll
/// frames cannot drag or rewrite the trail behind the player.
#[derive(Default)]
struct DodgeTrail {
    samples: Vec<Afterimage>,
    sample_clock: f32,
    was_dodging: bool,
}
impl DodgeTrail {
    const LIFETIME: f32 = 0.18;
    fn update(
        &mut self,
        position: Vec2,
        face: f32,
        frame: usize,
        pose: MotionPose,
        dodging: bool,
        dt: f32,
    ) {
        if dt <= 0. {
            return;
        }
        for sample in &mut self.samples {
            sample.age += dt;
        }
        self.samples.retain(|s| s.age < Self::LIFETIME);
        if dodging {
            self.sample_clock += dt;
            if !self.was_dodging || self.sample_clock >= 0.035 {
                self.sample_clock = 0.;
                let moved = self
                    .samples
                    .last()
                    .is_none_or(|last| (last.position - position).length_squared() > 1.);
                if moved {
                    self.samples.push(Afterimage {
                        position,
                        face,
                        frame,
                        pose,
                        age: 0.,
                    });
                }
            }
        } else {
            self.sample_clock = 0.;
        }
        self.was_dodging = dodging;
    }
}

fn enemy_key(e: &Enemy) -> (u32, u8) {
    (e.home.to_bits(), e.kind as u8)
}

pub struct Art {
    hero: Atlas,
    pub animator: Animator,
    foes: Atlas,
    boss: Atlas,
    enemy_animations: HashMap<(u32, u8), EnemyAnimator>,
    generation: Option<(u64, u32, u8)>,
    dodge_trail: DodgeTrail,
    hero_opacity: f32,
    /// Whether textures are sampled from mipmaps (Ultra).
    smooth: bool,
    /// Whether the mipmaps have been built.
    mipmapped: bool,
    pub environment: crate::environment::Environment,
}
impl Art {
    pub fn new() -> Self {
        let hero = Atlas::new(
            include_bytes!("../assets/sprites/wanderer-v2.png"),
            8,
            4,
            40.,
            16..20,
        );
        let foes = Atlas::new(
            include_bytes!("../assets/sprites/guardians-v1.png"),
            8,
            4,
            38.,
            0..2,
        );
        Self {
            hero,
            environment: crate::environment::Environment::new(),
            animator: Animator::new(),
            foes,
            enemy_animations: HashMap::new(),
            generation: None,
            dodge_trail: DodgeTrail::default(),
            hero_opacity: 1.,
            smooth: false,
            mipmapped: false,
            boss: Atlas::new(
                include_bytes!("../assets/sprites/regent-v1.png"),
                4,
                2,
                75.,
                0..2,
            ),
        }
    }
    /// Samples characters and scenery smoothly from mipmaps (`true`), or
    /// nearest-neighbour as they always have been. Magnified texels stay
    /// sharp either way; only shrinking is filtered.
    pub fn set_smooth(&mut self, smooth: bool) {
        // Browsers' WebGL 1 can't build mipmaps for these textures, whose
        // sizes aren't powers of two, so the browser build stays nearest.
        let smooth = smooth && cfg!(not(target_arch = "wasm32"));
        if self.smooth == smooth {
            return;
        }
        self.smooth = smooth;
        // SAFETY: called between frames, before anything is drawn with them.
        let gl = unsafe { get_internal_gl() };
        for texture in self
            .hero
            .textures()
            .chain(self.foes.textures())
            .chain(self.boss.textures())
            .chain(self.environment.textures())
        {
            let id = texture.raw_miniquad_id();
            if smooth {
                // The art never changes, so its mipmaps are built once.
                if !self.mipmapped {
                    gl.quad_context.texture_generate_mipmaps(id);
                }
                gl.quad_context.texture_set_min_filter(
                    id,
                    FilterMode::Linear,
                    miniquad::MipmapFilterMode::Linear,
                );
            } else {
                gl.quad_context.texture_set_min_filter(
                    id,
                    FilterMode::Nearest,
                    miniquad::MipmapFilterMode::None,
                );
            }
        }
        self.mipmapped |= smooth;
    }
    /// Forgets all animation state, as at launch, so a replayed script
    /// draws exactly the same frames again.
    pub fn restart(&mut self) {
        self.animator = Animator::new();
        self.enemy_animations.clear();
        self.generation = None;
        self.dodge_trail = DodgeTrail::default();
        self.hero_opacity = 1.;
    }
    pub fn animate(&mut self, game: &Game, dt: f32) {
        let generation = (game.seed, game.stage, game.level.biome as u8);
        if self.generation != Some(generation) {
            self.generation = Some(generation);
            self.enemy_animations.clear();
            self.animator = Animator::new();
            self.dodge_trail = DodgeTrail::default();
        }
        let frozen = game.screen.freezes_world()
            || matches!(game.screen, Screen::Dead | Screen::Victory)
            || game.hitstop > 0.;
        let dt = if frozen { 0. } else { dt };
        self.animator.update(&game.player, game.screen, dt);
        self.dodge_trail.update(
            game.player.pos,
            game.player.face,
            self.animator.frame(),
            self.animator.pose(),
            game.player.dodge > 0.,
            dt,
        );
        self.hero_opacity = if game.player.invuln > 0. && game.player.dodge <= 0. {
            // Retain a readable silhouette during immunity instead of removing
            // the entire character on alternate frames.
            0.79 + ((0.9 - game.player.invuln) * 45.).cos() * 0.21
        } else {
            1.
        };
        for e in &game.level.enemies {
            let animation = self
                .enemy_animations
                .entry(enemy_key(e))
                .or_insert_with(|| EnemyAnimator::new(e));
            if !frozen && e.hp > 0. {
                animation.update(e, dt);
            }
        }
    }
    pub fn player(&self, x: f32, y: f32, face: f32, opacity: f32) {
        let opacity = opacity * self.hero_opacity;
        self.hero.draw_pose(
            self.animator.frame(),
            vec2(x, y),
            face,
            Color::new(1., 1., 1., opacity),
            self.animator.pose(),
        );
        let flash = self.animator.hurt_flash();
        if flash > 0. {
            self.hero.draw_silhouette(
                self.animator.frame(),
                vec2(x, y),
                face,
                Color::new(1., 0.94, 0.83, flash * opacity),
                self.animator.pose(),
            );
        }
    }
    pub fn afterimages(&self, camera: f32) {
        for sample in &self.dodge_trail.samples {
            let fade = (1. - sample.age / DodgeTrail::LIFETIME).powi(2);
            self.hero.draw_silhouette(
                sample.frame,
                sample.position - vec2(camera, 0.),
                sample.face,
                Color::new(0.38, 0.86, 0.83, fade * 0.24),
                sample.pose,
            );
        }
    }
    pub fn preview(&self, time: f32) {
        let clips = [
            ("IDLE", 16 + (time * 6.) as usize % 4),
            ("RUN", (time * 14.) as usize % 8),
            ("SLASH", 8 + (time * 22.) as usize % 8),
            ("DODGE", 24 + (time * 12.) as usize % 4),
            ("JUMP", 20 + (time * 5.) as usize % 4),
            ("PARRY", 28),
            ("HEAL", 29),
            ("HURT", 30),
        ];
        for (i, (name, frame)) in clips.iter().enumerate() {
            let x = 110. + (i % 4) as f32 * 150.;
            let y = if i < 4 { 153. } else { 292. };
            draw_rectangle(x - 65., y - 102., 130., 121., Color::from_hex(0x172b3c));
            draw_line(x - 56., y, x + 56., y, 1., Color::from_hex(0x5c7d7a));
            self.hero.draw_scaled(*frame, vec2(x, y), 1., 1., 2.);
            draw_text(name, x - 22., y + 14., 12., Color::from_hex(0xecd7a2));
        }
    }
    pub fn enemy(&self, e: &Enemy, x: f32, _t: f32) {
        let animation = self.enemy_animations.get(&enemy_key(e));
        let phase = animation.map_or(0, |animation| animation.frame);
        let mut pose = animation.map_or_else(MotionPose::default, |animation| animation.pose);
        let (atlas, frame) = if e.kind == EnemyKind::Regent {
            (&self.boss, phase)
        } else {
            let (frame, factor) = match e.kind {
                EnemyKind::Warden => (phase, 1.),
                EnemyKind::Archer => (8 + phase, 1.05),
                EnemyKind::Brute => (16 + phase, 1.1),
                EnemyKind::Moth => (24 + phase, 0.7),
                EnemyKind::Regent => unreachable!(),
            };
            pose.scale_x *= factor;
            pose.scale_y *= factor;
            (&self.foes, frame)
        };
        atlas.draw_pose(frame, vec2(x, e.pos.y), e.face, WHITE, pose);
        if e.flash > 0. {
            atlas.draw_silhouette(
                frame,
                vec2(x, e.pos.y),
                e.face,
                Color::new(1., 0.97, 0.84, (e.flash / 0.12).clamp(0., 1.).sqrt()),
                pose,
            );
        }
    }
}

pub fn pixel_line(a: Vec2, b: Vec2, width: f32, col: Color) {
    let steps = (b - a).abs().max_element().ceil() as i32;
    for i in 0..=steps {
        let p = a + (b - a) * (i as f32 / steps.max(1) as f32);
        draw_rectangle(
            (p.x - width / 2.).round(),
            (p.y - width / 2.).round(),
            width.ceil(),
            width.ceil(),
            col,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::FLOOR;

    #[test]
    fn dodge_trail_keeps_historical_world_positions_frames_and_facing() {
        let mut trail = DodgeTrail::default();
        let pose = MotionPose::default();
        trail.update(vec2(100., 286.), 1., 24, pose, true, 0.02);
        trail.update(vec2(120., 286.), -1., 25, pose, true, 0.04);
        assert_eq!(trail.samples.len(), 2);
        assert_eq!(trail.samples[0].position, vec2(100., 286.));
        assert_eq!(trail.samples[0].frame, 24);
        assert_eq!(trail.samples[0].face, 1.);
        assert_eq!(trail.samples[1].position, vec2(120., 286.));
        assert_eq!(trail.samples[1].frame, 25);
        assert_eq!(trail.samples[1].face, -1.);
        let age = trail.samples[0].age;
        trail.update(vec2(1000., 200.), 1., 26, pose, true, 0.);
        assert_eq!(trail.samples.len(), 2);
        assert_eq!(trail.samples[0].age, age);
        trail.update(vec2(130., 286.), 1., 16, pose, false, 0.19);
        assert!(trail.samples.is_empty());
    }

    #[test]
    fn dodge_against_a_wall_does_not_stack_duplicate_ghosts() {
        let mut trail = DodgeTrail::default();
        for _ in 0..20 {
            trail.update(vec2(12., 286.), -1., 24, MotionPose::default(), true, 0.01);
            assert!(trail.samples.len() <= 1);
        }
        for i in 0..1000 {
            trail.update(
                vec2(i as f32 * 4., 286.),
                1.,
                25,
                MotionPose::default(),
                true,
                0.01,
            );
            assert!(
                trail.samples.len() <= 6,
                "the short trail must remain bounded"
            );
        }
    }

    #[test]
    fn enemy_walk_uses_motion_even_at_spawn_and_stops_away_from_spawn() {
        let mut e = Enemy::new(400., FLOOR, EnemyKind::Warden, crate::world::Threat::BASE);
        let mut a = EnemyAnimator::new(&e);
        e.pos.x += 1.;
        a.update(&e, 1. / 60.);
        assert!((2..=4).contains(&a.frame));
        e.pos.x += 100.;
        a.update(&e, 1. / 60.);
        a.update(&e, 1. / 60.);
        assert!(a.frame <= 1, "stationary enemy should idle after chasing");
    }

    #[test]
    fn cooldown_at_spawn_is_not_mistaken_for_an_attack() {
        let mut e = Enemy::new(400., FLOOR, EnemyKind::Regent, crate::world::Threat::BASE);
        e.timer = 0.7;
        let mut a = EnemyAnimator::new(&e);
        a.update(&e, 1. / 60.);
        assert!(a.frame <= 1);
        e.windup = 0.6;
        a.update(&e, 1. / 60.);
        assert_eq!(a.frame, 4);
        e.windup = 0.2;
        a.update(&e, 0.4);
        assert_eq!(a.frame, 5);
        e.windup = 0.;
        e.timer = 0.85;
        a.update(&e, 1. / 60.);
        assert_eq!(a.frame, 6);
        a.update(&e, 0.13);
        assert_eq!(a.frame, 7);
        a.update(&e, 0.2);
        assert!(a.frame <= 1);
    }

    #[test]
    fn stun_and_zero_simulation_time_freeze_enemy_animation() {
        let mut e = Enemy::new(400., FLOOR, EnemyKind::Moth, crate::world::Threat::BASE);
        let mut a = EnemyAnimator::new(&e);
        a.update(&e, 0.25);
        let frame = a.frame;
        a.update(&e, 0.);
        assert_eq!(a.frame, frame);
        e.stun = 0.2;
        a.update(&e, 0.15);
        assert_eq!(a.frame, frame);
        e.kind = EnemyKind::Warden;
        e.windup = 0.2;
        a.update(&e, 0.1);
        assert_eq!(a.frame, 7, "stunned windup should visibly recoil");
    }
}
