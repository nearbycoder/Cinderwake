//! A normal-input route follower for the reproducible vertical traversal capture.
//! It observes the same position/support state as a player and never moves actors,
//! rewrites physics, or skips a landing. The capture setup only removes combatants.
use crate::game::{Game, Input};

pub struct Traversal {
    pub waypoint: usize,
}

impl Traversal {
    pub fn new() -> Self {
        Self { waypoint: 0 }
    }

    pub fn finished(&self, g: &Game) -> bool {
        self.waypoint >= g.level.traversal.len() && g.player.ground
    }

    pub fn input(&mut self, g: &Game) -> Input {
        let p = &g.player;
        while let Some(target) = g.level.traversal.get(self.waypoint) {
            if p.ground && (target.y - p.pos.y).abs() < 1. && (target.x - p.pos.x).abs() < 9. {
                self.waypoint += 1;
            } else {
                break;
            }
        }
        let Some(target) = g.level.traversal.get(self.waypoint) else {
            return Input::default();
        };
        // Horizontal acceleration is exponential (22/s). Coast before the target
        // so overlapping ledges do not turn into abrupt left/right oscillation.
        let dx = target.x - p.pos.x;
        let stopping = p.vel.x / 22.;
        let axis = if (dx - stopping).abs() < 5. {
            0.
        } else {
            (dx - stopping).signum()
        };
        let dy = target.y - p.pos.y;
        let mut input = Input {
            axis,
            jump_held: true,
            ..Input::default()
        };
        if p.ground && dy > 3. {
            // Down+Space ignores the present one-way top. Release Down in the
            // air to preserve a gentle descent rather than triggering a slam.
            input.down = true;
            input.jump = true;
            input.jump_held = false;
        } else if p.ground && dy < -3. {
            if let Some(support) = g.level.support_at(target.x, target.y) {
                let gap = (support.x + 10. - p.pos.x)
                    .max(p.pos.x - (support.x + support.w - 10.))
                    .max(0.);
                // All authored stair rises are 48px. At full speed the landing
                // window begins within a normal held jump's horizontal reach.
                input.jump = gap <= 60.;
            }
        } else if !p.ground && dy < -6. && p.vel.y >= 0. && p.jumps < 2 {
            // A legitimate double jump can recover a late approach to a ledge;
            // it remains subject to the same input and collision rules as play.
            input.jump = true;
        }
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::Screen,
        save::Save,
        world::{Biome, Level, ObjectKind, STEP},
    };

    #[test]
    fn full_capture_route_visits_every_tier_and_reaches_exit_without_rescue() {
        for seed in [1, 4017, 98371] {
            for biome in [Biome::Aqueduct, Biome::Garden, Biome::Foundry, Biome::Crown] {
                let mut g = Game::new(seed, Save::default());
                g.level = Level::generate(seed, biome, 0);
                g.player.pos = g.level.spawn;
                g.level.enemies.clear();
                g.level.hazards.clear();
                g.screen = Screen::Playing;
                g.practice = true;
                let mut controller = Traversal::new();
                let mut tiers = [false; 3];
                let mut previous_waypoint = 0;
                let mut stationary_frames = 0;
                let mut elapsed_frames = 0;
                for frame in 0..90 * 60 {
                    elapsed_frames = frame + 1;
                    let input = controller.input(&g);
                    g.tick(STEP, input);
                    g.tick(
                        STEP,
                        Input {
                            jump: false,
                            ..input
                        },
                    );
                    if g.player.ground {
                        let tier = if g.player.pos.y < 0. {
                            0
                        } else if g.player.pos.y > 600. {
                            2
                        } else {
                            1
                        };
                        tiers[tier] = true;
                    }
                    assert!(
                        !g.notice.contains("last safe ledge"),
                        "rescue in {biome:?}, seed{seed}, waypoint{} frame{frame}",
                        controller.waypoint
                    );
                    assert_eq!(g.screen, Screen::Playing);
                    if controller.waypoint == previous_waypoint {
                        stationary_frames += 1;
                    } else {
                        stationary_frames = 0;
                        previous_waypoint = controller.waypoint;
                    }
                    assert!(stationary_frames < 600, "stalled in {biome:?}, seed{seed}, waypoint{} target{:?} pos{:?} vel{:?} ground{}", controller.waypoint, g.level.traversal.get(controller.waypoint), g.player.pos, g.player.vel, g.player.ground);
                    if controller.finished(&g) {
                        break;
                    }
                }
                assert!(
                    controller.finished(&g),
                    "unfinished in {biome:?}, seed{seed}, waypoint{} pos{:?}",
                    controller.waypoint,
                    g.player.pos
                );
                println!(
                    "{biome:?} seed {seed}: {:.2}s, {} landings",
                    elapsed_frames as f32 / 60.,
                    controller.waypoint
                );
                assert!(
                    tiers.iter().all(|visited| *visited),
                    "missed tier in {biome:?}: {tiers:?}"
                );
                let exit = g
                    .level
                    .objects
                    .iter()
                    .find(|o| o.kind == ObjectKind::Exit)
                    .unwrap()
                    .pos;
                assert!((g.player.pos.x - exit.x).abs() < 25.);
                assert!((g.player.pos.y - exit.y).abs() < 1.);
            }
        }
    }
}
