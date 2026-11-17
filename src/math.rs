use std::ops::{Add, Div, Mul, Neg, Sub};

/// 3D floating-point vector used for raymarching, SDFs, and RGB shading.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);
    pub const ONE: Self = Self::new(1.0, 1.0, 1.0);
    pub const UP: Self = Self::new(0.0, 1.0, 0.0);

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v, z: v }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-8 {
            self / len
        } else {
            Self::ZERO
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    #[inline]
    pub fn reflect(self, normal: Self) -> Self {
        self - normal * (2.0 * self.dot(normal))
    }

    #[inline]
    pub fn hadamard(self, other: Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    /// Applies ACES-like filmic tone mapping and sRGB gamma correction to `[0, 255]`.
    #[inline]
    pub fn to_rgb_u8(self) -> [u8; 3] {
        let map = |c: f32| -> u8 {
            let x = c.max(0.0);
            let mapped = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
            let gamma = mapped.clamp(0.0, 1.0).powf(1.0 / 2.2);
            (gamma * 255.0 + 0.5) as u8
        };
        [map(self.x), map(self.y), map(self.z)]
    }
}

impl Add for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self * rhs.x, self * rhs.y, self * rhs.z)
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;

    #[inline]
    fn div(self, rhs: f32) -> Self {
        let inv = 1.0 / rhs;
        Self::new(self.x * inv, self.y * inv, self.z * inv)
    }
}

impl Neg for Vec3 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

/// Orbital camera around a target point.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_tan: f32,
}

impl Camera {
    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32, fov_deg: f32) -> Self {
        let half_rad = (fov_deg * 0.5).to_radians();
        Self {
            target,
            yaw,
            pitch,
            distance,
            fov_tan: half_rad.tan(),
        }
    }

    pub fn position(&self) -> Vec3 {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let offset = Vec3::new(cp * sy, sp, cp * cy) * self.distance;
        self.target + offset
    }

    /// Returns `(origin, forward, right, up)` orthonormal camera basis.
    pub fn basis(&self) -> (Vec3, Vec3, Vec3, Vec3) {
        let origin = self.position();
        let forward = (self.target - origin).normalize();
        let right = forward.cross(Vec3::UP).normalize();
        let up = right.cross(forward).normalize();
        (origin, forward, right, up)
    }

    /// Generates a normalized world-space ray direction for NDC `u, v in [-1, 1]`.
    #[inline]
    pub fn ray_dir(forward: Vec3, right: Vec3, up: Vec3, fov_tan: f32, u: f32, v: f32) -> Vec3 {
        (forward + right * (u * fov_tan) + up * (v * fov_tan)).normalize()
    }

    pub fn adjust_orbit(&mut self, d_yaw: f32, d_pitch: f32, d_dist: f32) {
        self.yaw += d_yaw;
        self.pitch = (self.pitch + d_pitch).clamp(-1.45, 1.45);
        self.distance = (self.distance + d_dist).clamp(1.8, 28.0);
    }
}

#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec3_dot_cross_normalize() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::new(0.0, 1.0, 0.0);
        assert!(a.dot(b).abs() < 1e-6);
        let c = a.cross(b);
        assert!((c - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-6);

        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!((v.length() - 5.0).abs() < 1e-6);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn vec3_reflect_and_lerp() {
        let incident = Vec3::new(1.0, -1.0, 0.0).normalize();
        let normal = Vec3::new(0.0, 1.0, 0.0);
        let reflected = incident.reflect(normal);
        let expected = Vec3::new(1.0, 1.0, 0.0).normalize();
        assert!((reflected - expected).length() < 1e-6);

        let mid = Vec3::ZERO.lerp(Vec3::new(2.0, 4.0, 6.0), 0.5);
        assert!((mid - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
    }

    #[test]
    fn camera_basis_is_orthonormal() {
        let cam = Camera::new(Vec3::ZERO, 0.4, 0.25, 10.0, 60.0);
        let (origin, fwd, right, up) = cam.basis();
        assert!((origin.length() - 10.0).abs() < 1e-4);
        assert!((fwd.length() - 1.0).abs() < 1e-5);
        assert!((right.length() - 1.0).abs() < 1e-5);
        assert!((up.length() - 1.0).abs() < 1e-5);
        assert!(fwd.dot(right).abs() < 1e-5);
        assert!(fwd.dot(up).abs() < 1e-5);
        assert!(right.dot(up).abs() < 1e-5);
    }
}
