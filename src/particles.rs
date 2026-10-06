//! Event-driven effects. The visual random stream is independent of loot and world generation.
use crate::game::Game;
use macroquad::prelude::*;

pub const MAX_PARTICLES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Spark,
    Shard,
    Dust,
    Smoke,
    Mote,
    Ring,
    Flash,
}

#[derive(Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max: f32,
    pub color: Color,
    pub size: f32,
    pub kind: Kind,
    drag: f32,
    gravity: f32,
    floor: f32,
    angle: f32,
    spin: f32,
    bounces: u8,
}

/// This seed never touches Game::rng, so quality settings and effect counts cannot affect loot.
pub struct VisualRng(u64);
impl VisualRng {
    pub fn new(seed: u64) -> Self {
        Self((seed ^ 0xd1b5_4a32_d192_ed03).max(1))
    }
    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / 16_777_216.
    }
    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + self.unit() * (max - min)
    }
}

#[derive(Clone, Copy)]
pub enum Effect {
    Hit,
    Death,
    Hurt,
    Jump,
    AirJump,
    Land(f32),
    Step,
    Dodge,
    DodgeTrail,
    ParryReady,
    Parry,
    Slam,
    Explosion,
    Heal,
    HealCast,
    Loot,
    Trap,
    Bolt,
    Projectile { hostile: bool, grenade: bool },
    Burn,
}

fn push(particles: &mut Vec<Particle>, particle: Particle) {
    // Drop the oldest effect at saturation; every fresh combat cue still gets a chance to read.
    if particles.len() >= MAX_PARTICLES {
        particles.remove(0);
    }
    particles.push(particle);
}

struct Emitter<'a> {
    particles: &'a mut Vec<Particle>,
    rng: &'a mut VisualRng,
    pos: Vec2,
    dir: f32,
    floor: f32,
}
impl Emitter<'_> {
    #[allow(clippy::too_many_arguments)]
    fn cloud(
        &mut self,
        kind: Kind,
        count: usize,
        color: Color,
        speed: (f32, f32),
        life: (f32, f32),
        size: (f32, f32),
        upward: bool,
    ) {
        for _ in 0..count {
            let angle = if upward {
                self.rng.range(-2.95, -0.19)
            } else {
                self.rng.range(0., std::f32::consts::TAU)
            };
            let velocity = vec2(angle.cos(), angle.sin()) * self.rng.range(speed.0, speed.1)
                + vec2(self.dir * speed.1 * 0.22, 0.);
            let max = self.rng.range(life.0, life.1);
            let (gravity, drag) = match kind {
                Kind::Spark => (145., 0.6),
                Kind::Shard => (310., 0.45),
                Kind::Dust => (10., 5.),
                Kind::Smoke => (-14., 2.),
                Kind::Mote => (-12., 1.6),
                _ => (0., 0.),
            };
            let jitter = if matches!(kind, Kind::Smoke | Kind::Dust) {
                3.
            } else {
                1.
            };
            let pos = self.pos + vec2(self.rng.range(-jitter, jitter), self.rng.range(-jitter, 0.));
            let particle = Particle {
                pos,
                vel: velocity,
                life: max,
                max,
                color,
                size: self.rng.range(size.0, size.1),
                kind,
                drag,
                gravity,
                floor: self.floor,
                angle: self.rng.range(0., std::f32::consts::TAU),
                spin: self.rng.range(-7., 7.),
                bounces: 0,
            };
            push(self.particles, particle);
        }
    }
    fn shape(&mut self, kind: Kind, color: Color, radius: f32, life: f32) {
        push(
            self.particles,
            Particle {
                pos: self.pos,
                vel: Vec2::ZERO,
                life,
                max: life,
                color,
                size: radius,
                kind,
                drag: 0.,
                gravity: 0.,
                floor: self.floor,
                angle: 0.,
                spin: 0.,
                bounces: 0,
            },
        );
    }
    fn sparks(&mut self, count: usize, color: Color, speed: f32) {
        self.cloud(
            Kind::Spark,
            count,
            color,
            (speed * 0.25, speed),
            (0.16, 0.43),
            (0.7, 1.5),
            false,
        );
    }
    fn dust(&mut self, count: usize, speed: f32) {
        self.cloud(
            Kind::Dust,
            count,
            Color::from_hex(0x778d99),
            (speed * 0.3, speed),
            (0.22, 0.52),
            (1.4, 3.4),
            true,
        );
    }
}

