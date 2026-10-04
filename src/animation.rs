//! Animation state is independent of physics. Actions restart on their own edges;
//! idle/run loops and transitions retain consistent anchors in the sprite atlas.
use crate::game::{Player, Screen};
/// A small, sole-anchored deformation, applied in pixel-aligned horizontal
/// bands by the atlas. The sprite never rotates off the pixel grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionPose {
    pub scale_x: f32,
    pub scale_y: f32,
    pub lean: f32,
}
impl Default for MotionPose {
    fn default() -> Self {
        Self {
            scale_x: 1.,
            scale_y: 1.,
            lean: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Clip {
    Idle,
    Run,
    Rise,
    Fall,
    Land,
    Attack,
    Bolt,
    Throw,
    Place,
    Slam,
    Dodge,
    Parry,
    Heal,
    Hurt,
    Dead,
}
pub struct Animator {
    pub clip: Clip,
    pub elapsed: f32,
    previous_ground: bool,
    previous_hp: f32,
    previous_jumps: u32,
    initialized: bool,
    stride: f32,
    attack_duration: f32,
    landing: f32,
    landing_strength: f32,
    previous_vertical_speed: f32,
    momentum: f32,
    hurt: f32,
    pose: MotionPose,
}
impl Animator {
    pub fn new() -> Self {
        Self {
            clip: Clip::Idle,
            elapsed: 0.,
            previous_ground: true,
            previous_hp: 100.,
            previous_jumps: 0,
            initialized: false,
            stride: 0.,
            attack_duration: 0.27,
            landing: 0.,
            landing_strength: 0.,
            previous_vertical_speed: 0.,
            momentum: 0.,
            hurt: 0.,
            pose: MotionPose::default(),
        }
    }
    pub fn update(&mut self, p: &Player, screen: Screen, dt: f32) {
        if matches!(
            screen,
            Screen::Paused | Screen::Scroll | Screen::Camp | Screen::Victory
        ) {
            return;
        }
        // The renderer survives a new run, unlike Player. Transient reactions
        // from the previous life must not override the fresh player's pose.
        if !self.initialized || (self.previous_hp <= 0. && p.hp > 0.) {
            self.landing = 0.;
            self.hurt = 0.;
            self.previous_ground = p.ground;
            self.previous_hp = p.hp;
            self.previous_jumps = p.jumps;
            self.previous_vertical_speed = p.vel.y;
            self.momentum = 0.;
            self.pose = MotionPose::default();
            self.initialized = true;
        }
        self.landing = (self.landing - dt).max(0.);
        self.hurt = (self.hurt - dt).max(0.);
        if p.hp < self.previous_hp {
            self.hurt = 0.18;
        }
        if p.ground && !self.previous_ground {
            self.landing = 0.13;
            self.landing_strength = (self.previous_vertical_speed / 450.).clamp(0.25, 1.);
        }
        self.attack_duration = p.weapon.delay().min(0.28);
        let attack_age = p.weapon.delay() - p.attack_cd;
        let next = if p.hp <= 0. {
            Clip::Dead
        } else if screen == Screen::Title {
            Clip::Idle
        } else if p.dodge > 0. {
            Clip::Dodge
        } else if self.hurt > 0. {
            Clip::Hurt
        } else if p.heal_time > 0. {
            Clip::Heal
        } else if p.parry > 0. {
            Clip::Parry
        } else if p.attack_cd > 0. && attack_age < self.attack_duration {
            Clip::Attack
        } else if p.bow_cd > 0.32 - 0.16 {
            Clip::Bolt
        } else if p.grenade_cd > 5. - 0.22 {
            Clip::Throw
        } else if p.trap_cd > 8. - 0.2 {
            Clip::Place
        } else if p.slam && !p.ground {
            Clip::Slam
        } else if !p.ground {
            if p.vel.y < -10. {
                Clip::Rise
            } else {
                Clip::Fall
            }
        } else if self.landing > 0. {
            Clip::Land
        } else if p.vel.x.abs() > 18. {
            Clip::Run
        } else {
            Clip::Idle
        };
        let action_restart = match next {
            Clip::Rise => p.jumps > self.previous_jumps,
            _ => false,
        };
        if next != self.clip || action_restart {
            self.clip = next;
            self.elapsed = 0.;
        } else {
            self.elapsed += dt;
        }
        // Action poses follow simulation timers, including hitstop and repeated
        // held attacks. Render frame rate cannot outrun the actual strike.
        self.elapsed = match next {
            Clip::Attack => attack_age.max(0.),
            Clip::Dodge => (0.23 - p.dodge).max(0.),
            Clip::Parry => (0.2 - p.parry).max(0.),
            Clip::Heal => (0.8 - p.heal_time).max(0.),
            Clip::Bolt => (0.32 - p.bow_cd).max(0.),
            Clip::Throw => (5. - p.grenade_cd).max(0.),
            Clip::Place => (8. - p.trap_cd).max(0.),
            _ => self.elapsed,
        };
        if next == Clip::Run {
            self.stride += p.vel.x.abs() * dt;
        } else {
            self.stride = 0.;
        }
        self.previous_ground = p.ground;
        self.previous_hp = p.hp;
        self.previous_jumps = p.jumps;
        self.previous_vertical_speed = p.vel.y;
        if dt > 0. {
            let target = (p.vel.x / 145.).clamp(-1., 1.);
            self.momentum += (target - self.momentum) * (1. - (-18. * dt).exp());
            self.pose = self.motion_pose(p);
        }
        if next == Clip::Dead {
            self.pose = MotionPose::default();
        }
    }
    pub fn pose(&self) -> MotionPose {
        self.pose
    }
    pub fn hurt_flash(&self) -> f32 {
        if self.clip == Clip::Dead {
            return 0.;
        }
        ((self.hurt - 0.07) / 0.11).clamp(0., 1.)
    }
    fn motion_pose(&self, p: &Player) -> MotionPose {
        let mut pose = MotionPose::default();
        match self.clip {
            Clip::Idle | Clip::Run | Clip::Land => {
                // A fast landing compresses more than a short hop. Recovery is
                // monotonic and the foot pivot stays on the collision surface.
                let landing = (self.landing / 0.13).powi(2) * self.landing_strength;
                pose.scale_x += landing * 0.045;
                pose.scale_y -= landing * 0.07;
                pose.lean = self.momentum * 1.5;
            }
            Clip::Attack => {
                let phase = (self.elapsed / self.attack_duration).clamp(0., 1.);
                let ease = |t: f32| {
                    let t = t.clamp(0., 1.);
                    t * t * (3. - 2. * t)
                };
                let lean = if phase < 0.2 {
                    -0.9 * ease(phase / 0.2)
                } else if phase < 0.5 {
                    -0.9 + 3.3 * ease((phase - 0.2) / 0.3)
                } else {
                    2.4 * (1. - ease((phase - 0.5) / 0.5))
                };
                pose.lean = lean * p.face;
            }
            Clip::Rise => {
                let takeoff = (1. - self.elapsed / 0.12).clamp(0., 1.);
                pose.scale_x -= takeoff * 0.025;
                pose.scale_y += takeoff * 0.04;
                pose.lean = self.momentum;
            }
            Clip::Fall => {
                pose.lean = self.momentum * 0.7;
            }
            Clip::Slam => {
                pose.scale_x = 0.97;
                pose.scale_y = 1.04;
            }
            Clip::Dodge => {
                let tuck = (std::f32::consts::PI * self.elapsed / 0.23).sin().max(0.);
                pose.scale_x += tuck * 0.025;
                pose.scale_y -= tuck * 0.035;
                pose.lean = p.face * tuck;
            }
            Clip::Hurt => pose.lean = -p.face * 2. * (self.hurt / 0.18),
            Clip::Parry => pose.lean = -p.face * 0.8,
            Clip::Bolt => pose.lean = -p.face * (1. - self.elapsed / 0.16).max(0.),
            Clip::Throw => {
                pose.lean = p.face * (self.elapsed / 0.22 * std::f32::consts::PI).sin();
            }
            Clip::Place => {
                pose.scale_y = 0.98;
            }
            Clip::Heal | Clip::Dead => {}
        }
        pose
    }
    pub fn frame(&self) -> usize {
        match self.clip {
            Clip::Run => (self.stride / 145. * 14.) as usize % 8,
            Clip::Attack => 8 + ((self.elapsed / self.attack_duration * 8.) as usize).min(7),
            Clip::Bolt => 28,
            Clip::Throw => 9 + ((self.elapsed / 0.22 * 3.) as usize).min(2),
            Clip::Place => 20,
            Clip::Slam => 23,
            Clip::Idle => 16 + (self.elapsed * 6.) as usize % 4,
            Clip::Rise => {
                if self.elapsed < 0.07 {
                    20
                } else {
                    21
                }
            }
            Clip::Fall => {
                if self.elapsed < 0.05 {
                    22
                } else {
                    23
                }
            }
            Clip::Land => 20,
            Clip::Dodge => 24 + ((self.elapsed / 0.23 * 4.) as usize).min(3),
            Clip::Parry => 28,
            Clip::Heal => 29,
            Clip::Hurt => 30,
            Clip::Dead => 31,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::{Game, Input, Weapon},
        save::Save,
        world::STEP,
    };
    #[test]
    fn action_restarts_independent_of_global_clock() {
        let mut a = Animator::new();
        let mut p = Player::new(&Save::default());
        a.elapsed = 300.;
        p.attack = 0.19;
        p.attack_cd = 0.27;
        a.update(&p, Screen::Playing, 1. / 60.);
        assert_eq!(a.clip, Clip::Attack);
        assert_eq!(a.frame(), 8);
        for _ in 0..6 {
            p.attack_cd -= 1. / 60.;
            p.attack -= 1. / 60.;
            a.update(&p, Screen::Playing, 1. / 60.);
        }
        assert!(a.frame() > 8);
        p.attack = 0.;
        a.update(&p, Screen::Playing, 1. / 60.);
        p.attack = 0.19;
        p.attack_cd = p.weapon.delay();
        a.update(&p, Screen::Playing, 1. / 60.);
        assert_eq!(a.frame(), 8);
    }
    #[test]
    fn paused_animation_does_not_advance() {
        let mut a = Animator::new();
        let mut p = Player::new(&Save::default());
        p.vel.x = 100.;
        a.update(&p, Screen::Playing, 0.);
        a.update(&p, Screen::Playing, 0.2);
        let frame = a.frame();
        for screen in [
            Screen::Paused,
            Screen::Scroll,
            Screen::Camp,
            Screen::Victory,
        ] {
            a.update(&p, screen, 1.);
            assert_eq!(a.elapsed, 0.2);
            assert_eq!(a.frame(), frame);
        }
    }
    #[test]
    fn double_jump_restarts_takeoff_while_already_rising() {
        let mut a = Animator::new();
        let mut p = Player::new(&Save::default());
        p.ground = false;
        p.vel.y = -200.;
        p.jumps = 1;
        a.update(&p, Screen::Playing, 0.);
        a.update(&p, Screen::Playing, 0.1);
        assert_eq!(a.frame(), 21);

        p.jumps = 2;
        a.update(&p, Screen::Playing, 1. / 60.);
        assert_eq!(a.clip, Clip::Rise);
        assert_eq!(a.frame(), 20);
    }
    #[test]
    fn death_overrides_actions_and_new_run_clears_reactions() {
        let mut a = Animator::new();
        let mut p = Player::new(&Save::default());
        p.hp = 0.;
        p.dodge = 0.23;
        p.attack = 0.19;
        p.attack_cd = p.weapon.delay();
        a.update(&p, Screen::Dead, 1. / 60.);
        assert_eq!(a.clip, Clip::Dead);
        assert_eq!(a.frame(), 31);

        let fresh = Player::new(&Save::default());
        a.update(&fresh, Screen::Playing, 1. / 60.);
        assert_eq!(a.clip, Clip::Idle);
        assert_eq!(a.frame(), 16);
    }
    #[test]
    fn dodge_restarts_and_action_frames_do_not_wrap() {
        let mut a = Animator::new();
        let mut p = Player::new(&Save::default());
        p.dodge = 0.23;
        a.update(&p, Screen::Playing, 0.);
        p.dodge = 0.03;
        a.update(&p, Screen::Playing, 0.2);
        assert_eq!(a.frame(), 27);
        p.dodge = 0.01;
        a.update(&p, Screen::Playing, 0.1);
        assert_eq!(a.frame(), 27);
        p.dodge = 0.23;
        a.update(&p, Screen::Playing, 1. / 60.);
        assert_eq!(a.frame(), 24);

        a.clip = Clip::Attack;
        a.elapsed = 100.;
        assert_eq!(a.frame(), 15);
    }
    #[test]
    fn every_weapon_completes_and_restarts_held_attack() {
        for weapon in [Weapon::Sabre, Weapon::Glaive, Weapon::Hammer] {
            let mut game = Game::new(42, Save::default());
            game.practice = true;
            game.screen = Screen::Playing;
            game.level.enemies.clear();
            game.player.weapon = weapon;
            let mut animator = Animator::new();
            let mut starts = 0;
            let mut recoveries = 0;
            let mut last_frame = usize::MAX;
            for _ in 0..240 {
                game.tick(
                    STEP,
                    Input {
                        attack: true,
                        ..Input::default()
                    },
                );
                animator.update(&game.player, game.screen, STEP);
                let frame = animator.frame();
                if animator.clip == Clip::Attack && frame != last_frame {
                    starts += usize::from(frame == 8);
                    recoveries += usize::from(frame == 15);
                }
                last_frame = frame;
            }
            assert!(starts >= 3, "{weapon:?} must repeat while held");
            assert!(
                recoveries >= starts - 1,
                "{weapon:?} must reach final recovery frame"
            );
        }
    }
    #[test]
    fn attack_and_dodge_pose_hold_when_simulation_timers_hold() {
        let mut p = Player::new(&Save::default());
        let mut a = Animator::new();
        p.attack_cd = p.weapon.delay() - 0.1;
        a.update(&p, Screen::Playing, 0.1);
        let frame = a.frame();
        for _ in 0..12 {
            a.update(&p, Screen::Playing, 0.);
            assert_eq!(a.frame(), frame);
        }
        p.dodge = 0.13;
        a.update(&p, Screen::Playing, 0.);
        let frame = a.frame();
        a.update(&p, Screen::Playing, 0.);
        assert_eq!(a.frame(), frame);
    }
    #[test]
    fn run_stride_follows_distance_instead_of_wall_clock() {
        let mut p = Player::new(&Save::default());
        let mut slow = Animator::new();
        let mut fast = Animator::new();
        p.vel.x = 50.;
        slow.update(&p, Screen::Playing, 0.2);
        p.vel.x = 100.;
        fast.update(&p, Screen::Playing, 0.1);
        assert_eq!(slow.frame(), fast.frame());
        assert_eq!(slow.frame(), 0);
    }
    #[test]
    fn skill_feedback_and_slam_have_action_poses() {
        let mut p = Player::new(&Save::default());
        let mut a = Animator::new();
        p.bow_cd = 0.32;
        a.update(&p, Screen::Playing, 0.);
        assert_eq!(a.clip, Clip::Bolt);
        p.bow_cd = 0.;
        p.grenade_cd = 5.;
        a.update(&p, Screen::Playing, 0.);
        assert_eq!(a.clip, Clip::Throw);
        p.grenade_cd = 0.;
        p.trap_cd = 8.;
        a.update(&p, Screen::Playing, 0.);
        assert_eq!(a.clip, Clip::Place);
        p.trap_cd = 0.;
        p.slam = true;
        p.ground = false;
        p.vel.y = 600.;
        a.update(&p, Screen::Playing, 0.);
        assert_eq!(a.clip, Clip::Slam);
    }
    #[test]
    fn landing_reaction_cannot_hide_new_jump_or_hurt() {
        let mut p = Player::new(&Save::default());
        let mut a = Animator::new();
        p.ground = false;
        p.vel.y = 100.;
        a.update(&p, Screen::Playing, 0.);
        p.ground = true;
        a.update(&p, Screen::Playing, STEP);
        assert_eq!(a.clip, Clip::Land);
        p.ground = false;
        p.vel.y = -300.;
        p.jumps = 1;
        a.update(&p, Screen::Playing, STEP);
        assert_eq!(a.clip, Clip::Rise);
        p.hp -= 10.;
        a.update(&p, Screen::Playing, STEP);
        assert_eq!(a.clip, Clip::Hurt);
        p.hp = 0.;
        a.update(&p, Screen::Dead, 0.);
        assert_eq!(a.clip, Clip::Dead);
    }
    #[test]
    fn landing_compression_tracks_impact_speed_and_recovers_without_overshoot() {
        let mut player = Player::new(&Save::default());
        let mut gentle = Animator::new();
        let mut hard = Animator::new();
        player.ground = false;
        player.vel.y = 80.;
        gentle.update(&player, Screen::Playing, STEP);
        player.vel.y = 600.;
        hard.update(&player, Screen::Playing, STEP);
        player.ground = true;
        player.vel.y = 0.;
        gentle.update(&player, Screen::Playing, STEP);
        hard.update(&player, Screen::Playing, STEP);
        assert!(hard.pose().scale_y < gentle.pose().scale_y);
        assert!(hard.pose().scale_y >= 0.93);
        let mut last = hard.pose().scale_y;
        for _ in 0..20 {
            hard.update(&player, Screen::Playing, STEP);
            assert!(hard.pose().scale_y >= last);
            assert!(hard.pose().scale_y <= 1.);
            last = hard.pose().scale_y;
        }
        assert_eq!(hard.pose(), MotionPose::default());
    }
    #[test]
    fn attack_body_follows_windup_contact_and_recovery() {
        let mut player = Player::new(&Save::default());
        let mut a = Animator::new();
        player.attack_cd = player.weapon.delay() * 0.8;
        a.update(&player, Screen::Playing, STEP);
        let windup = a.pose().lean;
        player.attack_cd = player.weapon.delay() * 0.5;
        a.update(&player, Screen::Playing, STEP);
        let contact = a.pose().lean;
        player.attack_cd = player.weapon.delay() * 0.1;
        a.update(&player, Screen::Playing, STEP);
        let recovery = a.pose().lean;
        assert!(windup < 0.);
        assert!(contact > 2.);
        assert!(recovery >= 0. && recovery < contact);
        let frozen_pose = a.pose();
        a.update(&player, Screen::Playing, 0.);
        a.update(&player, Screen::Paused, 1.);
        assert_eq!(a.pose(), frozen_pose);
    }
    #[test]
    fn death_clears_deformation_and_does_not_freeze_a_white_hit_flash() {
        let mut player = Player::new(&Save::default());
        let mut a = Animator::new();
        a.update(&player, Screen::Playing, STEP);
        player.hp -= 10.;
        a.update(&player, Screen::Playing, STEP);
        assert!(a.hurt_flash() > 0.);
        player.hp = 0.;
        a.update(&player, Screen::Dead, 0.);
        assert_eq!(a.pose(), MotionPose::default());
        assert_eq!(a.hurt_flash(), 0.);
    }
}
