mod animation;
mod art;
mod atlas;
mod audio;
mod environment;
mod game;
mod particles;
mod postprocess;
mod render;
mod save;
mod scenery;
mod settings;
mod storage;
mod traversal_capture;
mod ui;
mod ui_skin;
mod world;
use game::*;
use macroquad::prelude::*;
fn conf() -> Conf {
    Conf {
        window_title: "Cinderwake — A Clockwork Roguelite".into(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}
fn read_input() -> Input {
    Input {
        axis: if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            1.
        } else {
            0.
        } - if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            1.
        } else {
            0.
        },
        jump: is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::W)
            || is_key_pressed(KeyCode::Up),
        jump_held: is_key_down(KeyCode::Space)
            || is_key_down(KeyCode::W)
            || is_key_down(KeyCode::Up),
        dodge: is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift),
        attack: is_key_down(KeyCode::J) || is_mouse_button_down(MouseButton::Left),
        bow: is_key_down(KeyCode::K),
        parry: is_key_pressed(KeyCode::L) || is_mouse_button_pressed(MouseButton::Right),
        grenade: is_key_pressed(KeyCode::Q),
        trap: is_key_pressed(KeyCode::R),
        heal: is_key_pressed(KeyCode::F),
        interact: is_key_pressed(KeyCode::E),
        down: is_key_down(KeyCode::S) || is_key_down(KeyCode::Down),
    }
}
fn menus(g: &mut Game) {
    if is_key_pressed(KeyCode::M) {
        g.settings.muted = !g.settings.muted;
        g.persist_settings();
    }
    if g.screen == Screen::Options {
        options_menu(g);
        return;
    }
    if is_key_pressed(KeyCode::Tab) && g.screen == Screen::Playing {
        g.map = !g.map;
    }
    if matches!(g.screen, Screen::Title | Screen::Paused) && is_key_pressed(KeyCode::O) {
        g.open_options();
        return;
    }
    if g.screen == Screen::Paused && is_key_pressed(KeyCode::X) {
        g.request_abandon();
        return;
    }
    if is_key_pressed(KeyCode::Escape) {
        g.abandon_armed = false;
        g.screen = if g.screen == Screen::Playing {
            Screen::Paused
        } else if g.screen == Screen::Paused {
            Screen::Playing
        } else {
            g.screen
        };
    }
    match g.screen {
        Screen::Title | Screen::Dead | Screen::Victory => {
            if is_key_pressed(KeyCode::Enter) {
                g.start();
            }
        }
        Screen::Scroll => {
            for (i, k) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
                .iter()
                .enumerate()
            {
                if is_key_pressed(*k) {
                    g.upgrade(i);
                }
            }
        }
        Screen::Camp => {
            for (i, k) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3]
                .iter()
                .enumerate()
            {
                if is_key_pressed(*k) {
                    g.buy(i);
                }
            }
            if is_key_pressed(KeyCode::A) || is_key_pressed(KeyCode::Left) {
                g.route = 0;
            }
            if is_key_pressed(KeyCode::D) || is_key_pressed(KeyCode::Right) {
                g.route = 1;
            }
            if is_key_pressed(KeyCode::Enter) {
                g.travel();
            }
        }
        _ => {}
    }
}

fn options_menu(g: &mut Game) {
    let pressed = |keys: &[KeyCode]| keys.iter().any(|k| is_key_pressed(*k));
    if pressed(&[KeyCode::Escape, KeyCode::O]) {
        g.close_options();
        return;
    }
    let rows = settings::Settings::ROWS;
    if pressed(&[KeyCode::W, KeyCode::Up]) {
        g.options_row = (g.options_row + rows - 1) % rows;
    }
    if pressed(&[KeyCode::S, KeyCode::Down]) {
        g.options_row = (g.options_row + 1) % rows;
    }
    let delta = if pressed(&[KeyCode::A, KeyCode::Left]) {
        -1
    } else if pressed(&[KeyCode::D, KeyCode::Right, KeyCode::Enter, KeyCode::Space]) {
        1
    } else {
        0
    };
    if delta != 0 {
        g.settings.adjust(g.options_row, delta);
    }
}

