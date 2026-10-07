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
    // Warnings go over every guardian so a crowd can't hide one.
    for e in &g.level.enemies {
        if e.hp > 0. && e.windup > 0. {
            warning(e, e.pos.x - cam, g.player.pos - vec2(cam, 0.), t);
        }
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

/// What a guardian's windup will do, for drawing its warning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Threatens {
    /// A strike landing anywhere between these world x positions.
    Strike { left: f32, right: f32 },
    /// An archer's bolt toward the hero.
    Bolt,
    /// The Regent's fan of five bolts.
    Volley,
}
pub fn threatens(e: &Enemy) -> Threatens {
    match e.kind {
        EnemyKind::Archer => Threatens::Bolt,
        // Every third Regent attack is a volley; `phase` counts them.
        EnemyKind::Regent if (e.phase + 1).is_multiple_of(3) => Threatens::Volley,
        kind => {
            let reach = kind.strike_reach().x;
            Threatens::Strike {
                left: e.pos.x - reach,
                right: e.pos.x + reach,
            }
        }
    }
}

/// A windup's warning colour, brightening from gold to red as it nears.
pub fn warning_colour(progress: f32) -> Color {
    Color::from_vec(c(0xffd36e).to_vec().lerp(c(0xff5a45).to_vec(), progress))
}
/// A windup's warning: a mark over the guardian that brightens to red as
/// the attack nears, and where the attack will land. Timing is unchanged;
/// this only makes the existing tell readable.
fn warning(e: &Enemy, x: f32, hero: Vec2, t: f32) {
    let y = e.pos.y;
    let progress = (1. - e.windup / e.kind.windup()).clamp(0., 1.);
    let col = warning_colour(progress);
    match threatens(e) {
        // Moths fly, so a mark on the ground would mislead; theirs is the mark alone.
        Threatens::Strike { left, right } if e.kind != EnemyKind::Moth => {
            let (left, right) = (left - e.pos.x + x, right - e.pos.x + x);
            // A line along the ground, on a dark edge so it reads on any
            // stone, with posts at the ends of the reach.
            let a = 0.45 + 0.5 * progress;
            r(left, y, right - left, 1., alpha(c(0x100e22), 0.5));
            r(left, y - 1., right - left, 1., alpha(col, a * 0.8));
            for edge in [left, right - 1.] {
                r(edge, y - 5., 1., 5., alpha(col, a));
            }
        }
        Threatens::Bolt => {
            let from = vec2(x, y - 20.);
            let to = hero - vec2(0., 15.);
            let span = (to - from).length().min(320.);
            let dir = (to - from).normalize_or_zero();
            let mut d = 6.;
            while d < span {
                let p = from + dir * d;
                r(p.x, p.y, 2., 1., alpha(col, 0.35 + 0.5 * progress));
                d += 6.;
            }
        }
        Threatens::Volley => {
            let from = vec2(x, y - 34.);
            for n in -2..=2 {
                let to = from + vec2(e.face * 85., n as f32 * 24.);
                line(from.x, from.y, to.x, to.y, alpha(col, 0.2 + 0.4 * progress));
            }
        }
        _ => {}
    }
    // The mark: a small plaque with an exclamation, lifting as it pulses.
    let bob = ((t * 18.).sin() * 0.5 + 0.5).round();
    let top = y - e.rect().h - 22. - bob;
    glow(x, top + 5., 10. + 6. * progress, col);
    r(x - 4., top - 1., 8., 12., c(0x100e22));
    r(x - 3., top, 6., 10., col);
    r(x - 1., top + 1., 2., 5., c(0x100e22));
    r(x - 1., top + 7., 2., 2., c(0x100e22));
}

