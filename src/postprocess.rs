//! Low-resolution light diffusion is composited over the untouched nearest-
//! sampled scene. The HUD is drawn afterwards and never enters either shader.
use crate::{
    fidelity::Fidelity,
    game::Game,
    world::{Biome, ObjectKind},
};
use macroquad::prelude::*;

const VERTEX: &str = include_str!("../assets/shaders/fullscreen.vert");
const BLOOM: &str = include_str!("../assets/shaders/bloom.frag");
const COMPOSITE: &str = include_str!("../assets/shaders/composite.frag");

struct Pipeline {
    blur: Material,
    /// One composite shader per post-processed step, each built for its
    /// light budget.
    composites: Vec<(Fidelity, Material)>,
    /// Bloom's horizontal and vertical targets, made for each size in use.
    targets: Vec<((u32, u32), RenderTarget, RenderTarget)>,
}

impl Pipeline {
    /// Blurs the scene's bright light into targets of `size`, reaching
    /// `reach` times the usual distance, and returns the result.
    fn bloom(&mut self, scene: &Texture2D, size: (u32, u32), reach: f32) -> Texture2D {
        if !self.targets.iter().any(|(s, ..)| *s == size) {
            let make = || {
                let target = render_target(size.0, size.1);
                target.texture.set_filter(FilterMode::Linear);
                target
            };
            self.targets.push((size, make(), make()));
        }
        let (_, horizontal, vertical) = self.targets.iter().find(|(s, ..)| *s == size).unwrap();
        // Every destination differs from its sampled source. Two 9-tap passes
        // give a soft emissive halo without ever blurring the sprite texture.
        // Taps are spaced in the world's units whatever the targets' size, so
        // the halo keeps its reach at every step.
        for (source, target, direction, extract) in [
            (scene, horizontal, vec2(reach / 640., 0.), 1_f32),
            (&horizontal.texture, vertical, vec2(0., reach / 360.), 0_f32),
        ] {
            let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 640., 360.));
            camera.render_target = Some(target.clone());
            set_camera(&camera);
            clear_background(BLACK);
            self.blur.set_uniform("Direction", direction);
            self.blur.set_uniform("Extract", extract);
            gl_use_material(&self.blur);
            draw_texture_ex(
                source,
                0.,
                0.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(640., 360.)),
                    flip_y: true,
                    ..Default::default()
                },
            );
            gl_use_default_material();
        }
        vertical.texture.clone()
    }
}

pub struct PostProcess {
    pipeline: Option<Pipeline>,
}

impl PostProcess {
    pub fn available(&self) -> bool {
        self.pipeline.is_some()
    }

