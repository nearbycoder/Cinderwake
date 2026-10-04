//! Deterministic authored-room assembly. Gameplay never depends on rendering.
use macroquad::prelude::*;

pub const FLOOR: f32 = 286.;
pub const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Biome {
    Aqueduct,
    Garden,
    Foundry,
    Crown,
}
impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Self::Aqueduct => "THE DROWNED AQUEDUCT",
            Self::Garden => "GLASSROOT CONSERVATORY",
            Self::Foundry => "THE EMBER FOUNDRY",
            Self::Crown => "CROWN OF THE MACHINE",
        }
    }
    pub fn subtitle(self) -> &'static str {
        match self {
            Self::Aqueduct => "Below the city, something still breathes.",
            Self::Garden => "Even the flowers remember the fire.",
            Self::Foundry => "A thousand hearts. One dying furnace.",
            Self::Crown => "The last light belongs to no king.",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EnemyKind {
    Warden,
    Archer,
    Moth,
    Brute,
    Regent,
}
#[derive(Clone, Debug)]
pub struct Enemy {
    pub pos: Vec2,
    pub home: f32,
    pub home_y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub kind: EnemyKind,
    pub face: f32,
    pub timer: f32,
    pub windup: f32,
    pub flash: f32,
    pub stun: f32,
    pub burn: f32,
    pub phase: u32,
}
impl Enemy {
    pub fn new(x: f32, y: f32, kind: EnemyKind, difficulty: u32) -> Self {
        let hp = match kind {
            EnemyKind::Warden => 65.,
            EnemyKind::Archer => 45.,
            EnemyKind::Moth => 32.,
            EnemyKind::Brute => 155.,
            EnemyKind::Regent => 1050.,
        } * (1. + difficulty as f32 * 0.12);
        Self {
            pos: vec2(x, y),
            home: x,
            home_y: y,
            hp,
            max_hp: hp,
            kind,
            face: -1.,
            timer: 0.7,
            windup: 0.,
            flash: 0.,
            stun: 0.,
            burn: 0.,
            phase: 0,
        }
    }
    pub fn rect(&self) -> Rect {
        let (w, h) = match self.kind {
            EnemyKind::Moth => (22., 18.),
            EnemyKind::Brute => (24., 38.),
            EnemyKind::Regent => (42., 66.),
            _ => (16., 29.),
        };
        Rect::new(self.pos.x - w / 2., self.pos.y - h, w, h)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObjectKind {
    Chest,
    Scroll,
    Fountain,
    Forge,
    Lore,
    Exit,
    Secret,
}
#[derive(Clone, Debug)]
pub struct Object {
    pub pos: Vec2,
    pub kind: ObjectKind,
    pub used: bool,
}
#[derive(Clone, Debug)]
pub struct Level {
    pub biome: Biome,
    pub width: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub spawn: Vec2,
    /// Connected landing targets for a complete surface / gallery / undercroft route.
    pub traversal: Vec<Vec2>,
    pub platforms: Vec<Rect>,
    pub enemies: Vec<Enemy>,
    pub objects: Vec<Object>,
    pub hazards: Vec<Rect>,
    pub seed: u64,
}
#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.next() % (hi - lo)
    }
}
impl Level {
    pub const UPPER: f32 = FLOOR - 336.;
    pub const LOWER: f32 = FLOOR + 336.;

    /// The first landing under these feet, including a support already being stood on.
    pub fn support_at(&self, x: f32, y: f32) -> Option<Rect> {
        self.platforms
            .iter()
            .filter(|p| x >= p.x && x <= p.x + p.w && p.y >= y - 3.)
            .min_by(|a, b| a.y.total_cmp(&b.y).then_with(|| b.w.total_cmp(&a.w)))
            .copied()
    }

    pub fn tier_name(y: f32) -> &'static str {
        if y < FLOOR - 165. {
            "UPPER GALLERIES"
        } else if y > FLOOR + 165. {
            "UNDERCROFT"
        } else {
            "SURFACE WORKS"
        }
    }

    fn floor(&mut self, x: f32, y: f32, width: f32, depth: f32) {
        self.platforms.push(Rect::new(x, y, width, depth));
    }

    /// Seven overlapping ledges rise one normal jump each. Wide landings also let
    /// players reverse direction and descend a shaft without precision jumping.
    fn flight(&mut self, x: f32, from_y: f32, direction: f32, width: f32) -> Vec<Vec2> {
        (1..=7)
            .map(|step| {
                let left = x + (step - 1) as f32 * 65.;
                let top = from_y + direction * step as f32 * 48.;
                self.floor(left, top, width, 12.);
                vec2(left + width / 2., top)
            })
            .collect()
    }

    fn object(&mut self, x: f32, y: f32, kind: ObjectKind) {
        self.objects.push(Object {
            pos: vec2(x, y),
            kind,
            used: false,
        });
    }

    pub fn generate(seed: u64, biome: Biome, difficulty: u32) -> Self {
        let mut rng = Rng(seed.max(1));
        let boss = biome == Biome::Crown;
        let width = if boss { 1500. } else { 3600. };
        let mut l = Self {
            biome,
            width,
            min_y: -230.,
            max_y: 760.,
            spawn: vec2(92., FLOOR),
            traversal: vec![vec2(92., FLOOR)],
            platforms: vec![],
            enemies: vec![],
            objects: vec![],
            hazards: vec![],
            seed,
        };
        let stair_width = 100. + rng.range(0, 3) as f32 * 4.;
        if boss {
            // A climb above the city opens onto the Regent's arena. Its solid
            // combat floor remains level, with a lower approach and upper gallery.
            l.floor(0., FLOOR, 200., 36.);
            let ascent = l.flight(175., FLOOR, -1., stair_width);
            l.traversal.push(vec2(160., FLOOR));
            l.traversal.extend(ascent);
            l.floor(565., Self::UPPER, 250., 28.);
            l.traversal.push(vec2(750., Self::UPPER));
            let descent = l.flight(780., Self::UPPER, 1., stair_width);
            l.traversal.extend(descent);
            l.floor(720., FLOOR, 780., 54.);

            l.flight(130., FLOOR, 1., stair_width);
            l.floor(520., Self::LOWER, 150., 42.);
            let lower_return = l.flight(650., Self::LOWER, -1., stair_width);
            l.traversal.extend(lower_return.iter().rev().copied());
            l.traversal.push(vec2(620., Self::LOWER));
            l.traversal.extend(lower_return);
            l.traversal.push(vec2(1400., FLOOR));
            // A gallery perch is useful in combat, but never the boss's support.
            l.floor(1130., FLOOR - 96., 125., 12.);
            l.floor(1310., FLOOR - 48., 100., 12.);
            l.floor(860., FLOOR - 48., 100., 12.);
            l.enemies
                .push(Enemy::new(1130., FLOOR, EnemyKind::Regent, difficulty));
            l.enemies
                .push(Enemy::new(675., Self::UPPER, EnemyKind::Archer, difficulty));
            l.enemies
                .push(Enemy::new(580., Self::LOWER, EnemyKind::Warden, difficulty));
            l.object(100., FLOOR, ObjectKind::Lore);
            l.object(720., Self::UPPER, ObjectKind::Scroll);
            l.object(620., Self::LOWER, ObjectKind::Secret);
            l.object(765., FLOOR, ObjectKind::Fountain);
            l.object(1410., FLOOR, ObjectKind::Exit);
        } else {
            // The main route deliberately changes floors. There is no continuous
            // surface slab underneath it that could bypass the climb and descent.
            l.floor(0., FLOOR, 400., 40.);
            l.traversal.push(vec2(360., FLOOR));
            let ascent = l.flight(410., FLOOR, -1., stair_width);
            l.traversal.extend(ascent);
            l.floor(800., Self::UPPER, 470., 30.);
            l.traversal
                .extend([vec2(1080., Self::UPPER), vec2(1240., Self::UPPER)]);
            let descent = l.flight(1250., Self::UPPER, 1., stair_width);
            l.traversal.extend(descent);
            l.floor(1640., FLOOR, 460., 38.);
            l.traversal.extend([vec2(1880., FLOOR), vec2(2070., FLOOR)]);
            let descent = l.flight(2080., FLOOR, 1., stair_width);
            l.traversal.extend(descent);
            l.floor(2470., Self::LOWER, 460., 48.);
            l.traversal
                .extend([vec2(2700., Self::LOWER), vec2(2890., Self::LOWER)]);
            let ascent = l.flight(2870., Self::LOWER, -1., stair_width);
            l.traversal.extend(ascent);
            l.floor(3260., FLOOR, 340., 44.);
            l.traversal.push(vec2(3480., FLOOR));

            // The undercroft and upper gallery create two complete optional loops,
            // connected at both ends. Surface chambers sit between their shafts.
            l.flight(280., FLOOR, 1., stair_width);
            l.floor(670., Self::LOWER, 900., 44.);
            l.flight(1510., Self::LOWER, -1., stair_width);
            l.floor(550., FLOOR, 940., 30.);
            l.flight(1810., FLOOR, -1., stair_width);
            l.floor(2200., Self::UPPER, 820., 32.);
            l.flight(2840., Self::UPPER, 1., stair_width);

            // Room silhouettes reflect their biome. Seeded recesses and shelves
            // sit within jump range of actual routes, never isolated decoration.
            let inset = rng.range(0, 5) as f32 * 12.;
            match biome {
                Biome::Aqueduct => {
                    l.floor(870. + inset, Self::LOWER - 48., 180., 18.);
                    l.floor(1010. + inset, Self::LOWER - 96., 150., 18.);
                    l.floor(2590. + inset, Self::UPPER - 48., 160., 18.);
                }
                Biome::Garden => {
                    l.floor(930. + inset, Self::UPPER - 48., 190., 12.);
                    l.floor(2500. + inset, Self::LOWER - 48., 210., 12.);
                    l.floor(2670. + inset, Self::LOWER - 96., 130., 12.);
                }
                Biome::Foundry => {
                    l.floor(790. + inset, FLOOR - 48., 220., 24.);
                    l.floor(980. + inset, FLOOR - 96., 145., 24.);
                    l.floor(2450. + inset, Self::UPPER - 48., 240., 24.);
                }
                Biome::Crown => unreachable!(),
            }

            for (x, y) in [
                (955., Self::UPPER),
                (1190., Self::UPPER),
                (780., FLOOR),
                (1260., FLOOR),
                (1770., FLOOR),
                (1990., FLOOR),
                (800., Self::LOWER),
                (1110., Self::LOWER),
                (1450., Self::LOWER),
                (2550., Self::LOWER),
                (2800., Self::LOWER),
                (2440., Self::UPPER),
                (2780., Self::UPPER),
                (3390., FLOOR),
            ] {
                let kind = match rng.range(0, 4) {
                    0 => EnemyKind::Archer,
                    1 => EnemyKind::Brute,
                    _ => EnemyKind::Warden,
                };
                l.enemies.push(Enemy::new(x, y, kind, difficulty));
            }
            for (x, y) in [(1390., 55.), (2330., 382.), (3010., 155.)] {
                l.enemies
                    .push(Enemy::new(x, y, EnemyKind::Moth, difficulty));
            }
            l.hazards.extend([
                Rect::new(1370., Self::LOWER - 5., 38., 5.),
                Rect::new(2890., Self::UPPER - 5., 38., 5.),
            ]);
            for (x, y, kind) in [
                (190., FLOOR, ObjectKind::Lore),
                (1080., Self::UPPER, ObjectKind::Scroll),
                (920., FLOOR, ObjectKind::Chest),
                (1880., FLOOR, ObjectKind::Fountain),
                (1250., Self::LOWER, ObjectKind::Forge),
                (730., Self::LOWER, ObjectKind::Secret),
                (2600., Self::UPPER, ObjectKind::Scroll),
                (2700., Self::LOWER, ObjectKind::Chest),
                (3480., FLOOR, ObjectKind::Exit),
            ] {
                l.object(x, y, kind);
            }
        }
        l
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    // Deliberately conservative jump envelope: one full held jump rises 52.9px
    // under gravity850 and travels at most102px at145px/s. A double jump is not
    // needed to prove route connectivity. Horizontal edge margins reserve8px.
    fn can_travel(from: Rect, to: Rect) -> bool {
        let rise = from.y - to.y;
        let gap = (to.x + 8. - (from.x + from.w - 8.))
            .max(from.x + 8. - (to.x + to.w - 8.))
            .max(0.);
        if rise > 50. {
            return false;
        }
        if rise.abs() < 0.1 && gap < 1. {
            return true;
        }
        let discriminant = 300. * 300. - 2. * 850. * rise;
        if discriminant < 0. {
            return false;
        }
        let flight_time = (300. + discriminant.sqrt()) / 850.;
        // Reserve a little acceleration and landing margin; a fall can also
        // descend vertically through a ledge using the actual drop-through input.
        gap <= (145. * flight_time - 16.).max(0.)
    }

    fn reachable(level: &Level, origin: usize) -> Vec<bool> {
        let mut reached = vec![false; level.platforms.len()];
        reached[origin] = true;
        let mut queue = vec![origin];
        while let Some(index) = queue.pop() {
            for (next, &platform) in level.platforms.iter().enumerate() {
                if !reached[next] && can_travel(level.platforms[index], platform) {
                    reached[next] = true;
                    queue.push(next);
                }
            }
        }
        reached
    }

    #[test]
    fn every_tier_and_exit_are_reachable_and_returnable_across_seeds() {
        for seed in 1..100 {
            for biome in [Biome::Aqueduct, Biome::Garden, Biome::Foundry, Biome::Crown] {
                let level = Level::generate(seed, biome, 0);
                let origin = level
                    .platforms
                    .iter()
                    .position(|p| {
                        level.spawn.x >= p.x
                            && level.spawn.x <= p.x + p.w
                            && (level.spawn.y - p.y).abs() < 1.
                    })
                    .unwrap();
                let visited = reachable(&level, origin);
                assert!(
                    visited.iter().all(|v| *v),
                    "unreachable platform in {biome:?}, seed{seed}"
                );
                // Reverse graph reachability catches one-way pits that a forward
                // traversal-only check would accept as a connected route.
                for index in 0..level.platforms.len() {
                    assert!(
                        reachable(&level, index)[origin],
                        "no return from platform{index} in {biome:?}"
                    );
                }
                assert!(level.platforms.iter().all(|p| p.w < level.width));
                assert!(level.platforms.iter().any(|p| p.y == Level::UPPER));
                assert!(level.platforms.iter().any(|p| p.y == Level::LOWER));
            }
        }
    }

    #[test]
    fn actors_rewards_and_traversal_have_real_safe_supports() {
        for seed in 1..30 {
            for biome in [Biome::Aqueduct, Biome::Garden, Biome::Foundry, Biome::Crown] {
                let level = Level::generate(seed, biome, 2);
                let positions = level
                    .objects
                    .iter()
                    .map(|o| o.pos)
                    .chain(
                        level
                            .enemies
                            .iter()
                            .filter(|e| e.kind != EnemyKind::Moth)
                            .map(|e| e.pos),
                    )
                    .chain(level.traversal.iter().copied());
                for pos in positions {
                    let support = level.support_at(pos.x, pos.y).expect("landing exists");
                    assert!(
                        (support.y - pos.y).abs() < 1.,
                        "unsupported actor/target{pos:?} in {biome:?}"
                    );
                    assert!(
                        pos.x > support.x + 6. && pos.x < support.x + support.w - 6.,
                        "edge actor/target {pos:?} on {support:?} in {biome:?}"
                    );
                    assert!(pos.y >= level.min_y && pos.y < level.max_y);
                }
                assert!(level.objects.iter().any(|o| o.kind == ObjectKind::Exit));
                assert!(level.objects.iter().all(|o| !level.hazards.iter().any(|h| {
                    o.pos.x >= h.x - 20.
                        && o.pos.x <= h.x + h.w + 20.
                        && (o.pos.y - (h.y + h.h)).abs() < 8.
                })));
            }
        }
    }

    #[test]
    fn generation_is_reproducible_and_seeds_change_rooms() {
        let a = Level::generate(1, Biome::Garden, 0);
        let b = Level::generate(1, Biome::Garden, 0);
        assert_eq!(a.platforms, b.platforms);
        assert_eq!(a.traversal, b.traversal);
        assert_ne!(a.platforms, Level::generate(2, Biome::Garden, 0).platforms);
        assert_ne!(a.platforms, Level::generate(1, Biome::Foundry, 0).platforms);
    }

    #[test]
    fn support_query_chooses_nearest_real_landing_below_feet() {
        let level = Level::generate(1, Biome::Aqueduct, 0);
        assert_eq!(level.support_at(92., 280.).unwrap().y, FLOOR);
        assert!(level.support_at(92., FLOOR + 4.).is_none());
        assert_eq!(
            level.support_at(1100., FLOOR + 4.).unwrap().y,
            Level::LOWER - 96.
        );
        assert_eq!(Level::tier_name(Level::UPPER), "UPPER GALLERIES");
        assert_eq!(Level::tier_name(FLOOR), "SURFACE WORKS");
        assert_eq!(Level::tier_name(Level::LOWER), "UNDERCROFT");
    }
}
