//! Drawing between simulation steps.
//!
//! The simulation runs at a fixed 120 steps per second, but displays refresh
//! at their own rates and frames never arrive exactly on time, so a frame
//! usually falls between two steps. Drawing only the latest step makes some
//! frames repeat the one before and others jump two steps, which shows as
//! judder. `Pose` remembers where the moving things were before the latest
//! step, and `Pose::show` places them the leftover fraction of the way to
//! where they are now, for drawing only. `Pose::restore` puts the real
//! positions back before the next step, so the simulation never sees a
//! blended value. Particles and floating numbers, which come and go every
//! step, are drawn back along their motion instead.
use crate::game::Game;
use crate::particles::{self, Particle};
use crate::world;
use macroquad::prelude::*;

/// Anything that moves further than this in one step has jumped (travel, a
/// respawn on a safe ledge, a new level) and is drawn where it landed.
const JUMP: f32 = 24.;

/// Positions of everything that moves smoothly: the camera, the hero,
/// guardians, and bolts. The pose `show` returns also holds the particles
/// and floating numbers as they were, for `restore`.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct Pose {
    camera: Vec2,
    hero: Vec2,
    enemies: Vec<Vec2>,
    shots: Vec<Vec2>,
    particles: Vec<Particle>,
    texts: Vec<(Vec2, f32)>,
}

impl Pose {
    pub fn of(g: &Game) -> Self {
        let mut pose = Self::default();
        pose.record(g);
        pose
    }
    /// Remembers the current positions, before a step (reusing the lists).
    pub fn record(&mut self, g: &Game) {
        self.camera = vec2(g.camera, g.camera_y);
        self.hero = g.player.pos;
        self.enemies.clear();
        self.enemies.extend(g.level.enemies.iter().map(|e| e.pos));
        self.shots.clear();
        self.shots.extend(g.shots.iter().map(|s| s.pos));
    }
    /// Moves `g` to `alpha` (0 to 1) of the way from this recorded pose to
    /// its current one, and returns the current pose for `restore`.
    pub fn show(&self, g: &mut Game, alpha: f32) -> Self {
        let mut now = Self::of(g);
        let alpha = alpha.clamp(0., 1.);
        let back = world::STEP * (1. - alpha);
        now.particles.clone_from(&g.particles);
        now.texts.extend(g.texts.iter().map(|t| (t.pos, t.life)));
        particles::rewind(&mut g.particles, back);
        for t in &mut g.texts {
            // Floating numbers rise at a steady 22 units a second.
            t.pos.y += 22. * back;
            t.life += back;
        }
        let camera = between(self.camera, now.camera, alpha);
        (g.camera, g.camera_y) = (camera.x, camera.y);
        g.player.pos = between(self.hero, now.hero, alpha);
        // Guardians stay in the list when they fall, so a different length
        // means a different level.
        if self.enemies.len() == g.level.enemies.len() {
            for (e, before) in g.level.enemies.iter_mut().zip(&self.enemies) {
                e.pos = between(*before, e.pos, alpha);
            }
        }
        if self.shots.len() == g.shots.len() {
            for (s, before) in g.shots.iter_mut().zip(&self.shots) {
                s.pos = between(*before, s.pos, alpha);
            }
        } else {
            // A bolt was fired or ended this step, so the lists no longer
            // line up; each bolt is drawn back along its flight instead.
            for s in &mut g.shots {
                s.pos -= s.vel * back;
            }
        }
        now
    }
    /// Puts back the positions `show` returned.
    pub fn restore(&self, g: &mut Game) {
        (g.camera, g.camera_y) = (self.camera.x, self.camera.y);
        g.player.pos = self.hero;
        for (e, pos) in g.level.enemies.iter_mut().zip(&self.enemies) {
            e.pos = *pos;
        }
        for (s, pos) in g.shots.iter_mut().zip(&self.shots) {
            s.pos = *pos;
        }
        if self.particles.len() == g.particles.len() {
            g.particles.clone_from(&self.particles);
        }
        for (t, (pos, life)) in g.texts.iter_mut().zip(&self.texts) {
            (t.pos, t.life) = (*pos, *life);
        }
    }
}

fn between(before: Vec2, now: Vec2, alpha: f32) -> Vec2 {
    if before.distance(now) > JUMP {
        now
    } else {
        before.lerp(now, alpha)
    }
}

