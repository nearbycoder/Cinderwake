//! Generated background plates and transparent modular scenery, composed in world
//! space. Surface anchors keep visual platform tops aligned with physics colliders.
use crate::world::{Biome, Level, Object, ObjectKind, FLOOR};
use macroquad::prelude::*;

/// The height of the dressing stood on the `i`th platform.
fn dressing_height(i: usize) -> f32 {
    if i.is_multiple_of(3) {
        38.
    } else {
        27.
    }
}
/// Whether a biome's platform dressing carries a lamp.
fn lit_dressing(biome: Biome) -> bool {
    matches!(biome, Biome::Aqueduct | Biome::Foundry)
}
/// Where each lamp on the platforms' dressing burns, in world units, with its
/// platform's index (for flicker), from the same placement `dressing` draws.
pub fn lamps(level: &Level) -> impl Iterator<Item = (usize, Vec2)> + '_ {
    level
        .platforms
        .iter()
        .enumerate()
        .filter(|(_, p)| p.w >= 140. && lit_dressing(level.biome))
        .map(|(i, p)| {
            let height = dressing_height(i);
            (i, vec2(p.x + 30., p.y - height * 0.52 - height * 0.1))
        })
}

/// Position within a biome, shared by panorama composition and the area label.
pub fn progress(cam: f32, level_width: f32) -> f32 {
    if level_width <= 640. {
        0.
    } else {
        (cam / (level_width - 640.)).clamp(0., 1.)
    }
}
fn smoothstep(value: f32) -> f32 {
    let t = value.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
pub fn zone_name(biome: Biome, cam: f32, level_width: f32) -> &'static str {
    let zone = ((progress(cam, level_width) * 3.) as usize).min(2);
    let names = match biome {
        Biome::Aqueduct => [
            "SUNKEN GALLERIES",
            "FLOODGATE CROSSING",
            "THE INNER PUMPWORKS",
        ],
        Biome::Garden => [
            "THE GLASS CLOISTER",
            "HEARTROOT GROVE",
            "THE MOONLIT ROTUNDA",
        ],
        Biome::Foundry => ["THE CHAIN LIFT", "CRUCIBLE CROSSING", "THE TURBINE HEART"],
        Biome::Crown => [
            "THE HIGH BRIDGE",
            "THE CELESTIAL ENGINE",
            "THE REGENT'S CLOCK",
        ],
    };
    names[zone]
}
fn hash(mut n: u32) -> u32 {
    n = n.wrapping_mul(374761393);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
}

