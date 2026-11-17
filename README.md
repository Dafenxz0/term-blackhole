# term-blackhole

[![CI](https://github.com/Dafenxz0/term-blackhole/actions/workflows/ci.yml/badge.svg)](https://github.com/Dafenxz0/term-blackhole/actions/workflows/ci.yml)

A real-time 3D raymarching and gravitational-lensing black hole / fractal renderer inside your terminal, written in Rust.

## Features

- **3 Interactive 3D Scenes**:
  - `blackhole`: Schwarzschild null-geodesic light bending around an event horizon, Keplerian swirling accretion disk with relativistic Doppler beaming, photon sphere halo, and warped galactic starfield.
  - `spheres`: Reflective metallic spheres (chrome, gold, cobalt) over a 3D checkerboard floor with soft penumbra shadows, Fresnel reflection, and a moving point light.
  - `mandelbulb`: Power-8 3D Mandelbulb distance estimator with multi-sample ambient occlusion and cosine orbit-trap coloring.
- **2x Vertical Resolution**: Uses Unicode upper half-block glyphs (`▀`) with 24-bit ANSI TrueColor (`\x1b[38;2;...;48;2;...m`) so every terminal cell renders 2 square-aspect pixels.
- **Multi-Core CPU Shading**: Parallelizes pixel rows across all CPU cores with `rayon`.
- **Interactive Camera & Headless Benchmark Mode**: Live `crossterm` controls plus a `--benchmark` CLI mode for headless rendering and FPS measurement.

## Quick Start

```bash
cargo run --release -- --scene blackhole
```

Headless benchmark (no TTY required):

```bash
cargo run --release -- --scene blackhole --frames 5 --width 80 --height 24 --benchmark
```

## CLI Flags

| Flag | Description | Default |
| --- | --- | --- |
| `-s`, `--scene <NAME>` | Scene to render (`blackhole`, `spheres`, `mandelbulb`) | `blackhole` |
| `--frames <N>` | Render `N` frames in headless mode and exit | Interactive loop |
| `--width <W>` | Override render width in terminal columns | Auto terminal width |
| `--height <H>` | Override render height in terminal rows | Auto terminal height |
| `--benchmark` | Headless benchmark mode with FPS and ANSI byte metrics | `false` |
| `-h`, `--help` | Print CLI usage and controls | `false` |

## Interactive Controls

| Key | Action |
| --- | --- |
| `W` / `S` | Zoom camera in / out |
| `A` / `D` or `Left` / `Right` | Orbit camera yaw |
| `Up` / `Down` | Orbit camera pitch |
| `1` / `2` / `3` | Switch between `blackhole`, `spheres`, `mandelbulb` |
| `Space` | Pause / resume time animation |
| `R` | Reset camera to scene default |
| `q` / `Esc` | Quit |

## Architecture

- [`src/math.rs`](src/math.rs): `Vec3` algebra, ACES filmic tone mapping + sRGB gamma conversion, and orbital `Camera` ray generation.
- [`src/scene/blackhole.rs`](src/scene/blackhole.rs): Second-order Schwarzschild geodesic integration, accretion disk crossing + Doppler beaming, and procedural starfield.
- [`src/scene/spheres.rs`](src/scene/spheres.rs): SDF raymarcher with central-difference normals, soft shadows, and 2-bounce reflections.
- [`src/scene/mandelbulb.rs`](src/scene/mandelbulb.rs): Power-8 Mandelbulb distance estimator, ambient occlusion, and cosine palette shading.
- [`src/renderer.rs`](src/renderer.rs): `FrameBuffer`, `rayon` parallel row shader, and TrueColor half-block (`▀`) ANSI encoder with adjacent-cell deduplication.
- [`src/cli.rs`](src/cli.rs) & [`src/main.rs`](src/main.rs): Argument parser, headless benchmark runner, and RAII `crossterm` interactive loop.

## Testing

```bash
cargo test --verbose
```
