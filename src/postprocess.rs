//! Low-resolution light diffusion is composited over the untouched nearest-
//! sampled scene. The HUD is drawn afterwards and never enters either shader.
use crate::{game::Game, world::Biome};
use macroquad::prelude::*;

const VERTEX: &str = include_str!("../assets/shaders/fullscreen.vert");
const BLOOM: &str = include_str!("../assets/shaders/bloom.frag");
const COMPOSITE: &str = include_str!("../assets/shaders/composite.frag");
const LIGHTS: usize = 8;

struct Pipeline {
    blur: Material,
    composite: Material,
    horizontal: RenderTarget,
    vertical: RenderTarget,
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
            let composite = load_material(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: COMPOSITE,
                },
                MaterialParams {
                    textures: vec!["Bloom".into()],
                    uniforms: vec![
                        UniformDesc::new("Grade", UniformType::Float4),
                        UniformDesc::array(UniformDesc::new("Lights", UniformType::Float4), LIGHTS),
                        UniformDesc::array(
                            UniformDesc::new("LightColors", UniformType::Float4),
                            LIGHTS,
                        ),
                    ],
                    ..Default::default()
                },
            )?;
            let horizontal = render_target(640, 360);
            let vertical = render_target(640, 360);
            horizontal.texture.set_filter(FilterMode::Linear);
            vertical.texture.set_filter(FilterMode::Linear);
            Ok(Pipeline {
                blur,
                composite,
                horizontal,
                vertical,
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

    pub fn prepare(&self, scene: &Texture2D, game: &Game, camera_offset: Vec2) {
        let Some(p) = &self.pipeline else { return };
        // Every destination differs from its sampled source. Two 9-tap passes
        // give a soft emissive halo without ever blurring the sprite texture.
        for (source, target, direction, extract) in [
            (scene, &p.horizontal, vec2(2. / 1280., 0.), 1_f32),
            (
                &p.horizontal.texture,
                &p.vertical,
                vec2(0., 1. / 360.),
                0_f32,
            ),
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
        p.composite.set_texture("Bloom", p.vertical.texture.clone());
        p.composite.set_uniform("Grade", grade(game.level.biome));
        let (lights, colors) = scene_lights(game, camera_offset);
        p.composite.set_uniform_array("Lights", &lights);
        p.composite.set_uniform_array("LightColors", &colors);
    }

    pub fn draw(&self, scene: &Texture2D, rect: Rect, enabled: bool) {
        if enabled {
            if let Some(p) = &self.pipeline {
                gl_use_material(&p.composite);
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

fn grade(biome: Biome) -> Vec4 {
    // Neutral enough to preserve the original copper/teal art palette.
    match biome {
        Biome::Aqueduct => vec4(0.96, 1.015, 1.045, 0.34),
        Biome::Garden => vec4(1.015, 1.025, 0.97, 0.31),
        Biome::Foundry => vec4(1.055, 0.99, 0.94, 0.30),
        Biome::Crown => vec4(1.015, 0.985, 1.055, 0.37),
    }
}

fn scene_lights(game: &Game, offset: Vec2) -> ([Vec4; LIGHTS], [Vec4; LIGHTS]) {
    let mut lights = [Vec4::ZERO; LIGHTS];
    let mut colors = [Vec4::ZERO; LIGHTS];
    let mut count = 0;
    let camera = (game.camera * 2.).round() / 2.;
    let mut add = |pos: Vec2, radius: f32, strength: f32, color: Vec3| {
        if count >= LIGHTS {
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
        let (lights, _) = scene_lights(&g, vec2(2., -1.));
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
            });
        }
        g.traps.push(crate::game::Trap {
            pos: vec2(180., 622.),
            life: 1.,
            tick: 0.,
        });
        let (lights, _) = scene_lights(&g, vec2(0., 374.));
        assert_eq!(lights.iter().filter(|l| l.w > 0.).count(), 2);
        assert_eq!(lights[1].y, 243.);
    }
}