struct Piece {
    source: Rect,
    surface: f32,
}
struct Sheet {
    texture: Texture2D,
    pieces: Vec<Piece>,
}
impl Sheet {
    fn new(bytes: &[u8], columns: &[u32], rows: &[u32]) -> Self {
        let cols = columns.len() - 1;
        let regions: Vec<_> = (0..cols * (rows.len() - 1))
            .map(|i| {
                let ox = columns[i % cols];
                let oy = rows[i / cols];
                [ox, oy, columns[i % cols + 1] - ox, rows[i / cols + 1] - oy]
            })
            .collect();
        Self::regions(bytes, &regions, 0.3)
    }
    fn regions(bytes: &[u8], regions: &[[u32; 4]], threshold: f32) -> Self {
        let image = Image::from_file_with_format(bytes, None).expect("embedded environment sheet");
        let mut pieces = vec![];
        for (i, &[ox, oy, cw, ch]) in regions.iter().enumerate() {
            assert!(ox + cw <= image.width as u32 && oy + ch <= image.height as u32);
            let (mut x0, mut y0, mut x1, mut y1) = (cw, ch, 0, 0);
            for y in 0..ch {
                for x in 0..cw {
                    if image.get_pixel(ox + x, oy + y).a > threshold {
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x);
                        y1 = y1.max(y);
                    }
                }
            }
            assert!(x0 <= x1 && y0 <= y1, "empty scenery cell {i}");
            // Moss/roots can precede the slab. Find the first substantial horizontal edge.
            let width = x1 - x0 + 1;
            let mut surface = y0;
            for y in y0..=y1 {
                let covered = (x0..=x1)
                    .filter(|&x| image.get_pixel(ox + x, oy + y).a > 0.5)
                    .count();
                if covered as f32 > width as f32 * 0.65 {
                    surface = y;
                    break;
                }
            }
            pieces.push(Piece {
                source: Rect::new(
                    (ox + x0) as f32,
                    (oy + y0) as f32,
                    width as f32,
                    (y1 - y0 + 1) as f32,
                ),
                surface: (surface - y0) as f32,
            });
        }
        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Nearest);
        Self { texture, pieces }
    }
    fn draw(&self, index: usize, x: f32, y: f32, width: f32, opacity: f32) {
        let p = &self.pieces[index];
        let h = width * p.source.h / p.source.w;
        draw_texture_ex(
            &self.texture,
            (x * 2.).round() / 2.,
            (y * 2.).round() / 2.,
            WHITE.with_alpha(opacity),
            DrawTextureParams {
                source: Some(p.source),
                dest_size: Some(vec2(width, h)),
                ..Default::default()
            },
        );
    }
    fn grounded(&self, index: usize, x: f32, feet: f32, height: f32, opacity: f32) {
        let p = &self.pieces[index];
        let width = height * p.source.w / p.source.h;
        self.draw(index, x - width / 2., feet - height, width, opacity);
    }
    fn platform(&self, index: usize, x: f32, top: f32, width: f32) {
        let p = &self.pieces[index];
        self.draw(index, x, top - p.surface * width / p.source.w, width, 1.);
    }
    fn rotated(&self, index: usize, center: Vec2, height: f32, angle: f32, opacity: f32) {
        let source = self.pieces[index].source;
        let width = height * source.w / source.h;
        draw_texture_ex(
            &self.texture,
            center.x - width / 2.,
            center.y - height / 2.,
            WHITE.with_alpha(opacity),
            DrawTextureParams {
                source: Some(source),
                dest_size: Some(vec2(width, height)),
                rotation: angle,
                pivot: Some(center),
                ..Default::default()
            },
        );
    }
    fn banner(&self, x: f32, top: f32, height: f32, time: f32, opacity: f32) {
        let source = self.pieces[3].source;
        let width = height * source.w / source.h;
        // Pin the crossbar; progressively displace cloth strips toward the frayed hem.
        for strip in 0..20 {
            let u = strip as f32 / 20.;
            let sway = (time * 2.4 - u * 4.).sin() * 4. * u * u;
            draw_texture_ex(
                &self.texture,
                (x - width / 2. + sway).round(),
                top + height * u,
                WHITE.with_alpha(opacity),
                DrawTextureParams {
                    source: Some(Rect::new(
                        source.x,
                        source.y + source.h * u,
                        source.w,
                        source.h / 20.,
                    )),
                    dest_size: Some(vec2(width, height / 20. + 0.5)),
                    ..Default::default()
                },
            );
        }
    }
}
pub struct Environment {
    backdrops: Vec<Texture2D>,
    panoramas: Vec<Texture2D>,
    rooftops: Texture2D,
    undercroft: Texture2D,
    terrain: Sheet,
    props: Sheet,
    mechanisms: Sheet,
}
impl Environment {
    pub fn new() -> Self {
        let files: [&[u8]; 4] = [
            include_bytes!("../assets/environment/aqueduct-v1.png"),
            include_bytes!("../assets/environment/garden-v1.png"),
            include_bytes!("../assets/environment/foundry-v1.png"),
            include_bytes!("../assets/environment/crown-v1.png"),
        ];
        let backdrops = files
            .into_iter()
            .map(|bytes| {
                let t = Texture2D::from_file_with_format(bytes, None);
                t.set_filter(FilterMode::Nearest);
                t
            })
            .collect();
        let panoramas = [
            include_bytes!("../assets/environment/aqueduct-panorama-v1.png").as_slice(),
            include_bytes!("../assets/environment/garden-panorama-v1.png").as_slice(),
            include_bytes!("../assets/environment/foundry-panorama-v1.png").as_slice(),
            include_bytes!("../assets/environment/crown-panorama-v1.png").as_slice(),
        ]
        .into_iter()
        .map(|bytes| {
            let t = Texture2D::from_file_with_format(bytes, None);
            t.set_filter(FilterMode::Nearest);
            t
        })
        .collect();
        Self {
            backdrops,
            panoramas,
            rooftops: Self::depth_texture(include_bytes!(
                "../assets/environment/rooftops-depth-v1.png"
            )),
            undercroft: Self::depth_texture(include_bytes!(
                "../assets/environment/undercroft-depth-v1.png"
            )),
            mechanisms: Sheet::regions(
                include_bytes!("../assets/environment/mechanisms-v1.png"),
                &[
                    [0, 0, 423, 530],
                    [423, 0, 394, 530],
                    [817, 0, 437, 530],
                    [1254, 0, 282, 554],
                    [0, 530, 500, 494],
                    [500, 530, 317, 494],
                    [817, 554, 418, 470],
                    [1235, 554, 301, 470],
                ],
                0.03,
            ),
            // Explicit gutters preserve the generated sheet's uneven column spacing.
            terrain: Sheet::new(
                include_bytes!("../assets/environment/terrain-v1.png"),
                &[0, 420, 920, 1180, 1536],
                &[0, 256, 512, 768, 1024],
            ),
            props: Sheet::new(
                include_bytes!("../assets/environment/props-v1.png"),
                &[0, 384, 768, 1152, 1536],
                &[0, 490, 1024],
            ),
        }
    }
    /// Every texture the scenery draws from.
    pub fn textures(&self) -> impl Iterator<Item = &Texture2D> {
        self.backdrops
            .iter()
            .chain(&self.panoramas)
            .chain([&self.rooftops, &self.undercroft])
            .chain([
                &self.terrain.texture,
                &self.props.texture,
                &self.mechanisms.texture,
            ])
    }
    fn depth_texture(bytes: &[u8]) -> Texture2D {
        let texture = Texture2D::from_file_with_format(bytes, None);
        texture.set_filter(FilterMode::Nearest);
        texture
    }
    fn index(biome: Biome) -> usize {
        match biome {
            Biome::Aqueduct => 0,
            Biome::Garden => 1,
            Biome::Foundry => 2,
            Biome::Crown => 3,
        }
    }
    pub fn background(&self, biome: Biome, cam: f32, width: f32, time: f32, cam_y: f32) {
        clear_background(Color::from_hex(0x0c1522));
        let index = Self::index(biome);
        let texture = &self.backdrops[index];
        let crop_h = (texture.width() * 9. / 16.).min(texture.height());
        let crop = Rect::new(
            0.,
            (texture.height() - crop_h) * 0.45,
            texture.width(),
            crop_h,
        );
        let journey = progress(cam, width);
        // The opening vista gives way to a wider panorama. Travel reveals new
        // architecture continuously; backtracking follows the same spatial sequence.
        draw_texture_ex(
            texture,
            -journey * 160.,
            -61. - (cam_y - 38.) * 0.07,
            WHITE,
            DrawTextureParams {
                source: Some(crop),
                dest_size: Some(vec2(800., 450.)),
                ..Default::default()
            },
        );
        let panorama = &self.panoramas[index];
        let pano_h = 400.;
        let pano_w = (panorama.width() / panorama.height() * pano_h).max(800.);
        draw_texture_ex(
            panorama,
            -journey * (pano_w - 640.),
            -38. - (cam_y - 38.) * 0.035,
            WHITE.with_alpha(smoothstep((journey - 0.08) / 0.38)),
            DrawTextureParams {
                dest_size: Some(vec2(pano_w, pano_h)),
                ..Default::default()
            },
        );
        // Height has its own scenery identity, independently of horizontal
        // biome progress. The wide overlap keeps stairwell transitions smooth.
        for (texture, opacity, anchor) in [
            (&self.rooftops, smoothstep((30. - cam_y) / 170.), -180.),
            (&self.undercroft, smoothstep((cam_y - 140.) / 185.), 360.),
        ] {
            if opacity > 0. {
                let height = 440.;
                let width = height * texture.width() / texture.height();
                draw_texture_ex(
                    texture,
                    -journey * (width - 640.),
                    -52. - (cam_y - anchor) * 0.075,
                    WHITE.with_alpha(opacity),
                    DrawTextureParams {
                        dest_size: Some(vec2(width, height)),
                        ..Default::default()
                    },
                );
            }
        }
        draw_rectangle(0., 0., 640., 360., Color::from_rgba(5, 12, 23, 28));
        self.moving_landmarks(biome, cam, time, cam_y);
        // Sparse middle-distance columns travel independently of the large vistas.
        for i in 0..9 {
            let x = i as f32 * 390. + 130. - cam * 0.62;
            if (-100. ..740.).contains(&x) {
                let h = 145. + (hash(i) % 55) as f32;
                self.terrain
                    .grounded(index * 4 + 2, x, FLOOR + 8. - (cam_y - 38.) * 0.45, h, 0.32);
            }
        }
        self.atmosphere(biome, cam, time);
    }