/// Presentation-only camera shake, scaled by the player's comfort setting.
fn camera_shake(g: &Game) -> Vec2 {
    if g.shake <= 0. {
        return Vec2::ZERO;
    }
    vec2((g.time * 93.).sin(), (g.time * 79.).cos()) * g.shake * 0.35 * g.settings.shake_scale()
}

const UI_GALLERY_NAMES: [&str; 14] = [
    "ui-00-title",
    "ui-01-playing",
    "ui-02-low-health-cooldowns-hammer",
    "ui-03-map",
    "ui-04-paused",
    "ui-05-scroll",
    "ui-06-camp-conservatory",
    "ui-07-camp-foundry",
    "ui-08-dead",
    "ui-09-victory",
    "ui-10-crown-boss",
    "ui-11-options",
    "ui-12-paused-abandon",
    "ui-13-tip",
];

// These are frozen visual fixtures for inspecting the interface, not a playthrough.
// Rebuild each fixture from a fixed seed without reading or writing player saves.
fn ui_fixture(index: usize) -> Game {
    let save = save::Save {
        embers: 74,
        vitality: 2,
        flask: 1,
        runs: 4,
        best_kills: 38,
        ..Default::default()
    };
    let mut g = Game::new(4017, save);
    g.practice = true;
    g.screen = Screen::Playing;
    g.player.pos = vec2(870., world::FLOOR);
    g.player.ground = true;
    g.player.hp = 104.;
    g.player.gold = 360;
    g.player.embers = 18;
    g.player.kills = 12;
    g.player.power = [3, 2, 1];
    g.player.tier = 2;
    g.camera = 640.;
    g.time = 4.;
    g.run_time = 327.;
    g.intro = 0.;
    g.settings.muted = true;
    match index {
        0 => g.screen = Screen::Title,
        1 => {}
        2 => {
            g.player.hp = 19.;
            g.player.weapon = Weapon::Hammer;
            g.player.attack_cd = 0.38;
            g.player.bow_cd = 0.22;
            g.player.grenade_cd = 3.4;
            g.player.trap_cd = 5.7;
            g.player.parry_cd = 0.4;
            g.player.dodge_cd = 0.5;
            g.player.flasks = 0;
            g.player.mutation = 2;
        }
        3 => g.map = true,
        4 => g.screen = Screen::Paused,
        5 => g.screen = Screen::Scroll,
        6 | 7 => {
            g.screen = Screen::Camp;
            g.route = index - 6;
            if index == 7 {
                g.player.mutation = 1;
                g.save.flask = 3;
                g.notice = "The flask is fully reinforced.".into();
                g.notice_time = 4.;
            }
        }
        8 => {
            g.screen = Screen::Dead;
            g.player.hp = 0.;
        }
        9 => {
            g.screen = Screen::Victory;
            g.player.kills = 38;
            g.save.rune = true;
            g.save.wins = 1;
        }
        10 => {
            g.level = world::Level::generate(4017, world::Biome::Crown, 0);
            g.stage = 2;
            for enemy in &mut g.level.enemies {
                if enemy.kind == world::EnemyKind::Regent {
                    enemy.hp = enemy.max_hp * 0.58;
                }
            }
        }
        11 => {
            g.screen = Screen::Paused;
            g.open_options();
            g.options_row = 2;
            g.settings.shake = 4;
            g.settings.reduce_flashes = true;
        }
        12 => {
            g.screen = Screen::Paused;
            g.request_abandon();
        }
        13 => g.hint = Some((Hint::Parry, HINT_SECONDS)),
        _ => unreachable!("UI gallery fixture index exceeds its capture list"),
    }
    g
}

