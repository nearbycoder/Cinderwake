//! The desktop window's icon, cut from the interface crest at launch so no
//! separate image has to be kept in step with the artwork.
use macroquad::{miniquad::conf::Icon, prelude::*};

/// A square around the crest's central medallion, in source pixels: left,
/// top, and side. The flanking scrollwork would shrink to nothing at
/// taskbar sizes, so it's left out.
const CROP: (u32, u32, u32) = (362, 157, 720);

/// The crest at 16, 32, and 64 pixels, or `None` if the artwork can't be
/// decoded (the platform then shows its default icon).
pub fn window_icon() -> Option<Icon> {
    let crest = Image::from_file_with_format(
        include_bytes!("../assets/ui/crest-v1.png"),
        Some(ImageFormat::Png),
    )
    .ok()?;
    Some(Icon {
        small: shrink(&crest),
        medium: shrink(&crest),
        big: shrink(&crest),
    })
}

/// Box-filters the crop down to a square RGBA image `LEN / 4` pixels in
/// area. Colour is weighted by alpha so transparent pixels don't darken
/// the edges.
fn shrink<const LEN: usize>(image: &Image) -> [u8; LEN] {
    let size = ((LEN / 4) as f32).sqrt() as u32;
    let (left, top, side) = CROP;
    let width = image.width as u32;
    let mut out = [0u8; LEN];
    for oy in 0..size {
        for ox in 0..size {
            let span = |o: u32| {
                (
                    o * side / size,
                    ((o + 1) * side / size).max(o * side / size + 1),
                )
            };
            let ((x0, x1), (y0, y1)) = (span(ox), span(oy));
            let mut sum = [0f32; 4];
            for y in top + y0..top + y1 {
                for x in left + x0..left + x1 {
                    let i = ((y * width + x) * 4) as usize;
                    let px = &image.bytes[i..i + 4];
                    let a = px[3] as f32;
                    for c in 0..3 {
                        sum[c] += px[c] as f32 * a;
                    }
                    sum[3] += a;
                }
            }
            let count = ((x1 - x0) * (y1 - y0)) as f32;
            let o = ((oy * size + ox) * 4) as usize;
            if sum[3] > 0. {
                for c in 0..3 {
                    out[o + c] = (sum[c] / sum[3]).round() as u8;
                }
            }
            out[o + 3] = (sum[3] / count).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_shows_the_medallion_at_every_size() {
        let icon = window_icon().expect("the crest decodes");
        let alpha = |pixels: &[u8], size: usize, x: usize, y: usize| pixels[(y * size + x) * 4 + 3];
        for (pixels, size) in [
            (&icon.small[..], 16),
            (&icon.medium[..], 32),
            (&icon.big[..], 64),
        ] {
            let mid = size / 2;
            assert!(
                alpha(pixels, size, mid, mid) > 240,
                "{size}px: opaque centre"
            );
            assert_eq!(
                alpha(pixels, size, 0, size - 1),
                0,
                "{size}px: clear corner"
            );
            let opaque = pixels.chunks(4).filter(|p| p[3] > 128).count();
            let share = opaque as f32 / (size * size) as f32;
            assert!((0.4..0.9).contains(&share), "{size}px: {share} opaque");
            // The medallion's glass is teal: more green and blue than red.
            // Sample it diagonally from the centre, clear of the brass spokes.
            let off = mid - size / 8;
            let p = &pixels[(off * size + off) * 4..][..4];
            assert!(
                p[1] > p[0] && p[2] > p[0],
                "{size}px: teal glass, got {p:?}"
            );
        }
    }
}