    fn moving_landmarks(&self, biome: Biome, cam: f32, time: f32, cam_y: f32) {
        let shift = -(cam_y - 38.) * 0.35;
        for i in 0..8 {
            let x = i as f32 * 540. + 250. - cam * 0.44;
            if !(-170. ..810.).contains(&x) {
                continue;
            }
            let phase = time + i as f32 * 1.7;
            match biome {
                Biome::Aqueduct => {
                    self.mechanisms
                        .rotated(0, vec2(x, 219. + shift), 105., phase * 0.17, 0.64);
                    // Narrow descending ribbons and glints establish flowing water.
                    for stream in 0..7 {
                        let sx = x + 53. + stream as f32 * 2.;
                        draw_rectangle(
                            sx,
                            133. + shift,
                            1.,
                            151.,
                            Color::from_rgba(98, 206, 222, 15),
                        );
                        for drop in 0..8 {
                            let y = 133.
                                + shift
                                + (phase * 53. + drop as f32 * 21. + stream as f32 * 9.)
                                    .rem_euclid(145.);
                            draw_rectangle(
                                sx.round(),
                                y.round(),
                                1.,
                                3. + (stream % 3) as f32,
                                Color::from_rgba(169, 231, 226, 65),
                            );
                        }
                    }
                }
                Biome::Garden => {
                    self.terrain.rotated(
                        6,
                        vec2(x, 182. + shift),
                        172.,
                        (phase * 0.65).sin() * 0.025,
                        0.49,
                    );
                    self.mechanisms
                        .banner(x + 121., 65. + shift, 114., phase, 0.57);
                }
                Biome::Foundry => {
                    self.mechanisms
                        .rotated(1, vec2(x, 183. + shift), 128., phase * 0.12, 0.62);
                    self.mechanisms.rotated(
                        1,
                        vec2(x + 88., 227. + shift),
                        68.,
                        -phase * 0.22,
                        0.57,
                    );
                    let rise = (phase * 0.2).fract();
                    self.mechanisms.grounded(
                        5,
                        x + 20.,
                        278. + shift - rise * 80.,
                        65. + rise * 35.,
                        (std::f32::consts::PI * rise).sin() * 0.19,
                    );
                }
                Biome::Crown => {
                    self.mechanisms
                        .rotated(2, vec2(x, 161. + shift), 153., phase * 0.065, 0.54);
                    self.mechanisms
                        .banner(x - 124., 48. + shift, 143., phase, 0.69);
                }
            }
        }
    }