/// A threat the player can't see: a guardian winding up, or a hostile bolt
/// flying toward the hero, outside the part of the view the HUD leaves clear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marker {
    /// Where to draw it, in interface coordinates (1280 × 720).
    pub at: Vec2,
    /// Unit direction from the hero toward the threat.
    pub toward: Vec2,
    /// A windup's progress from 0 to 1, or `None` for a bolt.
    pub windup: Option<f32>,
}
/// The part of the 1280 × 720 interface between the top and bottom HUD
/// panels (and below the Regent's gauge while it lives).
pub fn play_area(boss: bool) -> Rect {
    let top = if boss { 166. } else { 100. };
    Rect::new(0., top, 1280., 634. - top)
}
/// How far inside the play area markers sit.
const MARKER_INSET: f32 = 26.;
/// Hostile bolts farther than this, in world units, aren't marked.
pub const BOLT_RANGE: f32 = 420.;
/// Markers closer than this share one, the windup's if there is one.
const MARKER_GAP: f32 = 24.;
/// Markers at the edge of the play area for each threat out of view, on the
/// line from the hero toward it. Windups come first, so a windup's marker
/// wins over a bolt's beside it.
pub fn threat_markers(g: &Game) -> Vec<Marker> {
    if g.screen != Screen::Playing || g.map {
        return vec![];
    }
    let boss = g
        .level
        .enemies
        .iter()
        .any(|e| e.kind == EnemyKind::Regent && e.hp > 0.);
    let area = play_area(boss);
    let to_ui = |p: Vec2| (p - vec2(g.camera, g.camera_y)) * 2.;
    let chest = g.player.pos - vec2(0., 14.);
    let hero = to_ui(chest);
    let windups = g
        .level
        .enemies
        .iter()
        .filter(|e| e.hp > 0. && e.windup > 0.)
        .map(|e| {
            let progress = (1. - e.windup / e.kind.windup()).clamp(0., 1.);
            (to_ui(e.rect().center()), Some(progress))
        });
    let bolts = g
        .shots
        .iter()
        .filter(|s| {
            let to_hero = chest - s.pos;
            s.hostile
                && to_hero.length() < BOLT_RANGE
                && s.vel.normalize_or_zero().dot(to_hero.normalize_or_zero()) > 0.7
        })
        .map(|s| (to_ui(s.pos), None));
    let inset = Rect::new(
        area.x + MARKER_INSET,
        area.y + MARKER_INSET,
        area.w - MARKER_INSET * 2.,
        area.h - MARKER_INSET * 2.,
    );
    let mut markers: Vec<Marker> = vec![];
    for (at, windup) in windups.chain(bolts) {
        if area.contains(at) {
            continue;
        }
        let marker = Marker {
            at: edge_point(inset, hero, at),
            toward: (at - hero).normalize_or_zero(),
            windup,
        };
        if markers
            .iter()
            .all(|m| m.at.distance(marker.at) >= MARKER_GAP)
        {
            markers.push(marker);
        }
    }
    markers
}
/// Where the line from `from` toward `to` leaves `rect`, kept inside it.
fn edge_point(rect: Rect, from: Vec2, to: Vec2) -> Vec2 {
    let d = to - from;
    let mut s = 1f32;
    for (d, from, low, high) in [
        (d.x, from.x, rect.x, rect.right()),
        (d.y, from.y, rect.y, rect.bottom()),
    ] {
        if d > 0. {
            s = s.min((high - from) / d);
        } else if d < 0. {
            s = s.min((low - from) / d);
        }
    }
    let p = from + d * s.max(0.);
    vec2(
        p.x.clamp(rect.x, rect.right()),
        p.y.clamp(rect.y, rect.bottom()),
    )
}

/// Vitality at or below this share of the maximum warns the player.
pub const LOW_VITALITY: f32 = 0.3;
/// How strongly to warn about low vitality in play (atlas included), from 0
/// above `LOW_VITALITY` through about a third at it, to 1 at none.
pub fn low_vitality(g: &Game) -> f32 {
    let p = &g.player;
    let share = p.hp / p.max_hp;
    if g.screen != Screen::Playing || p.hp <= 0. || share > LOW_VITALITY {
        return 0.;
    }
    0.35 + 0.65 * (1. - share / LOW_VITALITY)
}
/// The warning tint's opacity at the screen's edge: it pulses about once a
/// second, or holds steady when flashes are reduced.
pub fn low_vitality_tint(level: f32, t: f32, steady: bool) -> f32 {
    let pulse = if steady {
        0.75
    } else {
        0.55 + 0.45 * (t * std::f32::consts::TAU * 1.1).sin().abs()
    };
    0.5 * level * pulse
}

pub use crate::ui::Ui;

#[cfg(test)]
mod tests {
    use super::*;

    fn playing() -> Game {
        let mut g = Game::new(42, crate::save::Save::default());
        g.practice = true;
        g.screen = Screen::Playing;
        g.level.enemies.clear();
        g.place_player(vec2(900., FLOOR));
        g
    }
    /// A guardian partway through its windup, `at` world units from the camera's corner.
    fn winding(g: &mut Game, kind: EnemyKind, at: Vec2) {
        let mut e = Enemy::new(g.camera + at.x, g.camera_y + at.y, kind, Threat::BASE);
        e.windup = kind.windup() * 0.5;
        g.level.enemies.push(e);
    }
    fn bolt(g: &mut Game, at: Vec2, vel: Vec2) {
        g.shots.push(Shot {
            pos: vec2(g.camera, g.camera_y) + at,
            vel,
            life: 3.,
            damage: 10.,
            hostile: true,
            kind: 2,
            from: Some(EnemyKind::Archer),
        });
    }

    #[test]
    fn only_threats_out_of_view_get_markers_on_their_edge() {
        let mut g = playing();
        winding(&mut g, EnemyKind::Warden, vec2(300., 250.));
        assert!(threat_markers(&g).is_empty(), "a windup in view needs none");
        let area = play_area(false);
        let inset = 26.;
        // Above the view, below it, left, right, and above but behind the
        // top HUD panels, which the camera shows but the player can't see.
        for (at, edge) in [
            (vec2(230., -60.), "top"),
            (vec2(230., 420.), "bottom"),
            (vec2(-40., 200.), "left"),
            (vec2(700., 200.), "right"),
            (vec2(230., 40.), "top"),
        ] {
            let mut g = playing();
            winding(&mut g, EnemyKind::Archer, at);
            let markers = threat_markers(&g);
            assert_eq!(markers.len(), 1, "{at}");
            let m = markers[0];
            assert!(area.contains(m.at), "{at}: inside the play area");
            let expected = match edge {
                "top" => (m.at.y - (area.y + inset)).abs() < 0.01 && m.toward.y < 0.,
                "bottom" => (m.at.y - (area.bottom() - inset)).abs() < 0.01 && m.toward.y > 0.,
                "left" => (m.at.x - inset).abs() < 0.01 && m.toward.x < 0.,
                _ => (m.at.x - (1280. - inset)).abs() < 0.01 && m.toward.x > 0.,
            };
            assert!(expected, "{at} should be marked on the {edge} edge: {m:?}");
            assert_eq!(m.windup, Some(0.5));
        }
    }