    pub fn new() -> Self {
        let build = || -> Result<Pipeline, macroquad::Error> {
            let blur = load_material(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: BLOOM,
                },
                MaterialParams {
                    uniforms: vec![
                        UniformDesc::new("Direction", UniformType::Float2),
                        UniformDesc::new("Extract", UniformType::Float1),
                    ],
                    ..Default::default()
                },
            )?;
            let mut composites = vec![];
            for fidelity in Fidelity::ALL.into_iter().filter(|f| f.post()) {
                let lights = fidelity.lights();
                let fragment = composite_source(fidelity);
                let material = load_material(
                    ShaderSource::Glsl {
                        vertex: VERTEX,
                        fragment: &fragment,
                    },
                    MaterialParams {
                        textures: if fidelity.scenery_lights() {
                            vec!["Bloom".into(), "BloomWide".into()]
                        } else {
                            vec!["Bloom".into()]
                        },
                        uniforms: vec![
                            UniformDesc::new("Grade", UniformType::Float4),
                            UniformDesc::new("SceneSize", UniformType::Float2),
                            UniformDesc::new("Scale", UniformType::Float1),
                            UniformDesc::array(
                                UniformDesc::new("Lights", UniformType::Float4),
                                lights,
                            ),
                            UniformDesc::array(
                                UniformDesc::new("LightColors", UniformType::Float4),
                                lights,
                            ),
                        ],
                        ..Default::default()
                    },
                );
                // A step whose shader won't build draws unfiltered rather
                // than taking the other steps with it.
                match material {
                    Ok(material) => composites.push((fidelity, material)),
                    Err(error) => {
                        eprintln!("PostFX {} unavailable: {error}", fidelity.name())
                    }
                }
            }
            Ok(Pipeline {
                blur,
                composites,
                targets: vec![],
            })
        };
        let pipeline = match build() {
            Ok(pipeline) => {
                eprintln!("PostFX ready: selective bloom and cinematic lighting");
                Some(pipeline)
            }
            Err(error) => {
                eprintln!("PostFX unavailable, retaining unfiltered scene: {error}");
                None
            }
        };
        Self { pipeline }
    }

    pub fn prepare(&mut self, scene: &Texture2D, game: &Game, camera_offset: Vec2) {
        let Some(p) = &mut self.pipeline else { return };
        let fidelity = game.fidelity();
        let Some(composite) = p
            .composites
            .iter()
            .find(|(f, _)| *f == fidelity)
            .map(|(_, m)| m.clone())
        else {
            return;
        };
        let bloom = p.bloom(scene, fidelity.bloom_size(), 1.);
        if fidelity.scenery_lights() {
            // A second, wider halo at low resolution, under the first.
            let wide = p.bloom(scene, (320, 180), 3.);
            composite.set_texture("BloomWide", wide);
        }
        composite.set_texture("Bloom", bloom);
        composite.set_uniform("Grade", grade(game.level.biome));
        let (lights, colors) = scene_lights(
            game,
            camera_offset,
            fidelity.lights(),
            fidelity.scenery_lights(),
        );
        composite.set_uniform_array("Lights", &lights);
        composite.set_uniform_array("LightColors", &colors);
    }

    /// Draws the scene into `rect`, which covers `pixels` window pixels
    /// across.
    pub fn draw(&self, scene: &Texture2D, rect: Rect, pixels: f32, fidelity: Fidelity) {
        if fidelity.post() {
            if let Some((_, material)) = self
                .pipeline
                .iter()
                .flat_map(|p| &p.composites)
                .find(|(f, _)| *f == fidelity)
            {
                material.set_uniform("SceneSize", scene.size());
                material.set_uniform("Scale", pixels / scene.width());
                gl_use_material(material);
            }
        }
        draw_texture_ex(
            scene,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();
    }
}

/// The composite shader's source for one step: its light budget set.
fn composite_source(fidelity: Fidelity) -> String {
    COMPOSITE
        .replace(
            "#define LIGHTS 8",
            &format!("#define LIGHTS {}", fidelity.lights()),
        )
        .replace(
            "#define ULTRA 0",
            if fidelity.scenery_lights() {
                "#define ULTRA 1"
            } else {
                "#define ULTRA 0"
            },
        )
}

fn grade(biome: Biome) -> Vec4 {
    // Neutral enough to preserve the original copper/teal art palette.
    match biome {
        Biome::Aqueduct => vec4(0.96, 1.015, 1.045, 0.34),
        Biome::Garden => vec4(1.015, 1.025, 0.97, 0.31),
        Biome::Foundry => vec4(1.055, 0.99, 0.94, 0.30),
        Biome::Crown => vec4(1.015, 0.985, 1.055, 0.37),
    }
}

/// Up to `budget` lights in view, in order of importance, and their colours;
/// unused entries are zero.
/// A light cast by the scenery: where, how far, how strong, and what colour.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SceneryLight {
    pos: Vec2,
    radius: f32,
    strength: f32,
    color: Vec3,
}