    fn atmosphere(&self, biome: Biome, cam: f32, time: f32) {
        for i in 0..4 {
            let x = (i as f32 * 256. + time * 5. - cam * 0.18).rem_euclid(1024.) - 192.;
            self.mechanisms
                .grounded(4, x, 281. + (time * 0.28 + i as f32).sin() * 5., 34., 0.11);
        }
        for i in 0..32 {
            let n = hash(i * 47 + 11);
            let depth = 0.15 + (n % 5) as f32 * 0.11;
            let offset = (n % 991) as f32;
            match biome {
                Biome::Aqueduct => {
                    let x = (offset - cam * depth).rem_euclid(680.) - 20.;
                    let y = ((n / 991 % 270) as f32 + time * (27. + depth * 34.)).rem_euclid(280.);
                    draw_rectangle(
                        x.round(),
                        y.round(),
                        0.5,
                        2.5,
                        Color::from_rgba(161, 220, 221, 45),
                    );
                }
                Biome::Garden => {
                    let x = (offset + time * (8. + depth * 8.) - cam * depth
                        + (time + i as f32).sin() * 7.)
                        .rem_euclid(700.)
                        - 30.;
                    let y = ((n / 991 % 290) as f32 + time * (9. + depth * 11.)).rem_euclid(300.);
                    self.mechanisms.rotated(
                        6,
                        vec2(x, y),
                        3. + depth * 4.,
                        time * 0.8 + i as f32,
                        0.48 + depth * 0.3,
                    );
                }
                Biome::Foundry => {
                    let x = (offset - cam * depth + (time * 1.4 + i as f32).sin() * 8.)
                        .rem_euclid(680.)
                        - 20.;
                    let y = 290.
                        - ((n / 991 % 270) as f32 + time * (12. + depth * 22.)).rem_euclid(290.);
                    draw_rectangle(
                        x.round(),
                        y.round(),
                        1.,
                        1. + depth * 2.,
                        Color::from_rgba(255, 173, 84, 135),
                    );
                }
                Biome::Crown => {
                    let x = (offset - cam * depth + time * 1.8).rem_euclid(680.) - 20.;
                    let y = (n / 991 % 230) as f32 + (time * 0.6 + i as f32).sin() * 3.;
                    let twinkle = 0.15 + (time * 1.1 + i as f32).sin().abs() * 0.4;
                    draw_rectangle(
                        x.round(),
                        y.round(),
                        1.,
                        1.,
                        Color::from_rgba(167, 224, 230, 255).with_alpha(twinkle),
                    );
                }
            }
        }
    }
    pub fn platform(&self, biome: Biome, p: &Rect, cam: f32) {
        let row = Self::index(biome) * 4;
        if p.h >= 24. {
            // Every corridor owns its visible slab. Never paint a continuous
            // floor over the open shaft or the route below it.
            let tiles = (p.w / 128.).ceil() as u32;
            for tile in 0..tiles {
                let x = p.x + tile as f32 * 128.;
                if x - cam > 680. || x + 128. - cam < -40. {
                    continue;
                }
                self.terrain
                    .platform(row + 1, x - cam, p.y, (p.x + p.w - x).min(128.));
            }
        } else {
            self.terrain.platform(row, p.x - cam, p.y, p.w);
        }
    }
    pub fn dressing(&self, level: &Level, cam: f32, cam_y: f32, time: f32) {
        let row = Self::index(level.biome) * 4;
        for (i, platform) in level.platforms.iter().enumerate() {
            if platform.w < 140. || platform.y < cam_y - 50. || platform.y > cam_y + 430. {
                continue;
            }
            let x = platform.x + 30. - cam;
            if (-70. ..710.).contains(&x) {
                let height = dressing_height(i);
                self.terrain.grounded(row + 3, x, platform.y, height, 0.82);
                if lit_dressing(level.biome) {
                    let flicker = 0.8 + 0.2 * (time * 9. + i as f32 * 4.).sin();
                    self.mechanisms.grounded(
                        7,
                        x,
                        platform.y - height * 0.52,
                        height * 0.2 * flicker,
                        0.8,
                    );
                }
            }
        }
    }
    pub fn object(&self, o: &Object, cam: f32, time: f32) {
        let x = o.pos.x - cam;
        if !(-80. ..720.).contains(&x) {
            return;
        }
        let (index, height) = match o.kind {
            ObjectKind::Chest | ObjectKind::Secret => (usize::from(o.used), 21.),
            ObjectKind::Scroll => {
                if o.used {
                    return;
                }
                (2, 18.)
            }
            ObjectKind::Fountain => (3, 43.),
            ObjectKind::Forge => (4, 31.),
            ObjectKind::Lore => (5, 29.),
            ObjectKind::Exit => (6, 91.),
        };
        let feet = if o.kind == ObjectKind::Scroll {
            o.pos.y - 12. + (time * 2.5).sin() * 2.
        } else {
            o.pos.y
        };
        self.props.grounded(index, x, feet, height, 1.);
        match o.kind {
            ObjectKind::Fountain => {
                for stream in 0..3 {
                    let yy = (time * 19. + stream as f32 * 5.).rem_euclid(17.);
                    draw_rectangle(
                        (x - 5. + stream as f32 * 5.).round(),
                        (feet - 25. + yy).round(),
                        0.5,
                        2.5,
                        Color::from_rgba(121, 252, 226, 180),
                    );
                }
            }
            ObjectKind::Forge => {
                self.mechanisms
                    .grounded(7, x - 7., feet - 13., 6. + (time * 10.).sin(), 0.9)
            }
            ObjectKind::Exit => {
                for i in 0..9 {
                    let y = feet - 7. - (time * 17. + i as f32 * 9.).rem_euclid(67.);
                    let xx = x + (time * 2. + i as f32).sin() * 6.;
                    draw_rectangle(
                        xx.round(),
                        y.round(),
                        1.,
                        2.,
                        Color::from_rgba(149, 255, 239, 150),
                    );
                }
            }
            _ => {}
        }
    }
    pub fn trap(&self, x: f32, y: f32) {
        self.props.grounded(7, x, y, 15., 1.);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn panorama_progress_covers_each_biome_without_overscroll() {
        for width in [1500., 3600.] {
            assert_eq!(progress(-20., width), 0.);
            assert_eq!(progress((width - 640.) / 2., width), 0.5);
            assert_eq!(progress(width - 640., width), 1.);
            assert_eq!(progress(width + 800., width), 1.);
        }
        assert_eq!(progress(100., 500.), 0.);
    }
    #[test]
    fn travel_reveals_three_distinct_areas_and_blends_continuously() {
        for biome in [Biome::Aqueduct, Biome::Garden, Biome::Foundry, Biome::Crown] {
            let names = [
                zone_name(biome, 0., 3600.),
                zone_name(biome, 1480., 3600.),
                zone_name(biome, 2960., 3600.),
            ];
            assert!(names[0] != names[1] && names[1] != names[2]);
        }
        assert_eq!(smoothstep(-1.), 0.);
        assert_eq!(smoothstep(2.), 1.);
        let values: Vec<_> = (0..=100).map(|i| smoothstep(i as f32 / 100.)).collect();
        assert!(values
            .windows(2)
            .all(|p| p[1] >= p[0] && p[1] - p[0] < 0.02));
    }
}
