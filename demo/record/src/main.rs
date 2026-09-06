use gif::{DisposalMethod, Encoder, Frame, Repeat};
use nano9_raster_demo::scene::{autoplay_frames, palette, HEIGHT, WIDTH};
use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

const SCALE: u32 = 4;

fn main() {
    let out = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../graphics/demo.gif")
    });
    if let Some(dir) = out.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let frames = autoplay_frames();
    let pal = palette();
    let screen_w = (WIDTH * SCALE) as u16;
    let screen_h = (HEIGHT * SCALE) as u16;
    let file = File::create(&out).unwrap();
    let mut enc = Encoder::new(BufWriter::new(file), screen_w, screen_h, &pal).unwrap();
    enc.set_repeat(Repeat::Infinite).unwrap();

    let mut prev = vec![0u8; (WIDTH * HEIGHT) as usize];
    let mut n = 0usize;
    for (i, (indexed, delay_cs)) in frames.iter().enumerate() {
        let full = i == 0;
        let (x0, y0, bw, bh) = if full {
            (0, 0, WIDTH, HEIGHT)
        } else {
            match dirty_bounds(&prev, indexed) {
                Some(b) => b,
                None => {
                    prev.clone_from(indexed);
                    continue;
                }
            }
        };
        let scaled = crop_scale(indexed, x0, y0, bw, bh, SCALE);
        let mut frame = Frame::from_indexed_pixels(
            (bw * SCALE) as u16,
            (bh * SCALE) as u16,
            Cow::Owned(scaled),
            None,
        );
        frame.left = (x0 * SCALE) as u16;
        frame.top = (y0 * SCALE) as u16;
        frame.delay = *delay_cs;
        frame.dispose = DisposalMethod::Keep;
        enc.write_frame(&frame).unwrap();
        prev.clone_from(indexed);
        n += 1;
    }

    eprintln!("wrote {} ({} unique frames)", out.display(), n);
}

fn dirty_bounds(prev: &[u8], cur: &[u8]) -> Option<(u32, u32, u32, u32)> {
    let mut min_x = WIDTH;
    let mut min_y = HEIGHT;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any = false;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let i = (y * WIDTH + x) as usize;
            if prev[i] != cur[i] {
                any = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    any.then_some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
}

fn crop_scale(src: &[u8], x0: u32, y0: u32, bw: u32, bh: u32, scale: u32) -> Vec<u8> {
    let w = bw * scale;
    let mut out = vec![0u8; (w * bh * scale) as usize];
    for y in 0..bh {
        for x in 0..bw {
            let c = src[((y0 + y) * WIDTH + x0 + x) as usize];
            let px = x * scale;
            let py = y * scale;
            for dy in 0..scale {
                let row = ((py + dy) * w + px) as usize;
                out[row..row + scale as usize].fill(c);
            }
        }
    }
    out
}
