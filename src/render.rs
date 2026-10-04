use crate::{game::*, world::*};
use macroquad::prelude::*;
pub const INK: Color = Color::new(0.035, 0.06, 0.085, 1.);
pub fn c(h: u32) -> Color {
    Color::from_hex(h)
}
fn alpha(mut col: Color, a: f32) -> Color {
    col.a = a;
    col
}
fn r(x: f32, y: f32, w: f32, h: f32, col: Color) {
    draw_rectangle(x.floor(), y.floor(), w.ceil(), h.ceil(), col);
}
fn line(x: f32, y: f32, x2: f32, y2: f32, col: Color) {
    crate::art::pixel_line(vec2(x, y), vec2(x2, y2), 1., col);
}
fn glow(x: f32, y: f32, size: f32, col: Color) {
    // Three discrete light bands retain the pixel silhouette around emissive props.
    for (scale, opacity) in [(1., 0.025), (0.65, 0.035), (0.32, 0.06)] {
        let radius = (size * scale).floor();
        r(
            x - radius,
            y - radius * 0.6,
            radius * 2.,
            radius * 1.2,
            alpha(col, opacity),
        );
        r(
            x - radius * 0.6,
            y - radius,
            radius * 1.2,
            radius * 2.,
            alpha(col, opacity),
        );
    }
}
pub fn scene(g: &Game, art: &crate::art::Art) {
    let cam = (g.camera * 2.).round() / 2.;
    let t = (g.time * 24.).floor() / 24.;
    for p in &g.level.platforms {
        let x = p.x - cam;
        if x > 660. || x + p.w < -20. || p.y > g.camera_y + 410. || p.y + 100. < g.camera_y {
            continue;
        }
        art.environment.platform(g.level.biome, p, cam);
    }
    for h in &g.level.hazards {
        for i in 0..7 {
            let x = h.x - cam + i as f32 * 6.;
            draw_triangle(
                vec2(x, h.y + 5.),
                vec2(x + 3., h.y - 4.),
                vec2(x + 6., h.y + 5.),
                c(0xb58a76),
            );
        }
    }
    art.environment.dressing(&g.level, cam, g.camera_y, g.time);
    for o in &g.level.objects {
        art.environment.object(o, cam, t);
    }
    for tr in &g.traps {
        let x = tr.pos.x - cam;
        glow(x, tr.pos.y - 6., 90., c(0x7be5d8));
        art.environment.trap(x, tr.pos.y);
        draw_circle_lines(x, tr.pos.y - 6., 10. + (t * 7.).sin() * 3., 1., c(0x8af4d4));
    }
    for e in &g.level.enemies {
        if e.hp <= 0. {
            continue;
        }
        let x = e.pos.x - cam;
        if !(-80. ..720.).contains(&x) || e.pos.y < g.camera_y - 60. || e.pos.y > g.camera_y + 440.
        {
            continue;
        }
        enemy(art, e, x, t);
    }
    let p = &g.player;
    let x = p.pos.x - cam;
    let y = p.pos.y;
    let shadow_floor = g
        .level
        .platforms
        .iter()
        .filter(|platform| {
            p.pos.x >= platform.x && p.pos.x <= platform.x + platform.w && platform.y >= y - 0.5
        })
        .map(|platform| platform.y)
        .min_by(f32::total_cmp);
    if let Some(shadow_floor) = shadow_floor {
        let shadow_strength = (1. - (shadow_floor - y).max(0.) / 170.).clamp(0., 1.);
        r(
            x - 9.,
            shadow_floor,
            18.,
            2.,
            alpha(c(0x080f1b), 0.45 * shadow_strength),
        );
    }
    art.afterimages(cam);
    art.player(x, y, p.face, 1.);
    if p.attack > 0. {
        let center = vec2(x + p.face * 3., y - 17.);
        let reach = p.weapon.reach();
        let progress = (1. - p.attack / 0.19).clamp(0., 1.);
        let swing = if p.combo == 1 { -1. } else { 1. };
        let tip = (-1.5 + progress * 3.) * swing;
        let length = 1.2 * (progress * 8.).min(1.);
        let fade = ((1. - progress) * 3.5).min(1.);
        for j in 0..24 {
            let u = j as f32 / 24.;
            let a = tip - swing * length * (1. - u);
            let a2 = a + swing * length / 24.;
            let v1 = center + vec2(a.cos() * p.face, a.sin()) * reach;
            let v2 = center + vec2(a2.cos() * p.face, a2.sin()) * reach;
            let taper = (u * std::f32::consts::PI).sin();
            crate::art::pixel_line(
                v1,
                v2,
                1. + taper * 3.,
                alpha(c(0xe2733b), fade * (0.3 + u * 0.7)),
            );
            crate::art::pixel_line(
                v1 - vec2(a.cos() * p.face, a.sin()) * 2.,
                v2 - vec2(a2.cos() * p.face, a2.sin()) * 2.,
                0.5 + taper * 1.5,
                alpha(c(0xffe3a0), fade * u),
            );
            if j > 4 && j < 17 {
                crate::art::pixel_line(v1, v2, 0.5, alpha(c(0xfff9d8), fade * u));
            }
        }
    }
    if p.parry > 0. {
        draw_circle_lines(x + p.face * 14., y - 16., 18., 2., c(0xc4ffdf));
        glow(x + p.face * 14., y - 16., 25., c(0x69fadc));
    }
    if p.heal_time > 0. {
        glow(x, y - 15., 35., c(0x71f6c7));
        for i in 0..8 {
            let a = i as f32 * 0.78 + t * 4.;
            r(
                x + a.cos() * 16.,
                y - 16. + a.sin() * 20.,
                2.,
                3.,
                c(0xb1ffcd),
            );
        }
    }
    for s in &g.shots {
        let col = c(if s.hostile { 0xff8972 } else { 0x8bfff2 });
        let x = s.pos.x - cam;
        if s.kind == 1 {
            r(x - 3., s.pos.y - 2., 6., 4., c(0xc57c36));
            r(x - 2., s.pos.y - 3., 4., 6., c(0xe1a451));
            r(x - 1., s.pos.y - 2., 2., 2., c(0xffedb4));
            glow(x, s.pos.y, 16., c(0xffae5a));
        } else {
            line(
                x,
                s.pos.y,
                x - s.vel.x * 0.025,
                s.pos.y - s.vel.y * 0.025,
                col,
            );
            r(x, s.pos.y, 3., 2., WHITE);
        }
    }
    crate::particles::draw(g, cam);
    for tx in &g.texts {
        draw_text(
            &tx.text,
            tx.pos.x - cam - 8.,
            tx.pos.y,
            10.,
            alpha(tx.color, tx.life.min(1.)),
        );
    }
    // Atmosphere is now biome-specific and drawn behind readable combat cues.
    for i in 0..12 {
        r(
            0.,
            g.camera_y + i as f32 * 2.,
            640.,
            2.,
            alpha(INK, (12 - i) as f32 * 0.025),
        );
    }
}
fn enemy(art: &crate::art::Art, e: &Enemy, x: f32, t: f32) {
    let y = e.pos.y;
    r(x - 10., y - 1., 20., 2., alpha(c(0x090e21), 0.6));
    art.enemy(e, x, t);
    if e.windup > 0. {
        let yy = y - e.rect().h - 12.;
        r(x - 2., yy, 4., 6., c(0x100e22));
        r(x - 1., yy, 2., 4., c(0xffd777));
        r(x - 1., yy + 6., 2., 2., c(0xfff3b0));
    }
    if e.hp < e.max_hp {
        let w = if e.kind == EnemyKind::Regent {
            50.
        } else {
            24.
        };
        r(
            x - w / 2. - 1.,
            y - e.rect().h - 6.,
            w + 2.,
            4.,
            c(0x0c1025),
        );
        r(
            x - w / 2.,
            y - e.rect().h - 5.,
            w * e.hp / e.max_hp,
            2.,
            c(0xf18d58),
        );
    }
    if e.burn > 0. {
        for i in 0..3 {
            crate::scenery::flame(x - 7. + i as f32 * 6., y - 2., t + i as f32 * 0.2, false);
        }
    }
}

pub use crate::ui::Ui;
