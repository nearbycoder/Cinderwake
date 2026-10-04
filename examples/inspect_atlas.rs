use macroquad::prelude::*;
fn main() {
    let bytes = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let im = Image::from_file_with_format(&bytes, None).unwrap();
    println!(
        "{}x{} alpha corner {:?}",
        im.width,
        im.height,
        im.get_pixel(0, 0)
    );
    let cols = 8;
    let rows = 4;
    for i in 0..cols * rows {
        let x0 = i % cols * im.width as u32 / cols;
        let y0 = i / cols * im.height as u32 / rows;
        let mut b = [9999, 9999, 0, 0];
        let mut n = 0;
        for y in y0..y0 + im.height as u32 / rows {
            for x in x0..x0 + im.width as u32 / cols {
                if im.get_pixel(x, y).a > 0.5 {
                    b[0] = b[0].min(x - x0);
                    b[1] = b[1].min(y - y0);
                    b[2] = b[2].max(x - x0);
                    b[3] = b[3].max(y - y0);
                    n += 1;
                }
            }
        }
        println!("frame {i}: {b:?} pixels {n}");
    }
}