const TOUR_FRAMES_PER_BIOME: u32 = 360;
const TOUR_FRAMES: u32 = TOUR_FRAMES_PER_BIOME * 4;
const MOTION_FRAMES: u32 = 900;

// The tour holds gameplay still while moving the camera through the complete
// level, making depth motion and scenery changes independently inspectable.
fn tour_position(frame: u32) -> (world::Biome, f32) {
    let biomes = [
        world::Biome::Aqueduct,
        world::Biome::Garden,
        world::Biome::Foundry,
        world::Biome::Crown,
    ];
    let index = (frame / TOUR_FRAMES_PER_BIOME).min(3) as usize;
    let progress = (frame % TOUR_FRAMES_PER_BIOME) as f32 / (TOUR_FRAMES_PER_BIOME - 1) as f32;
    (biomes[index], progress)
}

fn prepare_tour_frame(g: &mut Game, frame: u32) {
    let (biome, progress) = tour_position(frame);
    if frame.is_multiple_of(TOUR_FRAMES_PER_BIOME) {
        g.level = world::Level::generate(4017, biome, 0);
        g.level.enemies.clear();
        g.player = Player::new(&save::Save::default());
        g.screen = Screen::Playing;
        g.stage = match biome {
            world::Biome::Aqueduct => 0,
            world::Biome::Garden | world::Biome::Foundry => 1,
            world::Biome::Crown => 2,
        };
        g.intro = 0.;
        g.notice_time = 0.;
    }
    g.camera = (g.level.width - 640.).max(0.) * progress;
    g.player.pos = vec2((g.camera + 230.).min(g.level.width - 20.), world::FLOOR);
    g.player.vel = vec2(145., 0.);
    g.player.ground = true;
    g.time = frame as f32 / 60.;
    g.run_time = g.time;
}

// Scripted inputs still pass through the normal physics and combat simulation.
// The opening heal is safe and observable before the player enters combat.
fn motion_input(frame: u32) -> Input {
    Input {
        axis: if (160..300).contains(&frame) || frame >= 570 {
            1.
        } else {
            0.
        },
        heal: frame == 60,
        parry: frame == 130 || frame == 720,
        jump: matches!(frame, 200 | 215 | 600 | 620 | 810),
        jump_held: (200..245).contains(&frame)
            || (600..650).contains(&frame)
            || (810..835).contains(&frame),
        down: (245..259).contains(&frame),
        dodge: matches!(frame, 260 | 675 | 840),
        attack: (300..380).contains(&frame) || (570..900).contains(&frame) && frame % 90 < 40,
        bow: (405..465).contains(&frame) || (750..780).contains(&frame),
        grenade: frame == 490,
        trap: frame == 525,
        ..Default::default()
    }
}

