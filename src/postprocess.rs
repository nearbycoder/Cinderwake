//! Low-resolution light diffusion is composited over the untouched nearest-
//! sampled scene. The HUD is drawn afterwards and never enters either shader.
use crate::{fidelity::Fidelity, game::Game, world::Biome};
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
                composites.push((
                    fidelity,
                    load_material(
                        ShaderSource::Glsl {
                            vertex: VERTEX,
                            fragment: &fragment,
                        },
                        MaterialParams {
                            textures: vec!["Bloom".into()],
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
                    )?,
                ));
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
        let size = fidelity.bloom_size();
        if !p.targets.iter().any(|(s, ..)| *s == size) {
            let make = || {
                let target = render_target(size.0, size.1);
                target.texture.set_filter(FilterMode::Linear);
                target
            };
            p.targets.push((size, make(), make()));
        }
        let (_, horizontal, vertical) = p.targets.iter().find(|(s, ..)| *s == size).unwrap();
        // Every destination differs from its sampled source. Two 9-tap passes
        // give a soft emissive halo without ever blurring the sprite texture.
        // Taps are spaced in the world's units whatever the targets' size, so
        // the halo keeps its reach at every step.
        for (source, target, direction, extract) in [
            (scene, horizontal, vec2(1. / 640., 0.), 1_f32),
            (&horizontal.texture, vertical, vec2(0., 1. / 360.), 0_f32),
        ] {
            let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 640., 360.));
            camera.render_target = Some(target.clone());
            set_camera(&camera);
            clear_background(BLACK);
            p.blur.set_uniform("Direction", direction);
            p.blur.set_uniform("Extract", extract);
            gl_use_material(&p.blur);
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
        composite.set_texture("Bloom", vertical.texture.clone());
        composite.set_uniform("Grade", grade(game.level.biome));
        let (lights, colors) = scene_lights(game, camera_offset, fidelity.lights());
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
    COMPOSITE.replace(
        "#define LIGHTS 8",
        &format!("#define LIGHTS {}", fidelity.lights()),
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
fn scene_lights(game: &Game, offset: Vec2, budget: usize) -> (Vec<Vec4>, Vec<Vec4>) {
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
        let (lights, _) = scene_lights(&g, vec2(2., -1.), 8);
        assert_eq!(lights[0].x, 208.);
        assert_eq!(lights[0].y, 266.);
        assert!(lights.iter().all(|l| l.is_finite()));
        assert_eq!(g.player.pos, vec2(510., 286.));
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
        let (lights, _) = scene_lights(&g, vec2(0., 374.), 8);
        assert_eq!(lights.iter().filter(|l| l.w > 0.).count(), 2);
        assert_eq!(lights[1].y, 243.);
    }
}
