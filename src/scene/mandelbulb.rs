use std::f32::consts::TAU;

use crate::math::Vec3;

const MAX_MARCH_STEPS: usize = 96;
const MAX_DIST: f32 = 16.0;
const HIT_EPS: f32 = 0.0012;
const BAILOUT: f32 = 2.0;

/// Distance estimator for a power-8 Mandelbulb fractal, returning `(distance, orbit_trap)`.
pub fn mandelbulb_de(pos: Vec3, power: f32) -> (f32, f32) {
    let mut z = pos;
    let mut dr = 1.0_f32;
    let mut r = 0.0_f32;
    let mut trap = 1e6_f32;

    for _ in 0..10 {
        r = z.length();
        trap = trap.min(r);
        if r > BAILOUT {
            break;
        }
        if r < 1e-6 {
            return (0.0, 0.0);
        }

        let theta = (z.z / r).clamp(-1.0, 1.0).acos();
        let phi = z.y.atan2(z.x);
        dr = r.powf(power - 1.0) * power * dr + 1.0;

        let zr = r.powf(power);
        let new_theta = theta * power;
        let new_phi = phi * power;
        let sin_t = new_theta.sin();

        z = Vec3::new(
            sin_t * new_phi.cos(),
            new_theta.sin() * new_phi.sin(),
            new_theta.cos(),
        ) * zr
            + pos;
    }

    let de = if r > 1e-6 { 0.5 * r.ln() * r / dr } else { 0.0 };
    (de, trap)
}

fn calc_normal(p: Vec3, power: f32) -> Vec3 {
    let e = 0.001;
    let dx = mandelbulb_de(p + Vec3::new(e, 0.0, 0.0), power).0
        - mandelbulb_de(p - Vec3::new(e, 0.0, 0.0), power).0;
    let dy = mandelbulb_de(p + Vec3::new(0.0, e, 0.0), power).0
        - mandelbulb_de(p - Vec3::new(0.0, e, 0.0), power).0;
    let dz = mandelbulb_de(p + Vec3::new(0.0, 0.0, e), power).0
        - mandelbulb_de(p - Vec3::new(0.0, 0.0, e), power).0;
    Vec3::new(dx, dy, dz).normalize()
}

fn ambient_occlusion(p: Vec3, n: Vec3, power: f32) -> f32 {
    let mut occ = 0.0_f32;
    let mut weight = 0.5_f32;
    for i in 1..=5 {
        let h = 0.015 + 0.04 * (i as f32);
        let d = mandelbulb_de(p + n * h, power).0;
        occ += (h - d).max(0.0) * weight;
        weight *= 0.55;
    }
    (1.0 - 2.4 * occ).clamp(0.08, 1.0)
}

/// Cosine color palette inspired by Inigo Quilez.
#[inline]
fn cosine_palette(t: f32) -> Vec3 {
    let a = Vec3::new(0.52, 0.48, 0.56);
    let b = Vec3::new(0.45, 0.42, 0.40);
    let c = Vec3::new(1.0, 1.0, 1.0);
    let d = Vec3::new(0.02, 0.14, 0.36);
    Vec3::new(
        a.x + b.x * (TAU * (c.x * t + d.x)).cos(),
        a.y + b.y * (TAU * (c.y * t + d.y)).cos(),
        a.z + b.z * (TAU * (c.z * t + d.z)).cos(),
    )
}

/// Renders the Mandelbulb fractal with orbit-trap coloring and ambient occlusion.
pub fn trace_mandelbulb(origin: Vec3, dir: Vec3, time: f32) -> Vec3 {
    // Gently modulate power around 8.0 for organic breathing if desired, centered at 8.0.
    let power = 8.0 + (time * 0.25).sin() * 0.4;
    let ray_dir = dir.normalize();

    let mut t = 0.0_f32;
    let mut hit = false;
    let mut trap = 0.0_f32;
    let mut steps = 0_usize;

    for i in 0..MAX_MARCH_STEPS {
        steps = i;
        let p = origin + ray_dir * t;
        let (de, tr) = mandelbulb_de(p, power);
        trap = tr;
        if de < HIT_EPS {
            hit = true;
            break;
        }
        t += de * 0.85;
        if t > MAX_DIST {
            break;
        }
    }

    if !hit {
        // Subtle radial glow from iteration count around the fractal silhouette.
        let glow = ((steps as f32) / (MAX_MARCH_STEPS as f32)).powf(1.8);
        let bg = Vec3::new(0.02, 0.03, 0.06);
        return bg + Vec3::new(0.15, 0.35, 0.65) * (glow * 0.55);
    }

    let pos = origin + ray_dir * t;
    let normal = calc_normal(pos, power);
    let ao = ambient_occlusion(pos, normal, power);

    let key_dir = Vec3::new(0.65, 0.72, 0.48).normalize();
    let rim_dir = Vec3::new(-0.7, 0.25, -0.6).normalize();
    let diff = normal.dot(key_dir).max(0.0);
    let half_vec = (key_dir - ray_dir).normalize();
    let spec = normal.dot(half_vec).max(0.0).powf(32.0);
    let rim = (1.0 - (-ray_dir).dot(normal).max(0.0)).powi(3) * normal.dot(rim_dir).max(0.15);

    let albedo = cosine_palette(trap * 0.65 + (steps as f32) * 0.012);
    let rim_col = Vec3::new(0.25, 0.5, 0.9) * rim;
    let lit = albedo * (0.22 + 1.15 * diff) + Vec3::splat(spec * 0.65) + rim_col;
    lit * ao
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mandelbulb_de_is_positive_outside_and_small_at_origin() {
        let (far_de, _) = mandelbulb_de(Vec3::new(3.0, 0.0, 0.0), 8.0);
        assert!(far_de > 1.0, "expected positive far DE, got {far_de}");

        let (origin_de, _) = mandelbulb_de(Vec3::ZERO, 8.0);
        assert!(origin_de <= 0.01, "expected origin inside bulb, got {origin_de}");
    }

    #[test]
    fn mandelbulb_trace_produces_non_black_surface() {
        let col = trace_mandelbulb(Vec3::new(0.0, 0.0, 2.8), Vec3::new(0.0, 0.0, -1.0), 0.0);
        assert!(col.length() > 0.05, "expected lit fractal pixel, got {col:?}");
    }
}
