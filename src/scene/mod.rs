pub mod blackhole;
pub mod mandelbulb;
pub mod spheres;

use crate::math::{Camera, Vec3};

/// Available 3D raymarched scenes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneKind {
    #[default]
    Blackhole,
    Spheres,
    Mandelbulb,
}

impl SceneKind {
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "blackhole" | "bh" | "1" => Ok(Self::Blackhole),
            "spheres" | "sphere" | "2" => Ok(Self::Spheres),
            "mandelbulb" | "fractal" | "3" => Ok(Self::Mandelbulb),
            other => Err(format!(
                "unknown scene '{other}' (expected: blackhole, spheres, mandelbulb)"
            )),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Blackhole => "blackhole",
            Self::Spheres => "spheres",
            Self::Mandelbulb => "mandelbulb",
        }
    }

    pub fn default_camera(self) -> Camera {
        match self {
            Self::Blackhole => Camera::new(Vec3::ZERO, 0.35, 0.22, 10.5, 58.0),
            Self::Spheres => Camera::new(Vec3::new(0.0, -0.2, 0.0), 0.55, 0.26, 6.4, 58.0),
            Self::Mandelbulb => Camera::new(Vec3::ZERO, 0.65, 0.30, 2.75, 54.0),
        }
    }

    #[inline]
    pub fn shade(self, origin: Vec3, dir: Vec3, time: f32) -> Vec3 {
        match self {
            Self::Blackhole => blackhole::trace_blackhole(origin, dir, time),
            Self::Spheres => spheres::trace_spheres(origin, dir, time),
            Self::Mandelbulb => mandelbulb::trace_mandelbulb(origin, dir, time),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_selection_parses_valid_and_rejects_invalid() {
        assert_eq!(SceneKind::from_name("blackhole").unwrap(), SceneKind::Blackhole);
        assert_eq!(SceneKind::from_name("SPHERES").unwrap(), SceneKind::Spheres);
        assert_eq!(SceneKind::from_name("mandelbulb").unwrap(), SceneKind::Mandelbulb);
        assert!(SceneKind::from_name("unknown").is_err());
    }
}
