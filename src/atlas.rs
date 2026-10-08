//! Import connected sprite silhouettes instead of blindly cutting equal rectangles.
//! Generated swords can extend across a nominal cell boundary. Component-aware slicing
//! preserves those pixels without leaking a neighboring scarf into the animation.
//! Source PNGs remain unchanged; only GPU frame textures are prepared at load time.
use crate::animation::MotionPose;
use macroquad::prelude::*;
struct ImportedFrame {
    image: Image,
    pivot: Vec2,
}
struct Component {
    pixels: Vec<usize>,
    bounds: [usize; 4],
    center: (usize, usize),
}
fn import(image: &Image, columns: usize, rows: usize) -> Vec<ImportedFrame> {
    let w = image.width as usize;
    let h = image.height as usize;
    let mut labels = vec![usize::MAX; w * h];
    let mut components = vec![];
    for start in 0..w * h {
        if labels[start] != usize::MAX || image.bytes[start * 4 + 3] < 70 {
            continue;
        }
        let id = components.len();
        let mut stack = vec![start];
        labels[start] = id;
        let mut pixels = vec![];
        let mut bounds = [w, h, 0, 0];
        let (mut sx, mut sy) = (0, 0);
        while let Some(i) = stack.pop() {
            let x = i % w;
            let y = i / w;
            pixels.push(i);
            sx += x;
            sy += y;
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x);
            bounds[3] = bounds[3].max(y);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let ni = yy * w + xx;
                    if labels[ni] == usize::MAX && image.bytes[ni * 4 + 3] >= 70 {
                        labels[ni] = id;
                        stack.push(ni);
                    }
                }
            }
        }
        let center = (sx / pixels.len(), sy / pixels.len());
        components.push(Component {
            pixels,
            bounds,
            center,
        });
    }
    let mut selected: Vec<_> = components
        .iter()
        .enumerate()
        .filter(|(_, c)| c.pixels.len() > 700)
        .collect();
    selected.sort_by_key(|(_, c)| (c.center.1 / (h / rows), c.center.0));
    assert_eq!(
        selected.len(),
        columns * rows,
        "unexpected connected sprite count"
    );
    let mut frames = vec![];
    for (frame, (id, component)) in selected.iter().enumerate() {
        let b = component.bounds;
        let minx = b[0].saturating_sub(1);
        let miny = b[1].saturating_sub(1);
        let fw = b[2] - minx + 2;
        let fh = b[3] - miny + 2;
        let mut out = Image::gen_image_color(fw as u16, fh as u16, BLANK);
        for &i in &component.pixels {
            let x = i % w;
            let y = i / w;
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let k = yy * w + xx;
                    let same = labels[k] == *id;
                    let fringe = labels[k] == usize::MAX && image.bytes[k * 4 + 3] > 0;
                    if (same || fringe)
                        && xx >= minx
                        && yy >= miny
                        && xx < minx + fw
                        && yy < miny + fh
                    {
                        out.set_pixel(
                            (xx - minx) as u32,
                            (yy - miny) as u32,
                            image.get_pixel(xx as u32, yy as u32),
                        );
                    }
                }
            }
        }
        let nominal_center =
            (frame % columns) as f32 * w as f32 / columns as f32 + w as f32 / columns as f32 / 2.;
        frames.push(ImportedFrame {
            image: out,
            pivot: vec2(nominal_center - minx as f32, (b[3] - miny + 1) as f32),
        });
    }
    frames
}
pub struct SpriteFrame {
    texture: Texture2D,
    silhouette: Texture2D,
    pivot: Vec2,
}
fn silhouette(image: &Image) -> Image {
    let mut mask = image.clone();
    for pixel in mask.bytes.as_chunks_mut::<4>().0 {
        pixel[..3].fill(255);
    }
    mask
}
pub struct Atlas {
    frames: Vec<SpriteFrame>,
    pub scale: f32,
}
impl Atlas {
    pub fn new(
        bytes: &[u8],
        columns: usize,
        rows: usize,
        height: f32,
        reference: std::ops::Range<usize>,
    ) -> Self {
        let image =
            Image::from_file_with_format(bytes, None).expect("embedded sprite atlas must decode");
        let imported = import(&image, columns, rows);
        let mut heights: Vec<_> = reference.map(|i| imported[i].image.height as f32).collect();
        heights.sort_by(f32::total_cmp);
        let scale = height / heights[heights.len() / 2];
        let frames = imported
            .into_iter()
            .map(|f| {
                let texture = Texture2D::from_image(&f.image);
                texture.set_filter(FilterMode::Nearest);
                let silhouette = Texture2D::from_image(&silhouette(&f.image));
                silhouette.set_filter(FilterMode::Nearest);
                SpriteFrame {
                    texture,
                    silhouette,
                    pivot: f.pivot,
                }
            })
            .collect();
        Self { frames, scale }
    }
    /// Every frame's texture and silhouette.
    pub fn textures(&self) -> impl Iterator<Item = &Texture2D> {
        self.frames.iter().flat_map(|f| [&f.texture, &f.silhouette])
    }
    pub fn draw_scaled(&self, frame: usize, feet: Vec2, face: f32, opacity: f32, factor: f32) {
        self.draw_pose(
            frame,
            feet,
            face,
            Color::new(1., 1., 1., opacity),
            MotionPose {
                scale_x: factor,
                scale_y: factor,
                ..Default::default()
            },
        );
    }
    pub fn draw_pose(&self, frame: usize, feet: Vec2, face: f32, tint: Color, pose: MotionPose) {
        self.draw_styled(frame, feet, face, tint, pose, false);
    }
    pub fn draw_silhouette(
        &self,
        frame: usize,
        feet: Vec2,
        face: f32,
        tint: Color,
        pose: MotionPose,
    ) {
        self.draw_styled(frame, feet, face, tint, pose, true);
    }
    fn draw_styled(
        &self,
        frame: usize,
        feet: Vec2,
        face: f32,
        tint: Color,
        pose: MotionPose,
        mask: bool,
    ) {
        let f = &self.frames[frame % self.frames.len()];
        let texture = if mask { &f.silhouette } else { &f.texture };
        let scale = vec2(pose.scale_x, pose.scale_y) * self.scale;
        let px = if face < 0. {
            f.texture.width() - f.pivot.x
        } else {
            f.pivot.x
        };
        let pos = feet - vec2(px, f.pivot.y) * scale;
        let size = vec2(texture.width(), texture.height()) * scale;
        let snap = |n: f32| (n * 2.).round() / 2.;
        if pose.lean.abs() < 0.25 {
            draw_texture_ex(
                texture,
                snap(pos.x),
                snap(pos.y),
                tint,
                DrawTextureParams {
                    dest_size: Some(size),
                    flip_x: face < 0.,
                    ..Default::default()
                },
            );
            return;
        }
        // Two-world-pixel bands keep a lean crisp at the native 2x pixel
        // scale. Unlike texture rotation, the outlines stay on the pixel grid.
        let bands = (size.y / 2.).ceil().max(1.) as usize;
        for row in 0..bands {
            let sy = texture.height() * row as f32 / bands as f32;
            let end = texture.height() * (row + 1) as f32 / bands as f32;
            let top = snap(pos.y + sy * scale.y);
            let bottom = snap(pos.y + end * scale.y);
            let height_above_feet = (f.pivot.y - (sy + end) * 0.5) / f.pivot.y;
            draw_texture_ex(
                texture,
                snap(pos.x + pose.lean * height_above_feet),
                top,
                tint,
                DrawTextureParams {
                    dest_size: Some(vec2(size.x, bottom - top)),
                    source: Some(Rect::new(0., sy, texture.width(), end - sy)),
                    flip_x: face < 0.,
                    ..Default::default()
                },
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hit_silhouette_preserves_coverage_including_antialiased_fringe() {
        let image = Image {
            bytes: vec![10, 20, 30, 0, 40, 50, 60, 73, 70, 80, 90, 255],
            width: 3,
            height: 1,
        };
        let mask = silhouette(&image);
        assert_eq!(
            mask.bytes,
            [255, 255, 255, 0, 255, 255, 255, 73, 255, 255, 255, 255]
        );
        assert_eq!(image.bytes[0], 10, "the source asset remains unchanged");
    }
    #[test]
    fn generated_hero_import_preserves_cross_cell_sword() {
        let im =
            Image::from_file_with_format(include_bytes!("../assets/sprites/wanderer-v2.png"), None)
                .unwrap();
        let frames = import(&im, 8, 4);
        assert_eq!(frames.len(), 32);
        assert!(
            frames[12].image.width > 192,
            "extended sword must survive import"
        );
        for f in frames {
            assert!(f.image.width > 20 && f.image.height > 20);
            assert!(f.pivot.y > 0.);
            assert!(f.image.get_pixel(0, 0).a < 0.1);
        }
    }
    #[test]
    fn boss_frames_have_valid_silhouettes() {
        let im =
            Image::from_file_with_format(include_bytes!("../assets/sprites/regent-v1.png"), None)
                .unwrap();
        assert_eq!(import(&im, 4, 2).len(), 8);
    }
    #[test]
    fn all_enemy_silhouettes_are_separate() {
        let im = Image::from_file_with_format(
            include_bytes!("../assets/sprites/guardians-v1.png"),
            None,
        )
        .unwrap();
        assert_eq!(import(&im, 8, 4).len(), 32);
    }
}