/// Lights cast by the level's lamps, forges, wells, bellgate, inscriptions,
/// and memories, in world units, with a gentle flicker on flames unless
/// flashes are reduced. They come from the same placements the scenery is
/// drawn with.
fn scenery_lights(game: &Game) -> Vec<SceneryLight> {
    let steady = game.settings.reduce_flashes;
    let flicker = |seed: f32, depth: f32| {
        if steady {
            1.
        } else {
            1. - depth + depth * (game.time * 9. + seed * 4.).sin()
        }
    };
    let lamp_color = if game.level.biome == Biome::Foundry {
        vec3(1., 0.5, 0.18)
    } else {
        vec3(1., 0.72, 0.36)
    };
    let mut found: Vec<_> = crate::environment::lamps(&game.level)
        .map(|(i, pos)| SceneryLight {
            pos,
            radius: 66.,
            strength: 0.22 * flicker(i as f32, 0.12),
            color: lamp_color,
        })
        .collect();
    for (i, o) in game.level.objects.iter().enumerate() {
        let (lift, radius, strength, color) = match o.kind {
            ObjectKind::Forge => (13., 90., 0.3 * flicker(i as f32, 0.15), vec3(1., 0.55, 0.2)),
            ObjectKind::Fountain => (24., 76., 0.2, vec3(0.35, 0.95, 0.9)),
            ObjectKind::Exit => (46., 120., 0.24, vec3(0.55, 1., 0.9)),
            ObjectKind::Lore => (15., 50., 0.12, vec3(0.75, 0.85, 1.)),
            ObjectKind::Scroll if !o.used => (21., 50., 0.15, vec3(0.8, 0.62, 1.)),
            _ => continue,
        };
        found.push(SceneryLight {
            pos: o.pos - vec2(0., lift),
            radius,
            strength,
            color,
        });
    }
    found
}