/// How far into the next step a frame falls, given the simulated time left
/// over after the frame's whole steps.
pub fn leftover(accumulator: f32) -> f32 {
    (accumulator / world::STEP).clamp(0., 1.)
}

/// Frame timing gathered by `--pacing-check`: for each frame, the real
/// seconds since the one before and the whole steps it ran.
#[derive(Default)]
pub struct Pacing {
    frames: Vec<(f32, u32)>,
}

impl Pacing {
    pub fn push(&mut self, seconds: f32, steps: u32) {
        self.frames.push((seconds, steps));
    }
    /// Real seconds gathered so far.
    pub fn seconds(&self) -> f32 {
        self.frames.iter().map(|f| f.0).sum()
    }
    /// A summary of how evenly whole steps would have fallen across these
    /// frames. Without blending, a frame shows the moment of its latest step,
    /// so its view advances by whole steps however long the frame took.
    pub fn report(&self) -> String {
        let n = self.frames.len().max(1);
        let mut intervals: Vec<f32> = self.frames.iter().map(|f| f.0 * 1000.).collect();
        intervals.sort_by(f32::total_cmp);
        let at = |q: f32| intervals[((intervals.len() - 1) as f32 * q) as usize];
        let mean = intervals.iter().sum::<f32>() / n as f32;
        let mut counts = [0usize; 4];
        for (_, steps) in &self.frames {
            counts[(*steps as usize).min(3)] += 1;
        }
        // The usual number of steps per frame at this refresh rate.
        let usual = (0..4).max_by_key(|i| counts[*i]).unwrap_or(1);
        let uneven = n - counts[usual];
        // Without blending, a frame shows the moment of its latest step,
        // which lags real time by the leftover time. That lag changing from
        // frame to frame is the judder; with blending the lag is always one
        // step, so it doesn't change.
        let (mut real, mut shown) = (0., 0.);
        let lags: Vec<f32> = self
            .frames
            .iter()
            .map(|(seconds, steps)| {
                real += seconds;
                shown += *steps as f32 * world::STEP;
                real - shown
            })
            .collect();
        let lag = lags.iter().sum::<f32>() / n as f32;
        let wobble = lags.iter().map(|l| (l - lag).abs()).sum::<f32>() / n as f32;
        format!(
            "frames={n} interval_ms(mean={mean:.2} p5={:.2} p95={:.2}) \
             steps_per_frame(0={} 1={} 2={} 3+={}) \
             uneven_without_blending={uneven} ({:.1}%) \
             shown_moment_wobble_ms(without_blending={:.2} with_blending=0_by_design)",
            at(0.05),
            at(0.95),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
            uneven as f32 * 100. / n as f32,
            wobble * 1000.,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Input, Screen, Shot};
    use crate::save::Save;

    fn running() -> Input {
        Input {
            axis: 1.,
            ..Default::default()
        }
    }
    fn practice(clear: bool) -> Game {
        let mut g = Game::new(4017, Save::default());
        g.start();
        g.intro = 0.;
        if clear {
            g.level.enemies.clear();
            g.level.hazards.clear();
        }
        g
    }
    /// Runs frames of the given lengths the way the main loop does and
    /// returns each frame's length and the hero's drawn position.
    fn frames(lengths: &[f32], blend: bool) -> Vec<(f32, f32)> {
        let mut g = practice(true);
        let mut pose = Pose::of(&g);
        let mut accumulator = 0.;
        lengths
            .iter()
            .map(|&dt| {
                accumulator += dt;
                while accumulator >= world::STEP {
                    pose.record(&g);
                    g.tick(world::STEP, running());
                    accumulator -= world::STEP;
                }
                let x = if blend {
                    let now = pose.show(&mut g, leftover(accumulator));
                    let x = g.player.pos.x;
                    now.restore(&mut g);
                    x
                } else {
                    g.player.pos.x
                };
                (dt, x)
            })
            .collect()
    }
    /// The hero's drawn speed in each frame after it reaches full speed,
    /// relative to the median.
    fn speeds(lengths: &[f32], blend: bool) -> Vec<f32> {
        let drawn = frames(lengths, blend);
        let start = lengths.len() / 3;
        let speeds: Vec<f32> = drawn
            .windows(2)
            .skip(start)
            .map(|w| (w[1].1 - w[0].1) / w[1].0)
            .collect();
        let mut sorted = speeds.clone();
        sorted.sort_by(f32::total_cmp);
        let median = sorted[sorted.len() / 2];
        assert!(median > 50., "the hero runs ({median})");
        speeds.iter().map(|s| s / median).collect()
    }
    fn display(hz: f32, seconds: f32) -> Vec<f32> {
        vec![1. / hz; (hz * seconds) as usize]
    }

    #[test]
    fn running_moves_evenly_at_any_refresh_rate() {
        // 120 Hz with a small wobble, as frames really arrive.
        let wobbly: Vec<f32> = (0..240)
            .map(|i| 1. / 120. + [0.0004, -0.0005, 0.0002, -0.0001][i % 4])
            .collect();
        for (name, lengths) in [
            ("60 Hz", display(60., 2.)),
            ("120 Hz wobbling", wobbly),
            ("144 Hz", display(144., 2.)),
            ("165 Hz", display(165., 2.)),
        ] {
            let smooth = speeds(&lengths, true);
            let worst = smooth.iter().map(|s| (s - 1.).abs()).fold(0., f32::max);
            assert!(worst < 0.1, "{name}: blended speed strays by {worst}");
            if name != "60 Hz" {
                // Whole steps alone repeat a frame or jump two steps.
                let stepped = speeds(&lengths, false);
                let worst = stepped.iter().map(|s| (s - 1.).abs()).fold(0., f32::max);
                assert!(worst > 0.5, "{name}: whole steps stray by only {worst}");
            }
        }
    }

    #[test]
    fn a_jump_is_drawn_where_it_lands() {
        let mut g = practice(true);
        let pose = Pose::of(&g);
        let before = g.player.pos;
        g.player.pos.x += 400.;
        g.camera += 1.;
        let now = pose.show(&mut g, 0.5);
        assert_eq!(
            g.player.pos,
            before + vec2(400., 0.),
            "teleports aren't blended"
        );
        assert_eq!(g.camera, pose.camera.x + 0.5, "small moves are");
        now.restore(&mut g);
        assert_eq!(g.player.pos, before + vec2(400., 0.));
    }

    #[test]
    fn bolts_fired_or_ended_this_step_are_drawn_back_along_their_flight() {
        let mut g = practice(true);
        let pose = Pose::of(&g);
        g.shots.push(Shot {
            pos: vec2(300., 200.),
            vel: vec2(240., 0.),
            life: 1.,
            damage: 1.,
            hostile: true,
            kind: 0,
            from: None,
        });
        let now = pose.show(&mut g, 0.25);
        let drawn = g.shots[0].pos.x;
        assert!((drawn - (300. - 240. * world::STEP * 0.75)).abs() < 1e-3);
        now.restore(&mut g);
        assert_eq!(g.shots[0].pos, vec2(300., 200.));
    }

    #[test]
    fn drawing_between_steps_never_changes_the_simulation() {
        // The same run, with and without blending every frame at 144 Hz,
        // among the opening's guardians. Particles and floating numbers are
        // compared after every frame, since they come and go.
        let run = |blend: bool| {
            let mut g = practice(false);
            let mut pose = Pose::of(&g);
            let mut accumulator = 0.;
            let mut effects = vec![];
            for frame in 0..(144 * 6) {
                accumulator += 1. / 144.;
                let input = Input {
                    attack: frame % 50 < 25,
                    jump: frame % 97 == 0,
                    ..running()
                };
                while accumulator >= world::STEP {
                    pose.record(&g);
                    g.tick(world::STEP, input);
                    accumulator -= world::STEP;
                }
                if blend && g.screen == Screen::Playing {
                    let now = pose.show(&mut g, leftover(accumulator));
                    now.restore(&mut g);
                }
                let texts: Vec<_> = g.texts.iter().map(|t| (t.pos, t.life)).collect();
                effects.push((g.particles.clone(), texts));
            }
            (g, effects)
        };
        let ((plain, plain_effects), (blended, blended_effects)) = (run(false), run(true));
        assert_eq!(Pose::of(&plain), Pose::of(&blended));
        assert!(plain_effects
            .iter()
            .any(|(p, t)| !p.is_empty() && !t.is_empty()));
        assert!(
            plain_effects == blended_effects,
            "particles and numbers differ"
        );
        assert!(
            plain.level.enemies.iter().any(|e| e.hp < e.max_hp),
            "guardians fought"
        );
        assert_eq!(plain.player.hp, blended.player.hp);
        assert_eq!(plain.run_time, blended.run_time);
    }

    /// Runs 0.4 s of frames at 144 Hz with the hero standing still and
    /// returns how far `probe` moved in each frame as drawn.
    fn drawn_steps(g: &mut Game, blend: bool, probe: impl Fn(&Game) -> Vec2) -> Vec<f32> {
        let mut pose = Pose::of(g);
        let mut accumulator = 0.;
        let mut last = probe(g);
        (0..58)
            .map(|_| {
                accumulator += 1. / 144.;
                while accumulator >= world::STEP {
                    pose.record(g);
                    g.tick(world::STEP, Input::default());
                    accumulator -= world::STEP;
                }
                let at = if blend {
                    let now = pose.show(g, leftover(accumulator));
                    let at = probe(g);
                    now.restore(g);
                    at
                } else {
                    probe(g)
                };
                let moved = at.distance(last);
                last = at;
                moved
            })
            .collect()
    }
    /// A practice game holding one long-lived, drifting smoke particle and
    /// one floating number.
    fn drifting() -> Game {
        let mut g = practice(true);
        let at = g.player.pos - vec2(0., 20.);
        particles::emit(
            &mut g.particles,
            &mut particles::VisualRng::new(9),
            crate::particles::Effect::Explosion,
            at,
            1.,
            world::FLOOR,
            crate::fidelity::Fidelity::High,
        );
        let longest = g
            .particles
            .iter()
            .filter(|p| p.kind == crate::particles::Kind::Smoke)
            .map(|p| p.life)
            .fold(0., f32::max);
        g.particles.retain(|p| p.life == longest);
        g.particles.truncate(1);
        assert!(longest > 0.6, "the smoke outlives the measurement");
        g.label(at, "12".into(), WHITE);
        g
    }

    #[test]
    fn particles_and_numbers_move_evenly_between_steps() {
        let particle = |g: &Game| g.particles[0].pos;
        let number = |g: &Game| g.texts[0].pos;
        // Each frame's movement against the one before (smoke slows down
        // gradually, so neighbouring frames should move almost the same).
        let unevenness = |moved: Vec<f32>| {
            moved[1..]
                .windows(2)
                .map(|w| (w[1] / w[0].max(1e-6) - 1.).abs())
                .fold(0., f32::max)
        };
        let blended = unevenness(drawn_steps(&mut drifting(), true, particle));
        assert!(blended < 0.1, "blended smoke strays by {blended}");
        let stepped = unevenness(drawn_steps(&mut drifting(), false, particle));
        assert!(stepped > 0.5, "whole-step smoke strays by only {stepped}");
        let blended = unevenness(drawn_steps(&mut drifting(), true, number));
        assert!(blended < 0.01, "a blended number strays by {blended}");
        let stepped = unevenness(drawn_steps(&mut drifting(), false, number));
        assert!(
            stepped > 0.5,
            "a whole-step number strays by only {stepped}"
        );
    }

    #[test]
    fn showing_and_restoring_leaves_particles_and_numbers_as_they_were() {
        let mut g = drifting();
        for _ in 0..10 {
            g.tick(world::STEP, Input::default());
        }
        let pose = Pose::of(&g);
        let (particles, number) = (g.particles.clone(), (g.texts[0].pos, g.texts[0].life));
        let now = pose.show(&mut g, 0.3);
        assert_ne!(g.particles, particles, "drawn back along their motion");
        assert!(g.texts[0].pos.y > number.0.y && g.texts[0].life > number.1);
        now.restore(&mut g);
        assert_eq!(g.particles, particles);
        assert_eq!((g.texts[0].pos, g.texts[0].life), number);
    }

    #[test]
    fn the_pacing_report_counts_uneven_frames() {
        let mut pacing = Pacing::default();
        for steps in [1, 1, 0, 2, 1, 1, 1, 1] {
            pacing.push(1. / 120., steps);
        }
        let report = pacing.report();
        assert!(
            report.contains("steps_per_frame(0=1 1=6 2=1 3+=0)"),
            "{report}"
        );
        assert!(
            report.contains("uneven_without_blending=2 (25.0%)"),
            "{report}"
        );
    }
}
