mod cli;
mod math;
mod renderer;
mod scene;

use std::io::{self, BufWriter, Write};
use std::process;
use std::time::{Duration, Instant};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};

use cli::{help_text, parse_args, CliConfig};
use math::Camera;
use renderer::{encode_half_blocks, render_frame, FrameBuffer};
use scene::SceneKind;

fn handle_key(
    code: KeyCode,
    modifiers: KeyModifiers,
    scene: &mut SceneKind,
    cam: &mut Camera,
    paused: &mut bool,
) -> bool {
    match (code, modifiers) {
        (KeyCode::Char('q' | 'Q'), _) | (KeyCode::Esc, _) => return true,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => return true,
        (KeyCode::Char('1'), _) => {
            *scene = SceneKind::Blackhole;
            *cam = scene.default_camera();
        }
        (KeyCode::Char('2'), _) => {
            *scene = SceneKind::Spheres;
            *cam = scene.default_camera();
        }
        (KeyCode::Char('3'), _) => {
            *scene = SceneKind::Mandelbulb;
            *cam = scene.default_camera();
        }
        (KeyCode::Char(' '), _) => *paused = !*paused,
        (KeyCode::Char('r' | 'R'), _) => *cam = scene.default_camera(),
        (KeyCode::Char('w' | 'W'), _) => cam.adjust_orbit(0.0, 0.0, -0.35),
        (KeyCode::Char('s' | 'S'), _) => cam.adjust_orbit(0.0, 0.0, 0.35),
        (KeyCode::Char('a' | 'A') | KeyCode::Left, _) => cam.adjust_orbit(-0.10, 0.0, 0.0),
        (KeyCode::Char('d' | 'D') | KeyCode::Right, _) => cam.adjust_orbit(0.10, 0.0, 0.0),
        (KeyCode::Up, _) => cam.adjust_orbit(0.0, 0.08, 0.0),
        (KeyCode::Down, _) => cam.adjust_orbit(0.0, -0.08, 0.0),
        _ => {}
    }
    false
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All))?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

fn run_headless(cfg: &CliConfig) -> io::Result<()> {
    let cols = cfg.width.unwrap_or(80);
    let rows = cfg.height.unwrap_or(24);
    let frames = cfg.frames.unwrap_or(5);
    let mut fb = FrameBuffer::new(cols, rows);
    let mut encoded = String::new();
    let mut cam = cfg.scene.default_camera();

    let start = Instant::now();
    let mut total_bytes = 0_usize;

    for i in 0..frames {
        let time = (i as f32) * 0.08;
        cam.adjust_orbit(0.03, 0.0, 0.0);
        render_frame(&mut fb, cfg.scene, &cam, time);
        encode_half_blocks(&fb, &mut encoded, false);
        total_bytes += encoded.len();

        if !cfg.benchmark && i + 1 == frames {
            let mut out = io::stdout().lock();
            out.write_all(encoded.as_bytes())?;
            out.write_all(b"\n")?;
            out.flush()?;
        }
    }

    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
    let fps = (frames as f64) / elapsed;
    let ms_per_frame = (elapsed * 1000.0) / (frames as f64);

    if cfg.benchmark {
        println!(
            "scene={} resolution={}x{}px ({}x{} cells) frames={} elapsed={:.3}s avg={:.2}ms/frame fps={:.2} ansi_bytes={}",
            cfg.scene.label(),
            fb.width,
            fb.height,
            cols,
            rows,
            frames,
            elapsed,
            ms_per_frame,
            fps,
            total_bytes
        );
    }

    Ok(())
}

fn run_interactive(cfg: &CliConfig) -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut stdout = BufWriter::with_capacity(1 << 16, io::stdout());

    let (term_w, term_h) = terminal::size().unwrap_or((80, 24));
    let mut cols = cfg.width.unwrap_or(term_w as usize).max(16);
    let mut rows = cfg.height.unwrap_or(term_h as usize).max(8);

    let mut scene = cfg.scene;
    let mut cam = scene.default_camera();
    let mut fb = FrameBuffer::new(cols, rows.saturating_sub(1).max(1));
    let mut encoded = String::new();
    let mut paused = false;
    let mut sim_time = 0.0_f32;
    let mut last_tick = Instant::now();
    let mut fps_smooth = 0.0_f64;
    let mut rendered = 0_usize;

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;

        if dt > 0.0 {
            let inst_fps = 1.0 / dt;
            fps_smooth = if fps_smooth == 0.0 {
                inst_fps
            } else {
                fps_smooth * 0.85 + inst_fps * 0.15
            };
        }

        if !paused {
            sim_time += dt as f32;
        }

        while event::poll(Duration::from_millis(1))? {
            match event::read()? {
                Event::Resize(w, h) => {
                    if cfg.width.is_none() {
                        cols = (w as usize).max(16);
                    }
                    if cfg.height.is_none() {
                        rows = (h as usize).max(8);
                    }
                    fb.resize(cols, rows.saturating_sub(1).max(1));
                }
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if handle_key(key.code, key.modifiers, &mut scene, &mut cam, &mut paused) {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        render_frame(&mut fb, scene, &cam, sim_time);
        encode_half_blocks(&fb, &mut encoded, true);
        stdout.write_all(encoded.as_bytes())?;

        let status = format!(
            "\r\n\x1b[48;2;18;22;30;38;2;220;230;245m [1:blackhole 2:spheres 3:mandelbulb] active:{} | {}x{}px | {:5.1} FPS | {} | WASD/Arrows Space:Pause r:Reset q:Quit\x1b[K\x1b[0m",
            scene.label(),
            fb.width,
            fb.height,
            fps_smooth,
            if paused { "PAUSED" } else { "LIVE" }
        );
        stdout.write_all(status.as_bytes())?;
        stdout.flush()?;

        rendered += 1;
        if let Some(limit) = cfg.frames {
            if rendered >= limit {
                return Ok(());
            }
        }
    }
}

fn main() {
    let cfg = match parse_args(std::env::args()) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(2);
        }
    };

    if cfg.show_help {
        println!("{}", help_text());
        return;
    }

    let res = if cfg.benchmark || cfg.frames.is_some() {
        run_headless(&cfg)
    } else {
        run_interactive(&cfg)
    };

    if let Err(err) = res {
        eprintln!("runtime error: {err}");
        process::exit(1);
    }
}
