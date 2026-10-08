//! A fade and a lift for whatever the interface draws inside `with`, so a menu
//! can ease into place without every drawing call taking extra arguments.
//! The interface imports these in place of Macroquad's own functions of the
//! same names; outside `with` they draw exactly as Macroquad's do.
use macroquad::prelude as mq;
use macroquad::prelude::{Color, TextDimensions, TextParams, Texture2D, Vec2};
use std::cell::Cell;

thread_local! {
    /// Opacity multiplier, and how far down (in interface units) to draw.
    static LOOK: Cell<(f32, f32)> = const { Cell::new((1., 0.)) };
}

/// Draws `draw` at `alpha` times its usual opacity, `lift` units lower.
pub fn with(alpha: f32, lift: f32, draw: impl FnOnce()) {
    let before = LOOK.get();
    LOOK.set((before.0 * alpha, before.1 + lift));
    draw();
    LOOK.set(before);
}

fn faded(color: Color) -> Color {
    Color {
        a: color.a * LOOK.get().0,
        ..color
    }
}
fn lift() -> f32 {
    LOOK.get().1
}

pub fn draw_rectangle(x: f32, y: f32, w: f32, h: f32, color: Color) {
    mq::draw_rectangle(x, y + lift(), w, h, faded(color));
}
pub fn draw_rectangle_lines(x: f32, y: f32, w: f32, h: f32, thickness: f32, color: Color) {
    mq::draw_rectangle_lines(x, y + lift(), w, h, thickness, faded(color));
}
pub fn draw_circle(x: f32, y: f32, r: f32, color: Color) {
    mq::draw_circle(x, y + lift(), r, faded(color));
}
pub fn draw_circle_lines(x: f32, y: f32, r: f32, thickness: f32, color: Color) {
    mq::draw_circle_lines(x, y + lift(), r, thickness, faded(color));
}
pub fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
    mq::draw_line(x1, y1 + lift(), x2, y2 + lift(), thickness, faded(color));
}
pub fn draw_poly(x: f32, y: f32, sides: u8, radius: f32, rotation: f32, color: Color) {
    mq::draw_poly(x, y + lift(), sides, radius, rotation, faded(color));
}
pub fn draw_triangle(v1: Vec2, v2: Vec2, v3: Vec2, color: Color) {
    let down = Vec2::new(0., lift());
    mq::draw_triangle(v1 + down, v2 + down, v3 + down, faded(color));
}
pub fn draw_text_ex(text: &str, x: f32, y: f32, params: TextParams) -> TextDimensions {
    let color = faded(params.color);
    mq::draw_text_ex(text, x, y + lift(), TextParams { color, ..params })
}
pub fn draw_texture_ex(
    texture: &Texture2D,
    x: f32,
    y: f32,
    color: Color,
    params: mq::DrawTextureParams,
) {
    mq::draw_texture_ex(texture, x, y + lift(), faded(color), params);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_nest_and_restore() {
        assert_eq!(LOOK.get(), (1., 0.));
        with(0.5, 10., || {
            assert_eq!(faded(mq::WHITE).a, 0.5);
            assert_eq!(lift(), 10.);
            with(0.5, 4., || {
                assert_eq!(faded(mq::WHITE).a, 0.25);
                assert_eq!(lift(), 14.);
            });
            assert_eq!(lift(), 10.);
        });
        assert_eq!(LOOK.get(), (1., 0.), "back to plain drawing");
        assert_eq!(faded(mq::WHITE), mq::WHITE);
    }
}
