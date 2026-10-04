//! Small transient pixel effects; authored scenery lives in environment.rs.
use macroquad::prelude::*;
fn c(h: u32) -> Color {
    Color::from_hex(h)
}
fn px(x: f32, y: f32, w: f32, h: f32, col: u32) {
    draw_rectangle(x.floor(), y.floor(), w.ceil(), h.ceil(), c(col));
}
fn hash(mut x: u32) -> u32 {
    x = x.wrapping_mul(374761393);
    x = (x ^ (x >> 13)).wrapping_mul(1274126177);
    x ^ (x >> 16)
}
pub fn flame(x: f32, y: f32, t: f32, blue: bool) {
    let phase = (t * 10.) as u32;
    let cols = if blue {
        [0x165a68, 0x28bdaa, 0x92ffce, 0xecffcb]
    } else {
        [0x8a3443, 0xe77332, 0xffc654, 0xfff5bd]
    };
    for row in 0..14 {
        let width = (7 - row / 2).max(1) as f32;
        let sway =
            ((hash(phase + row as u32 * 13) % 3) as f32 - 1.) * if row > 5 { 1. } else { 0. };
        px(x - width / 2. + sway, y - row as f32, width, 2., cols[0]);
        if row < 10 {
            px(
                x - width / 2. + sway + 1.,
                y - row as f32,
                (width - 2.).max(1.),
                1.,
                cols[1],
            );
        }
        if row < 6 {
            px(x - 1., y - row as f32, 2., 1., cols[2]);
        }
    }
    px(x, y - 2., 1., 3., cols[3]);
}
