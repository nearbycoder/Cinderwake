use macroquad::prelude::*;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let im = Image::from_file_with_format(&std::fs::read(&args[1]).unwrap(), None).unwrap();
    let w = im.width as usize;
    let h = im.height as usize;
    let mut seen = vec![false; w * h];
    let mut components = vec![];
    for start in 0..w * h {
        if seen[start] || im.bytes[start * 4 + 3] < 70 {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let (mut n, mut sx, mut sy) = (0, 0, 0);
        let mut bounds = [w, h, 0, 0];
        while let Some(i) = stack.pop() {
            let x = i % w;
            let y = i / w;
            n += 1;
            sx += x;
            sy += y;
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x);
            bounds[3] = bounds[3].max(y);
            for yy in y.saturating_sub(1)..=(y + 1).min(h - 1) {
                for xx in x.saturating_sub(1)..=(x + 1).min(w - 1) {
                    let ni = yy * w + xx;
                    if !seen[ni] && im.bytes[ni * 4 + 3] >= 70 {
                        seen[ni] = true;
                        stack.push(ni);
                    }
                }
            }
        }
        if n > 700 {
            components.push((sy / n / (h / 4), sx / n, n, bounds));
        }
    }
    components.sort_by_key(|c| (c.0, c.1));
    for c in &components {
        println!("{c:?}");
    }
    println!("total {}", components.len());
}
