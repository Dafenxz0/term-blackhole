use crate::scene::SceneKind;

/// Parsed command-line options for `term-blackhole`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliConfig {
    pub scene: SceneKind,
    pub frames: Option<usize>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub benchmark: bool,
    pub show_help: bool,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            scene: SceneKind::Blackhole,
            frames: None,
            width: None,
            height: None,
            benchmark: false,
            show_help: false,
        }
    }
}

pub fn help_text() -> &'static str {
    "term-blackhole - Real-time 3D raymarching & gravitational lensing in your terminal\n\
     \n\
     USAGE:\n\
       term-blackhole [OPTIONS]\n\
     \n\
     OPTIONS:\n\
       -s, --scene <NAME>   Scene to render: blackhole | spheres | mandelbulb [default: blackhole]\n\
           --frames <N>     Render N frames and exit\n\
           --width <W>      Override terminal column width\n\
           --height <H>     Override terminal row height\n\
           --benchmark      Headless benchmark mode (no raw mode / alternate screen)\n\
       -h, --help           Print help information\n\
     \n\
     INTERACTIVE CONTROLS:\n\
       W / S                Zoom in / out\n\
       A / D or Left/Right  Orbit yaw\n\
       Up / Down            Orbit pitch\n\
       1 / 2 / 3            Switch scene (blackhole / spheres / mandelbulb)\n\
       Space                Pause / resume animation\n\
       q / Esc              Quit"
}

pub fn parse_args<I, S>(args: I) -> Result<CliConfig, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut cfg = CliConfig::default();
    let mut iter = args.into_iter().skip(1);

    while let Some(arg) = iter.next() {
        let flag = arg.as_ref();
        match flag {
            "-h" | "--help" => {
                cfg.show_help = true;
                return Ok(cfg);
            }
            "--benchmark" => cfg.benchmark = true,
            "-s" | "--scene" => {
                let val = iter.next().ok_or_else(|| format!("missing value for {flag}"))?;
                cfg.scene = SceneKind::from_name(val.as_ref())?;
            }
            "--frames" => {
                let val = iter.next().ok_or_else(|| "missing value for --frames".to_string())?;
                let n: usize = val
                    .as_ref()
                    .parse()
                    .map_err(|_| format!("invalid --frames value '{}'", val.as_ref()))?;
                if n == 0 {
                    return Err("--frames must be at least 1".to_string());
                }
                cfg.frames = Some(n);
            }
            "--width" => {
                let val = iter.next().ok_or_else(|| "missing value for --width".to_string())?;
                let w: usize = val
                    .as_ref()
                    .parse()
                    .map_err(|_| format!("invalid --width value '{}'", val.as_ref()))?;
                if w < 4 {
                    return Err("--width must be at least 4".to_string());
                }
                cfg.width = Some(w);
            }
            "--height" => {
                let val = iter.next().ok_or_else(|| "missing value for --height".to_string())?;
                let h: usize = val
                    .as_ref()
                    .parse()
                    .map_err(|_| format!("invalid --height value '{}'", val.as_ref()))?;
                if h < 2 {
                    return Err("--height must be at least 2".to_string());
                }
                cfg.height = Some(h);
            }
            unknown => return Err(format!("unknown argument '{unknown}' (try --help)")),
        }
    }

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_benchmark_flags() {
        let cfg = parse_args([
            "term-blackhole",
            "--scene",
            "mandelbulb",
            "--frames",
            "10",
            "--width",
            "80",
            "--height",
            "24",
            "--benchmark",
        ])
        .unwrap();

        assert_eq!(cfg.scene, SceneKind::Mandelbulb);
        assert_eq!(cfg.frames, Some(10));
        assert_eq!(cfg.width, Some(80));
        assert_eq!(cfg.height, Some(24));
        assert!(cfg.benchmark);
        assert!(!cfg.show_help);
    }

    #[test]
    fn rejects_invalid_scene_or_zero_frames() {
        assert!(parse_args(["bin", "--scene", "nope"]).is_err());
        assert!(parse_args(["bin", "--frames", "0"]).is_err());
        assert!(parse_args(["bin", "--bogus"]).is_err());
    }
}