pub fn emit(
    particles: &mut Vec<Particle>,
    rng: &mut VisualRng,
    effect: Effect,
    pos: Vec2,
    dir: f32,
    floor: f32,
) {
    let mut e = Emitter {
        particles,
        rng,
        pos,
        dir,
        floor,
    };
    let amber = Color::from_hex(0xffba67);
    let cream = Color::from_hex(0xffedbb);
    let teal = Color::from_hex(0x8bf6e1);
    match effect {
        Effect::Hit | Effect::Death => {
            let dead = matches!(effect, Effect::Death);
            e.sparks(
                if dead { 20 } else { 10 },
                amber,
                if dead { 205. } else { 160. },
            );
            e.cloud(
                Kind::Shard,
                if dead { 7 } else { 3 },
                Color::from_hex(0xad8370),
                (30., 130.),
                (0.42, 0.8),
                (1., 2.3),
                true,
            );
            e.shape(Kind::Flash, cream, if dead { 16. } else { 10. }, 0.09);
            e.shape(Kind::Ring, amber, if dead { 26. } else { 15. }, 0.18);
            if dead {
                e.cloud(
                    Kind::Smoke,
                    5,
                    Color::from_hex(0x66717c),
                    (5., 28.),
                    (0.45, 0.8),
                    (3., 5.),
                    true,
                );
            }
        }
        Effect::Hurt => {
            e.sparks(13, Color::from_hex(0xfa735b), 145.);
            e.shape(Kind::Flash, Color::from_hex(0xffc0a8), 12., 0.08);
        }
        Effect::Jump => e.dust(7, 42.),
        Effect::AirJump => {
            e.cloud(
                Kind::Mote,
                9,
                teal,
                (12., 48.),
                (0.2, 0.4),
                (0.7, 1.2),
                false,
            );
            e.shape(Kind::Ring, teal, 15., 0.24);
        }
        Effect::Land(speed) => e.dust(
            (speed / 42.).clamp(5., 12.) as usize,
            (speed * 0.18).clamp(28., 85.),
        ),
        Effect::Step => e.dust(2, 24.),
        Effect::Dodge => {
            e.dust(8, 70.);
            e.cloud(
                Kind::Mote,
                6,
                teal,
                (15., 60.),
                (0.16, 0.3),
                (0.7, 1.2),
                false,
            );
        }
        Effect::DodgeTrail => {
            e.cloud(
                Kind::Mote,
                2,
                teal,
                (4., 20.),
                (0.1, 0.23),
                (0.6, 1.),
                false,
            );
            e.dust(1, 20.);
        }
        Effect::ParryReady => e.shape(Kind::Ring, teal, 10., 0.18),
        Effect::Parry => {
            e.sparks(22, teal, 235.);
            e.shape(Kind::Flash, Color::from_hex(0xe5ffff), 23., 0.12);
            e.shape(Kind::Ring, teal, 40., 0.28);
        }
        Effect::Slam => {
            e.sparks(20, teal, 190.);
            e.cloud(
                Kind::Shard,
                9,
                Color::from_hex(0x75949c),
                (60., 185.),
                (0.4, 0.9),
                (1., 2.5),
                true,
            );
            e.dust(16, 100.);
            e.shape(Kind::Ring, teal, 69., 0.34);
            e.shape(Kind::Flash, Color::from_hex(0xd1fff2), 22., 0.12);
        }
        Effect::Explosion => {
            e.cloud(
                Kind::Smoke,
                10,
                Color::from_hex(0x68707c),
                (15., 58.),
                (0.6, 1.15),
                (4., 8.),
                true,
            );
            e.cloud(
                Kind::Shard,
                10,
                Color::from_hex(0xbf7950),
                (70., 210.),
                (0.4, 0.95),
                (1., 2.8),
                true,
            );
            e.sparks(33, amber, 265.);
            e.shape(Kind::Ring, Color::from_hex(0xffa75b), 88., 0.38);
            e.shape(Kind::Flash, cream, 32., 0.13);
        }
        Effect::Heal => {
            e.cloud(
                Kind::Mote,
                25,
                teal,
                (15., 85.),
                (0.38, 0.8),
                (0.7, 1.8),
                true,
            );
            e.shape(Kind::Ring, teal, 30., 0.42);
            e.shape(Kind::Flash, Color::from_hex(0xcaffec), 12., 0.14);
        }
        Effect::HealCast => {
            e.cloud(
                Kind::Mote,
                2,
                teal,
                (7., 22.),
                (0.25, 0.48),
                (0.5, 1.2),
                true,
            );
        }
        Effect::Loot => {
            e.cloud(
                Kind::Mote,
                20,
                teal,
                (18., 82.),
                (0.36, 0.85),
                (0.8, 1.6),
                true,
            );
            e.shape(Kind::Ring, teal, 25., 0.3);
        }
        Effect::Trap => {
            e.sparks(12, Color::from_hex(0xb8cfff), 85.);
            e.shape(Kind::Ring, Color::from_hex(0x87ade4), 25., 0.3);
        }
        Effect::Bolt => {
            e.sparks(5, teal, 65.);
            e.shape(Kind::Flash, Color::from_hex(0xcaffec), 6., 0.07);
        }
        Effect::Projectile { hostile, grenade } => {
            if grenade {
                e.cloud(
                    Kind::Smoke,
                    1,
                    Color::from_hex(0x847665),
                    (3., 9.),
                    (0.18, 0.35),
                    (1., 2.2),
                    true,
                );
                e.cloud(
                    Kind::Mote,
                    1,
                    amber,
                    (3., 9.),
                    (0.12, 0.22),
                    (0.5, 0.9),
                    true,
                );
            } else {
                let color = if hostile {
                    Color::from_hex(0xfb9878)
                } else {
                    teal
                };
                e.cloud(
                    Kind::Mote,
                    1,
                    color,
                    (2., 8.),
                    (0.12, 0.24),
                    (0.6, 1.1),
                    false,
                );
            }
        }
        Effect::Burn => {
            e.cloud(
                Kind::Mote,
                1,
                amber,
                (8., 19.),
                (0.2, 0.4),
                (0.7, 1.3),
                true,
            );
        }
    }
}