    #[test]
    fn bolts_are_marked_only_while_arriving_from_near() {
        let hero = |g: &Game| g.player.pos - vec2(g.camera, g.camera_y);
        let mut g = playing();
        g.camera = g.player.pos.x - 320.;
        let from = vec2(650., hero(&g).y - 14.);
        bolt(&mut g, from, vec2(-170., 0.));
        let markers = threat_markers(&g);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].windup, None);
        assert!(markers[0].toward.x > 0.99, "it comes from the right");
        g.shots[0].vel = vec2(170., 0.);
        assert!(threat_markers(&g).is_empty(), "flying away");
        g.shots[0].vel = vec2(-170., 0.);
        g.shots[0].pos.x = g.player.pos.x + BOLT_RANGE + 10.;
        assert!(threat_markers(&g).is_empty(), "too far to matter yet");
        g.shots[0].pos.x = g.player.pos.x + 200.;
        g.shots[0].hostile = false;
        assert!(threat_markers(&g).is_empty(), "the hero's own bolt");
    }

    #[test]
    fn markers_show_only_in_play_and_merge_when_close() {
        let mut g = playing();
        winding(&mut g, EnemyKind::Archer, vec2(230., -60.));
        // A bolt just beside it shares its marker; a windup's wins.
        bolt(&mut g, vec2(232., -50.), vec2(0., 170.));
        let markers = threat_markers(&g);
        assert_eq!(markers.len(), 1);
        assert!(markers[0].windup.is_some());
        g.map = true;
        assert!(threat_markers(&g).is_empty(), "not over the atlas");
        g.map = false;
        g.screen = Screen::Paused;
        assert!(threat_markers(&g).is_empty(), "not on menus");
    }

    #[test]
    fn low_vitality_warns_below_thirty_percent_and_only_in_play() {
        let mut g = playing();
        let max = g.player.max_hp;
        g.player.hp = max * 0.31;
        assert_eq!(low_vitality(&g), 0.);
        g.player.hp = max * 0.3;
        assert!((low_vitality(&g) - 0.35).abs() < 1e-4);
        g.player.hp = max * 0.15;
        let half = low_vitality(&g);
        g.player.hp = max * 0.01;
        assert!(
            half > 0.35 && low_vitality(&g) > half,
            "stronger as it falls"
        );
        g.map = true;
        assert!(
            low_vitality(&g) > 0.,
            "the world keeps going behind the atlas"
        );
        for screen in [Screen::Paused, Screen::Dead, Screen::Options, Screen::Camp] {
            g.screen = screen;
            assert_eq!(low_vitality(&g), 0., "{screen:?}");
        }
        g.screen = Screen::Playing;
        g.player.hp = 0.;
        assert_eq!(low_vitality(&g), 0., "not once the run has ended");
        // It pulses, unless flashes are reduced; then it holds steady.
        let tints: Vec<f32> = (0..20)
            .map(|i| low_vitality_tint(1., i as f32 * 0.05, false))
            .collect();
        let (low, high) = tints
            .iter()
            .fold((1f32, 0f32), |(l, h), t| (l.min(*t), h.max(*t)));
        assert!(high - low > 0.1, "pulses: {low}..{high}");
        let steady: Vec<f32> = (0..20)
            .map(|i| low_vitality_tint(1., i as f32 * 0.05, true))
            .collect();
        assert!(steady.windows(2).all(|w| w[0] == w[1]));
        assert_eq!(low_vitality_tint(0., 0.3, false), 0.);
    }

    #[test]
    fn warnings_show_exactly_where_strikes_land() {
        for kind in [EnemyKind::Warden, EnemyKind::Brute, EnemyKind::Regent] {
            let e = Enemy::new(500., FLOOR, kind, Threat::BASE);
            let Threatens::Strike { left, right } = threatens(&e) else {
                panic!("{kind:?} strikes");
            };
            let reach = kind.strike_reach().x;
            assert_eq!((left, right), (500. - reach, 500. + reach), "{kind:?}");
        }
        let archer = Enemy::new(0., FLOOR, EnemyKind::Archer, Threat::BASE);
        assert_eq!(threatens(&archer), Threatens::Bolt);
        let mut regent = Enemy::new(0., FLOOR, EnemyKind::Regent, Threat::BASE);
        regent.phase = 2;
        assert_eq!(threatens(&regent), Threatens::Volley, "every third attack");
    }
}