/// Up to `budget` lights in view, in order of importance, and their colours;
/// unused entries are zero. Combat lights come first; with `scenery`, the
/// scenery's lights nearest the middle of the view fill what's left.
fn scene_lights(game: &Game, offset: Vec2, budget: usize, scenery: bool) -> (Vec<Vec4>, Vec<Vec4>) {
    let mut lights = vec![Vec4::ZERO; budget];
    let mut colors = vec![Vec4::ZERO; budget];
    let mut count = 0;
    let camera = (game.camera * 2.).round() / 2.;
    let mut add = |pos: Vec2, radius: f32, strength: f32, color: Vec3| {
        if count >= budget {
            return;
        }
        let pos = pos - vec2(camera, 0.) - offset;
        if pos.x + radius < 0.
            || pos.x - radius > 640.
            || pos.y + radius < 0.
            || pos.y - radius > 360.
        {
            return;
        }
        lights[count] = vec4(pos.x, pos.y, radius, strength);
        colors[count] = color.extend(0.);
        count += 1;
    };
    let hero = &game.player;
    let casting = hero.parry > 0. || hero.heal_time > 0. || hero.bow_cd > 0.18;
    add(
        hero.pos - vec2(0., 21.),
        55.,
        if casting { 0.28 } else { 0.075 },
        if casting {
            vec3(0.35, 1., 0.83)
        } else {
            vec3(0.6, 0.84, 0.92)
        },
    );
    if hero.attack > 0. {
        add(
            hero.pos + vec2(hero.face * 26., -20.),
            68.,
            0.22 * (hero.attack / 0.19) * game.settings.flash_scale(),
            vec3(1., 0.63, 0.22),
        );
    }
    for shot in &game.shots {
        add(
            shot.pos,
            if shot.kind == 1 { 53. } else { 32. },
            0.17,
            if shot.hostile || shot.kind == 1 {
                vec3(1., 0.38, 0.12)
            } else {
                vec3(0.2, 0.85, 1.)
            },
        );
    }
    for trap in &game.traps {
        add(trap.pos - vec2(0., 5.), 52., 0.16, vec3(0.25, 1., 0.76));
    }
    for enemy in &game.level.enemies {
        if enemy.hp > 0. && (enemy.flash > 0. || enemy.burn > 0.) {
            add(
                enemy.pos - vec2(0., 20.),
                42.,
                0.16 * game.settings.flash_scale(),
                vec3(1., 0.49, 0.14),
            );
        }
    }
    if scenery {
        let middle = vec2(camera + 320., 180.) + offset;
        let mut candidates = scenery_lights(game);
        candidates.sort_by(|a, b| {
            a.pos
                .distance_squared(middle)
                .total_cmp(&b.pos.distance_squared(middle))
        });
        for light in candidates {
            add(light.pos, light.radius, light.strength, light.color);
        }
    }
    (lights, colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::Save;
    #[test]
    fn lights_track_world_camera_and_shake_without_affecting_simulation() {
        let mut g = Game::new(7, Save::default());
        g.camera = 300.;
        g.player.pos = vec2(510., 286.);
        let (lights, _) = scene_lights(&g, vec2(2., -1.), 8, false);
        assert_eq!(lights[0].x, 208.);
        assert_eq!(lights[0].y, 266.);
        assert!(lights.iter().all(|l| l.is_finite()));
        assert_eq!(g.player.pos, vec2(510., 286.));
    }
    #[test]
    fn scenery_lights_fill_only_the_budget_combat_leaves_and_stay_in_view() {
        let mut g = Game::new(4017, Save::default());
        g.start();
        let foundry =
            crate::world::Level::generate(4017, Biome::Foundry, crate::world::Threat::BASE);
        g.level = foundry;
        g.camera = 600.;
        g.player.pos = vec2(900., crate::world::FLOOR);
        let offset = vec2(0., g.camera_y);
        let (without, _) = scene_lights(&g, offset, 16, false);
        let (with, colors) = scene_lights(&g, offset, 16, true);
        let lit = |l: &[Vec4]| l.iter().filter(|l| l.w > 0.).count();
        assert_eq!(lit(&without), 1, "only the hero's own light");
        assert!(lit(&with) > 2, "lamps light the Foundry: {}", lit(&with));
        assert_eq!(with[0], without[0], "combat lights keep their places");
        for (light, color) in with.iter().zip(&colors).filter(|(l, _)| l.w > 0.) {
            assert!(
                light.x + light.z >= 0. && light.x - light.z <= 640.,
                "{light:?}"
            );
            assert!(
                light.y + light.z >= 0. && light.y - light.z <= 360.,
                "{light:?}"
            );
            assert!(color.truncate().max_element() <= 1.);
        }
        let (tight, _) = scene_lights(&g, offset, 4, true);
        assert_eq!(tight.len(), 4, "never more than the budget");
        assert!(scenery_lights(&g).iter().all(|l| l.strength <= 0.35));
    }

    #[test]
    fn scenery_flicker_holds_steady_with_flashes_reduced() {
        let mut g = Game::new(4017, Save::default());
        g.level = crate::world::Level::generate(4017, Biome::Foundry, crate::world::Threat::BASE);
        g.settings.reduce_flashes = true;
        let at = |g: &mut Game, t: f32| {
            g.time = t;
            scenery_lights(g)
        };
        let first = at(&mut g, 0.1);
        assert!(!first.is_empty());
        assert_eq!(first, at(&mut g, 0.37));
        g.settings.reduce_flashes = false;
        assert_ne!(at(&mut g, 0.1), at(&mut g, 0.37));
    }

    #[test]
    fn offscreen_tiers_do_not_consume_visible_light_budget() {
        let mut g = Game::new(9, Save::default());
        g.player.pos = vec2(100., 622.);
        for _ in 0..12 {
            g.shots.push(crate::game::Shot {
                pos: vec2(200., -50.),
                vel: Vec2::ZERO,
                life: 1.,
                damage: 1.,
                hostile: false,
                kind: 0,
                from: None,
            });
        }
        g.traps.push(crate::game::Trap {
            pos: vec2(180., 622.),
            life: 1.,
            tick: 0.,
        });
        let (lights, _) = scene_lights(&g, vec2(0., 374.), 8, false);
        assert_eq!(lights.iter().filter(|l| l.w > 0.).count(), 2);
        assert_eq!(lights[1].y, 243.);
    }
}
