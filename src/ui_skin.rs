//! ImageGen UI sources, cropped without resampling and composed as nine-slice frames.
use macroquad::prelude::*;

pub struct Skin {
    frames: Texture2D,
    frame_bounds: Vec<Rect>,
    icons: Texture2D,
    icon_bounds: Vec<Rect>,
    crest: Texture2D,
    crest_bounds: Rect,
}

fn atlas(bytes: &[u8], cols: u32, rows: u32, row_edges: Option<&[u32]>) -> (Texture2D, Vec<Rect>) {
    let image = Image::from_file_with_format(bytes, None).expect("embedded UI atlas");
    let cw = image.width as u32 / cols;
    let edges: Vec<u32> = row_edges.map_or_else(
        || (0..=rows).map(|r| r * image.height as u32 / rows).collect(),
        |edges| edges.to_vec(),
    );
    assert_eq!(edges.len(), rows as usize + 1);
    assert_eq!(edges.last(), Some(&(image.height as u32)));
    let mut bounds = Vec::new();
    for i in 0..cols * rows {
        let ox = i % cols * cw;
        let oy = edges[(i / cols) as usize];
        let ch = edges[(i / cols + 1) as usize] - oy;
        let (mut x0, mut y0, mut x1, mut y1) = (cw, ch, 0, 0);
        for y in 0..ch {
            for x in 0..cw {
                if image.get_pixel(ox + x, oy + y).a > 0.3 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        assert!(x0 < x1 && y0 < y1, "empty UI atlas cell {i}");
        bounds.push(Rect::new(
            (ox + x0) as f32,
            (oy + y0) as f32,
            (x1 - x0 + 1) as f32,
            (y1 - y0 + 1) as f32,
        ));
    }
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    (texture, bounds)
}

fn region(texture: &Texture2D, source: Rect, dest: Rect, tint: Color) {
    draw_texture_ex(
        texture,
        dest.x.round(),
        dest.y.round(),
        tint,
        DrawTextureParams {
            source: Some(source),
            dest_size: Some(vec2(dest.w, dest.h)),
            ..Default::default()
        },
    );
}

impl Skin {
    pub fn new() -> Self {
        let (frames, frame_bounds) =
            atlas(include_bytes!("../assets/ui/frames-v1.png"), 2, 2, None);
        // The crown tip starts two pixels above its nominal row; cut in the gutter.
        let (icons, icon_bounds) = atlas(
            include_bytes!("../assets/ui/icons-v1.png"),
            4,
            4,
            Some(&[0, 252, 508, 764, 1024]),
        );
        let (crest, crest_bounds) = atlas(include_bytes!("../assets/ui/crest-v1.png"), 1, 1, None);
        Self {
            frames,
            frame_bounds,
            icons,
            icon_bounds,
            crest,
            crest_bounds: crest_bounds[0],
        }
    }

    // Preserve corner pixel density as panels stretch to fit live text and HUD data.
    fn frame(&self, index: usize, dest: Rect, corner: f32) {
        let s = self.frame_bounds[index];
        let edge = (s.h.min(s.w) * 0.24).floor();
        let cap = corner.min(dest.w / 3.).min(dest.h / 3.);
        let sx = [s.x, s.x + edge, s.x + s.w - edge, s.x + s.w];
        let sy = [s.y, s.y + edge, s.y + s.h - edge, s.y + s.h];
        let dx = [dest.x, dest.x + cap, dest.x + dest.w - cap, dest.x + dest.w];
        let dy = [dest.y, dest.y + cap, dest.y + dest.h - cap, dest.y + dest.h];
        for y in 0..3 {
            for x in 0..3 {
                region(
                    &self.frames,
                    Rect::new(sx[x], sy[y], sx[x + 1] - sx[x], sy[y + 1] - sy[y]),
                    Rect::new(dx[x], dy[y], dx[x + 1] - dx[x], dy[y + 1] - dy[y]),
                    WHITE,
                );
            }
        }
    }
    pub fn panel(&self, rect: Rect) {
        self.frame(0, rect, 32.);
    }
    pub fn plaque(&self, rect: Rect) {
        self.frame(3, rect, 16.);
    }
    pub fn slot(&self, rect: Rect) {
        self.frame(2, rect, 12.);
    }
    pub fn gauge(&self, rect: Rect, fraction: f32, color: Color) {
        self.frame(1, rect, 10.);
        let inner = Rect::new(
            rect.x + 14.,
            rect.y + rect.h * 0.35,
            rect.w - 28.,
            rect.h * 0.3,
        );
        draw_rectangle(
            inner.x,
            inner.y,
            inner.w,
            inner.h,
            Color::from_hex(0x111c28),
        );
        let width = (inner.w * fraction.clamp(0., 1.)).round();
        if width > 0. {
            draw_rectangle(inner.x, inner.y, width, inner.h, color);
            draw_rectangle(
                inner.x,
                inner.y,
                width,
                2.,
                Color::from_rgba(255, 245, 207, 120),
            );
        }
    }
    pub fn icon(&self, index: usize, rect: Rect, opacity: f32) {
        let s = self.icon_bounds[index];
        let scale = (rect.w / s.w).min(rect.h / s.h);
        let w = s.w * scale;
        let h = s.h * scale;
        region(
            &self.icons,
            s,
            Rect::new(rect.x + (rect.w - w) / 2., rect.y + (rect.h - h) / 2., w, h),
            WHITE.with_alpha(opacity),
        );
    }
    pub fn crest(&self, rect: Rect) {
        let s = self.crest_bounds;
        let scale = (rect.w / s.w).min(rect.h / s.h);
        region(
            &self.crest,
            s,
            Rect::new(
                rect.x + (rect.w - s.w * scale) / 2.,
                rect.y + (rect.h - s.h * scale) / 2.,
                s.w * scale,
                s.h * scale,
            ),
            WHITE,
        );
    }
}