/// Fits the 16:9 frame inside the window. Drawing rectangles use logical
/// (DPI-independent) units, but camera viewports address physical framebuffer
/// pixels, so the HUD viewport must be scaled or it shrinks into a corner on
/// fractional and Retina displays.
fn letterbox(screen_w: f32, screen_h: f32, dpi: f32) -> (Rect, (i32, i32, i32, i32)) {
    let scale = (screen_w / 1280.).min(screen_h / 720.);
    let (w, h) = (1280. * scale, 720. * scale);
    let (x, y) = ((screen_w - w) / 2., (screen_h - h) / 2.);
    let px = |v: f32| (v * dpi).round() as i32;
    (Rect::new(x, y, w, h), (px(x), px(y), px(w), px(h)))
}

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let capture = args.iter().any(|s| s == "--capture");
    let gallery = args.iter().any(|s| s == "--gallery");
    let sprite_preview = args.iter().any(|s| s == "--sprite-preview");
    let ui_gallery = args.iter().any(|s| s == "--ui-gallery");
    let environment_tour = args.iter().any(|s| s == "--environment-tour");
    let motion_capture = args.iter().any(|s| s == "--motion-capture");
    let vertical_capture = args.iter().any(|s| s == "--vertical-capture");
    let automated = capture || motion_capture || vertical_capture;
    let no_postfx = args.iter().any(|s| s == "--no-postfx");
    let profile_render = args.iter().any(|s| s == "--profile-render");
    let demo = args.iter().any(|s| s == "--demo") || automated;
    let staged = ui_gallery || gallery || environment_tour;
    let practice = demo || staged || sprite_preview;
    let seed = if practice {
        4017
    } else {
        // `std::time` is unavailable in the browser; miniquad's clock is portable.
        miniquad::date::now() as u64
    };
    let mut g = Game::new(
        seed,
        if practice {
            save::Save::default()
        } else {
            save::Save::load()
        },
    );
    g.practice = practice;
    if !practice {
        g.settings = settings::Settings::load();
        g.teach = true;
    }
    // A launch flag disables lighting for this session without saving the choice.
    if no_postfx {
        g.settings.postfx = false;
    }
    if practice {
        g.start();
    }
    if motion_capture {
        g.player.hp = g.player.max_hp * 0.45;
        g.intro = 0.;
        g.notice_time = 0.;
    }
    if vertical_capture {
        // Navigation proof: authored geometry and live input/physics, with combat
        // removed so the entire route can be inspected without a scripted fight.
        g.level.enemies.clear();
        g.level.hazards.clear();
        g.intro = 0.;
        g.notice_time = 0.;
    }
    let mut traversal = traversal_capture::Traversal::new();
    let mut arrival_hold = 0_u32;
    let ui = render::Ui::new();
    let mut audio = audio::Audio::new().await;
    let mut art = art::Art::new();
    let postfx = postprocess::PostProcess::new();
    let target = render_target(1280, 720);
    target.texture.set_filter(FilterMode::Nearest);
    let mut accumulator = 0.;
    let mut pending = Input::default();
    let mut frame = 0u32;
    let mut fullscreen = false;
    let mut previous_level = (g.level.seed, g.level.biome, g.seed);
    let mut arrival = 0.0_f32;
    let mut render_times = Vec::new();
    loop {
        let frame_started = miniquad::date::now();
        if ui_gallery {
            g = ui_fixture(frame as usize);
        } else if environment_tour {
            prepare_tour_frame(&mut g, frame);
        } else if gallery {
            let biome = [
                world::Biome::Aqueduct,
                world::Biome::Garden,
                world::Biome::Foundry,
                world::Biome::Crown,
            ][frame as usize];
            g.level = world::Level::generate(4017, biome, 0);
            g.player.pos = vec2(870., world::FLOOR);
            g.camera = 640.;
            g.time = 4.;
            g.intro = 0.;
        }
        if !staged && !automated {
            menus(&mut g);
        }
        if !staged && !automated && is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if !staged && !automated && is_key_pressed(KeyCode::F9) {
            g.settings.postfx = !g.settings.postfx;
            g.persist_settings();
            g.notify(if !postfx.available() {
                "Cinematic lighting unavailable on this graphics backend."
            } else if g.settings.postfx {
                "Cinematic lighting enabled."
            } else {
                "Cinematic lighting disabled."
            });
        }
        let mut input = if staged {
            Input::default()
        } else {
            read_input()
        };
        if vertical_capture && !staged {
            input = traversal.input(&g);
        } else if motion_capture && !staged {
            input = motion_input(frame);
        } else if demo && !staged {
            let t = frame as f32 / 60.;
            input = Input {
                axis: if t < 1.2 { 0. } else { 1. },
                attack: t > 2.,
                bow: t > 5.,
                jump: frame % 150 == 90,
                jump_held: frame % 150 >= 90 && frame % 150 < 115,
                dodge: frame.is_multiple_of(230) && frame > 0,
                grenade: frame % 310 == 160,
                trap: frame % 400 == 200,
                interact: true,
                ..Default::default()
            };
            if g.screen == Screen::Scroll {
                g.upgrade(0);
            }
            if g.screen == Screen::Camp {
                g.travel();
            }
            if g.screen == Screen::Dead {
                g.start();
            }
        }
        input.jump |= pending.jump;
        input.dodge |= pending.dodge;
        input.parry |= pending.parry;
        input.grenade |= pending.grenade;
        input.trap |= pending.trap;
        input.heal |= pending.heal;
        input.interact |= pending.interact;
        let paused = matches!(
            g.screen,
            Screen::Paused | Screen::Scroll | Screen::Camp | Screen::Options
        );
        let frame_dt = if demo || environment_tour {
            1. / 60.
        } else {
            get_frame_time().min(0.1)
        };
        if staged || paused {
            accumulator = 0.;
        }
        accumulator += if staged || paused { 0. } else { frame_dt };
        let mut animation_dt = 0.;
        while accumulator >= world::STEP {
            let animate_step =
                g.screen == Screen::Title || (g.screen == Screen::Playing && g.hitstop <= 0.);
            g.tick(world::STEP, input);
            if animate_step {
                animation_dt += world::STEP;
            }
            accumulator -= world::STEP;
            input.jump = false;
            input.dodge = false;
            input.parry = false;
            input.grenade = false;
            input.trap = false;
            input.heal = false;
            input.interact = false;
        }
        pending = if staged || paused {
            Input::default()
        } else {
            input
        };
        let silent = automated || staged || sprite_preview;
        audio.update(
            &mut g.sounds,
            if silent { 0. } else { g.settings.music_gain() },
            if silent {
                0.
            } else {
                g.settings.effects_gain()
            },
        );
        art.animate(
            &g,
            if environment_tour {
                frame_dt
            } else {
                animation_dt
            },
        );
        let current_level = (g.level.seed, g.level.biome, g.seed);
        if current_level != previous_level && !ui_gallery && !gallery {
            arrival = 0.45;
        }
        previous_level = current_level;
        if !paused {
            arrival = (arrival - frame_dt).max(0.);
        }
        let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., 640., 360.));
        camera.render_target = Some(target.clone());
        camera.target += camera_shake(&g);
        camera.target = camera.target.round();
        let mut backdrop_camera = Camera2D::from_display_rect(Rect::new(0., 0., 640., 360.));
        backdrop_camera.render_target = Some(target.clone());
        backdrop_camera.target = camera.target;
        camera.target.y += (g.camera_y * 2.).round() / 2.;
        let camera_offset = camera.target - vec2(320., 180.);
        if sprite_preview && !ui_gallery {
            set_camera(&backdrop_camera);
            clear_background(render::INK);
            art.preview(g.time);
            draw_text(
                "CINDERWAKE / IMAGEGEN ANIMATION ATLAS",
                46.,
                26.,
                16.,
                Color::from_hex(0xf2d7a2),
            );
        } else {
            set_camera(&backdrop_camera);
            art.environment.background(
                g.level.biome,
                (g.camera * 2.).round() / 2.,
                g.level.width,
                g.time,
                g.camera_y,
            );
            set_camera(&camera);
            render::scene(&g, &art);
        }
        let use_postfx = g.settings.postfx && !sprite_preview;
        if use_postfx {
            postfx.prepare(&target.texture, &g, camera_offset);
        }
        set_default_camera();
        clear_background(render::INK);
        let (frame_rect, viewport) = letterbox(screen_width(), screen_height(), screen_dpi_scale());
        let Rect { x, y, w, h } = frame_rect;
        postfx.draw(&target.texture, frame_rect, use_postfx);
        if arrival > 0. && !sprite_preview {
            let amount = arrival / 0.45;
            draw_rectangle(x, y, w, h, render::INK.with_alpha(amount * amount));
        }
        let ui_cam = Camera2D {
            zoom: vec2(2. / 1280., 2. / 720.),
            viewport: Some(viewport),
            ..Camera2D::from_display_rect(Rect::new(0., 0., 1280., 720.))
        };
        set_camera(&ui_cam);
        if !sprite_preview || ui_gallery {
            ui.draw(&g);
        }
        if environment_tour && !ui_gallery {
            draw_text(
                "STAGED ENVIRONMENT TOUR / CAMERA TRAVERSAL",
                20.,
                604.,
                15.,
                Color::from_hex(0xa1c6c4),
            );
        } else if vertical_capture && !staged {
            draw_text(
                "ROUTE TEST / LIVE MOVEMENT AND PHYSICS / COMBAT DISABLED",
                20.,
                604.,
                15.,
                Color::from_hex(0xa1c6c4),
            );
        } else if motion_capture && !staged {
            draw_text(
                "SCRIPTED INPUT CAPTURE / LIVE PHYSICS AND COMBAT",
                20.,
                604.,
                15.,
                Color::from_hex(0xa1c6c4),
            );
        }
        set_default_camera();
        if profile_render && frame >= 60 {
            // CPU simulation + draw submission only; capture readback and vsync
            // are excluded, so this is not presented as GPU frame rate.
            render_times.push((miniquad::date::now() - frame_started) * 1000.);
        }
        // Browsers have no writable `captures/` directory.
        if !staged
            && (is_key_pressed(KeyCode::F12) && cfg!(not(target_arch = "wasm32"))
                || (capture && frame.is_multiple_of(6))
                || (sprite_preview && frame == 120))
        {
            std::fs::create_dir_all("captures").ok();
            let path = if sprite_preview {
                "captures/animation-preview.png".into()
            } else {
                format!("captures/frame-{frame:05}.png")
            };
            get_screen_data().export_png(&path);
        }
        if (environment_tour || motion_capture || vertical_capture)
            && !ui_gallery
            && frame.is_multiple_of(3)
        {
            let directory = if environment_tour {
                "tour"
            } else if vertical_capture {
                "vertical"
            } else {
                "motion"
            };
            std::fs::create_dir_all(format!("captures/{directory}")).ok();
            get_screen_data().export_png(&format!("captures/{directory}/frame-{frame:05}.png"));
        }
        if ui_gallery {
            std::fs::create_dir_all("captures").ok();
            get_screen_data().export_png(&format!(
                "captures/{}.png",
                UI_GALLERY_NAMES[frame as usize]
            ));
            if frame as usize + 1 == UI_GALLERY_NAMES.len() {
                break;
            }
        }
        if !ui_gallery && sprite_preview && frame >= 180 {
            break;
        }
        if gallery && !ui_gallery {
            std::fs::create_dir_all("captures").ok();
            get_screen_data().export_png(&format!("captures/biome-{frame}.png"));
            if frame == 3 {
                break;
            }
        }
        if capture && !ui_gallery && frame >= 900 {
            break;
        }
        if (environment_tour && frame + 1 >= TOUR_FRAMES)
            || (motion_capture && !staged && frame + 1 >= MOTION_FRAMES)
        {
            break;
        }
        if vertical_capture {
            if traversal.finished(&g) {
                arrival_hold += 1;
            }
            if arrival_hold >= 60 || frame >= 5400 {
                eprintln!(
                    "Vertical route: {}/{} targets reached in {:.2}s; finished={}",
                    traversal.waypoint,
                    g.level.traversal.len(),
                    frame as f32 / 60.,
                    traversal.finished(&g)
                );
                break;
            }
        }
        frame += 1;
        next_frame().await
    }
    if !render_times.is_empty() {
        render_times.sort_by(f64::total_cmp);
        let mean = render_times.iter().sum::<f64>() / render_times.len() as f64;
        let p95 = render_times[(render_times.len() * 95 / 100).min(render_times.len() - 1)];
        eprintln!("CPU simulation/draw submission: mean {mean:.2} ms, p95 {p95:.2} ms ({} frames; excludes readback/vsync)", render_times.len());
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;

    #[test]
    fn shake_setting_scales_only_the_presented_camera() {
        let mut g = Game::new(4017, save::Save::default());
        g.time = 1.3;
        g.shake = 6.;
        let full = camera_shake(&g);
        assert!(full.length() > 0.);
        g.settings.shake = 5;
        assert!((camera_shake(&g) - full * 0.5).length() < 1e-4);
        g.settings.shake = 0;
        assert_eq!(camera_shake(&g), Vec2::ZERO);
        assert_eq!(g.shake, 6., "the simulation's shake timer is untouched");
    }

    #[test]
    fn hud_viewport_covers_the_physical_frame_at_any_dpi() {
        // A 1280 x 720 framebuffer at 1.25x reports a 1024 x 576 logical screen.
        let (rect, viewport) = letterbox(1024., 576., 1.25);
        assert_eq!(rect, Rect::new(0., 0., 1024., 576.));
        assert_eq!(viewport, (0, 0, 1280, 720));
        let (_, viewport) = letterbox(1280., 720., 1.);
        assert_eq!(viewport, (0, 0, 1280, 720));
        // Retina: pillarboxed logical window, doubled physical viewport.
        let (rect, viewport) = letterbox(1600., 720., 2.);
        assert_eq!(rect, Rect::new(160., 0., 1280., 720.));
        assert_eq!(viewport, (320, 0, 2560, 1440));
    }

    #[test]
    fn environment_tour_covers_full_width_of_every_biome() {
        let mut g = Game::new(4017, save::Save::default());
        g.practice = true;
        let expected = [
            world::Biome::Aqueduct,
            world::Biome::Garden,
            world::Biome::Foundry,
            world::Biome::Crown,
        ];
        for (index, biome) in expected.into_iter().enumerate() {
            let start = index as u32 * TOUR_FRAMES_PER_BIOME;
            prepare_tour_frame(&mut g, start);
            assert_eq!(g.level.biome, biome);
            assert_eq!(g.camera, 0.);
            assert!(g.level.enemies.is_empty());
            prepare_tour_frame(&mut g, start + TOUR_FRAMES_PER_BIOME - 1);
            assert_eq!(g.camera, g.level.width - 640.);
            assert!(g.player.pos.x > g.camera && g.player.pos.x < g.camera + 640.);
            assert!(g.practice);
        }
    }

    #[test]
    fn motion_capture_inputs_reach_all_requested_actions() {
        let mut g = Game::new(4017, save::Save::default());
        g.practice = true;
        g.start();
        g.player.hp = g.player.max_hp * 0.45;
        let mut seen = [false; 11];
        for frame in 0..MOTION_FRAMES {
            let mut input = motion_input(frame);
            for _ in 0..2 {
                let was_slamming = g.player.slam;
                g.tick(world::STEP, input);
                seen[0] |= g.player.heal_time > 0.;
                seen[1] |= g.player.parry > 0.;
                seen[2] |= g.player.dodge > 0.;
                seen[3] |= g.player.attack > 0.;
                seen[4] |= g.player.bow_cd > 0.;
                seen[5] |= g.player.jumps >= 2;
                seen[6] |= g.player.grenade_cd > 0.;
                seen[7] |= g.player.trap_cd > 0.;
                seen[8] |= g.player.ground && g.player.vel.x.abs() > 18.;
                seen[9] |= g.player.slam && !g.player.ground;
                seen[10] |= was_slamming && !g.player.slam && g.player.ground;
                input.jump = false;
                input.dodge = false;
                input.parry = false;
                input.grenade = false;
                input.trap = false;
                input.heal = false;
            }
        }
        assert!(
            seen.into_iter().all(|action| action),
            "missing actions: {seen:?}"
        );
        assert_eq!(g.screen, Screen::Playing, "capture must remain in gameplay");
        assert_eq!(g.player.flasks, 1, "opening heal must complete");
    }
}