pub fn update(particles: &mut Vec<Particle>, dt: f32) {
    for p in particles.iter_mut() {
        p.life -= dt;
        p.pos += p.vel * dt;
        p.vel *= (-p.drag * dt).exp();
        p.vel.y += p.gravity * dt;
        p.angle += p.spin * dt;
        if p.kind == Kind::Shard && p.pos.y >= p.floor && p.vel.y > 0. {
            p.pos.y = p.floor;
            p.vel.y *= -0.32;
            p.vel.x *= 0.64;
            p.spin *= 0.65;
            p.bounces += 1;
            if p.bounces >= 2 {
                p.vel = Vec2::ZERO;
                p.gravity = 0.;
                p.spin = 0.;
            }
        }
    }
    particles.retain(|p| p.life > 0.);
}

fn tinted(mut color: Color, alpha: f32) -> Color {
    color.a *= alpha;
    color
}

/// All positions land on the half-world-pixel grid used by the native 1280x720 target.
pub fn draw(game: &Game, camera: f32) {
    let flash = game.settings.flash_scale();
    // Smoke sits underneath hot fragments, independent of emitter insertion order.
    for soft in [true, false] {
        for p in &game.particles {
            if matches!(p.kind, Kind::Smoke | Kind::Dust) != soft
                || p.pos.x < camera - 100.
                || p.pos.x > camera + 740.
            {
                continue;
            }
            let pos = vec2(
                ((p.pos.x - camera) * 2.).round() * 0.5,
                (p.pos.y * 2.).round() * 0.5,
            );
            let remaining = (p.life / p.max).clamp(0., 1.);
            let elapsed = 1. - remaining;
            match p.kind {
                Kind::Spark => {
                    let tail = p.vel.normalize_or_zero()
                        * (p.vel.length() * 0.028).clamp(1., 7.5)
                        * remaining;
                    draw_line(
                        pos.x - tail.x,
                        pos.y - tail.y,
                        pos.x,
                        pos.y,
                        p.size.max(0.6),
                        tinted(p.color, remaining),
                    );
                    draw_rectangle(
                        pos.x - 0.5,
                        pos.y - 0.5,
                        1.,
                        1.,
                        tinted(Color::from_hex(0xfff1cf), remaining),
                    );
                }
                Kind::Shard => {
                    let arm = vec2(p.angle.cos(), p.angle.sin()) * p.size;
                    let cross = vec2(-arm.y, arm.x) * 0.55;
                    draw_triangle(
                        pos + arm,
                        pos - arm + cross,
                        pos - arm - cross,
                        tinted(p.color, remaining.sqrt()),
                    );
                }
                Kind::Dust | Kind::Smoke => {
                    let size = p.size * (1. + elapsed * 1.6);
                    let alpha = remaining * if p.kind == Kind::Smoke { 0.22 } else { 0.29 };
                    draw_poly(
                        pos.x,
                        pos.y,
                        6,
                        size,
                        p.angle.to_degrees(),
                        tinted(p.color, alpha),
                    );
                    draw_poly(
                        pos.x - size * 0.35,
                        pos.y - size * 0.3,
                        5,
                        size * 0.72,
                        0.,
                        tinted(p.color, alpha * 0.4),
                    );
                }
                Kind::Mote => {
                    let size = (p.size * remaining.sqrt()).max(0.5);
                    draw_rectangle(
                        pos.x - size,
                        pos.y - size,
                        size * 2.,
                        size * 2.,
                        tinted(p.color, remaining),
                    );
                }
                Kind::Ring => {
                    let radius = p.size * (1. - remaining * remaining).max(0.05);
                    let color = tinted(p.color, remaining * remaining * 0.7 * flash);
                    let flatten = p.pos.y > p.floor - 8.;
                    for segment in 0..24 {
                        let a = segment as f32 / 24. * std::f32::consts::TAU;
                        let b = (segment + 1) as f32 / 24. * std::f32::consts::TAU;
                        let y_scale = if flatten { 0.3 } else { 1. };
                        draw_line(
                            pos.x + a.cos() * radius,
                            pos.y + a.sin() * radius * y_scale,
                            pos.x + b.cos() * radius,
                            pos.y + b.sin() * radius * y_scale,
                            0.8,
                            color,
                        );
                    }
                }
                Kind::Flash => {
                    let radius = p.size * remaining;
                    let color = tinted(p.color, remaining * 0.9 * flash);
                    draw_triangle(
                        pos + vec2(-radius, 0.),
                        pos + vec2(0., -radius * 0.22),
                        pos + vec2(radius, 0.),
                        color,
                    );
                    draw_triangle(
                        pos + vec2(-radius, 0.),
                        pos + vec2(0., radius * 0.22),
                        pos + vec2(radius, 0.),
                        color,
                    );
                    draw_line(
                        pos.x,
                        pos.y - radius * 0.6,
                        pos.x,
                        pos.y + radius * 0.6,
                        1.,
                        color,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::FLOOR;
    #[test]
    fn overload_is_bounded_and_all_effects_expire() {
        let mut particles = Vec::new();
        let mut rng = VisualRng::new(42);
        for _ in 0..50 {
            emit(
                &mut particles,
                &mut rng,
                Effect::Explosion,
                vec2(100., FLOOR - 3.),
                1.,
                FLOOR,
            );
        }
        assert_eq!(particles.len(), MAX_PARTICLES);
        for _ in 0..240 {
            update(&mut particles, 1. / 120.);
        }
        assert!(particles.is_empty());
    }
    #[test]
    fn visual_seed_is_repeatable_and_shards_never_tunnel_below_floor() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        emit(
            &mut a,
            &mut VisualRng::new(7),
            Effect::Explosion,
            vec2(100., FLOOR - 4.),
            1.,
            FLOOR,
        );
        emit(
            &mut b,
            &mut VisualRng::new(7),
            Effect::Explosion,
            vec2(100., FLOOR - 4.),
            1.,
            FLOOR,
        );
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.vel, b.vel);
            assert_eq!(a.max, b.max);
        }
        for _ in 0..120 {
            update(&mut a, 1. / 120.);
            assert!(a
                .iter()
                .filter(|p| p.kind == Kind::Shard)
                .all(|p| p.pos.y <= p.floor));
        }
    }
}
