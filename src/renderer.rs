use std::fmt::Write;

use rayon::prelude::*;

use crate::math::Camera;
use crate::scene::SceneKind;

/// RGB pixel buffer whose `height` is `2 * terminal_rows` for half-block encoding.
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 3]>,
}

impl FrameBuffer {
    pub fn new(term_cols: usize, term_rows: usize) -> Self {
        let width = term_cols.max(1);
        let height = term_rows.max(1) * 2;
        Self {
            width,
            height,
            pixels: vec![[0, 0, 0]; width * height],
        }
    }

    pub fn resize(&mut self, term_cols: usize, term_rows: usize) {
        let width = term_cols.max(1);
        let height = term_rows.max(1) * 2;
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.pixels.resize(width * height, [0, 0, 0]);
        }
    }
}

/// Shades all pixels of `fb` in parallel across CPU cores using `rayon`.
pub fn render_frame(fb: &mut FrameBuffer, scene: SceneKind, camera: &Camera, time: f32) {
    let w = fb.width;
    let h = fb.height;
    let aspect = (w as f32) / (h as f32);
    let (origin, forward, right, up) = camera.basis();
    let fov_tan = camera.fov_tan;
    let inv_w = 1.0 / (w as f32);
    let inv_h = 1.0 / (h as f32);

    fb.pixels
        .par_chunks_mut(w)
        .enumerate()
        .for_each(|(y, row)| {
            let v = 1.0 - ((y as f32) + 0.5) * inv_h * 2.0;
            for (x, px) in row.iter_mut().enumerate() {
                let u = (((x as f32) + 0.5) * inv_w * 2.0 - 1.0) * aspect;
                let dir = Camera::ray_dir(forward, right, up, fov_tan, u, v);
                *px = scene.shade(origin, dir, time).to_rgb_u8();
            }
        });
}

/// Encodes `fb` into 24-bit ANSI TrueColor using Unicode upper half-block (`▀`).
/// Each terminal cell packs two vertical pixels: top as foreground (`38;2`), bottom as background (`48;2`).
pub fn encode_half_blocks(fb: &FrameBuffer, out: &mut String, cursor_home: bool) {
    out.clear();
    let cell_rows = fb.height / 2;
    out.reserve(fb.width * cell_rows * 24);

    if cursor_home {
        out.push_str("\x1b[H");
    }

    for cy in 0..cell_rows {
        let top_row = &fb.pixels[(cy * 2) * fb.width..(cy * 2 + 1) * fb.width];
        let bot_row = &fb.pixels[(cy * 2 + 1) * fb.width..(cy * 2 + 2) * fb.width];
        let mut prev_pair: Option<([u8; 3], [u8; 3])> = None;

        for (&top, &bot) in top_row.iter().zip(bot_row.iter()) {
            let pair = (top, bot);
            if prev_pair != Some(pair) {
                let _ = write!(
                    out,
                    "\x1b[38;2;{};{};{};48;2;{};{};{}m",
                    top[0], top[1], top[2], bot[0], bot[1], bot[2]
                );
                prev_pair = Some(pair);
            }
            out.push('▀');
        }
        out.push_str("\x1b[0m");
        if cy + 1 < cell_rows {
            out.push_str("\r\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_block_encoder_emits_truecolor_fg_and_bg() {
        let mut fb = FrameBuffer::new(2, 1);
        // Row 0 (top pixels): red, green
        fb.pixels[0] = [255, 10, 20];
        fb.pixels[1] = [0, 200, 30];
        // Row 1 (bottom pixels): blue, white
        fb.pixels[2] = [10, 20, 255];
        fb.pixels[3] = [250, 250, 250];

        let mut encoded = String::new();
        encode_half_blocks(&fb, &mut encoded, false);

        assert!(encoded.contains("\x1b[38;2;255;10;20;48;2;10;20;255m▀"));
        assert!(encoded.contains("\x1b[38;2;0;200;30;48;2;250;250;250m▀"));
        assert!(encoded.ends_with("\x1b[0m"));
    }

    #[test]
    fn half_block_encoder_deduplicates_identical_adjacent_cells() {
        let mut fb = FrameBuffer::new(3, 1);
        for x in 0..3 {
            fb.pixels[x] = [12, 34, 56];
            fb.pixels[3 + x] = [78, 90, 123];
        }

        let mut encoded = String::new();
        encode_half_blocks(&fb, &mut encoded, false);

        assert_eq!(encoded, "\x1b[38;2;12;34;56;48;2;78;90;123m▀▀▀\x1b[0m");
    }

    #[test]
    fn render_frame_populates_buffer_in_parallel() {
        let mut fb = FrameBuffer::new(16, 8);
        let scene = SceneKind::Spheres;
        let cam = scene.default_camera();
        render_frame(&mut fb, scene, &cam, 0.0);
        assert_eq!(fb.pixels.len(), 16 * 16);
        assert!(fb.pixels.iter().any(|&p| p != [0, 0, 0]));
    }
}
