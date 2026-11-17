use crate::math::{smoothstep, Vec3};

pub const SCHWARZSCHILD_RADIUS: f32 = 1.0;
const DISK_INNER_RADIUS: f32 = 1.85;
const DISK_OUTER_RADIUS: f32 = 5.6;
const ESCAPE_RADIUS_SQ: f32 = 400.0;
const MAX_STEPS: usize = 160;

/// Computes Schwarzschild null-geodesic ray acceleration:
/// `a = -1.5 * r_s * |p x v|^2 / |p|^5 * p`.
#[inline]
pub fn gravitational_acceleration(pos: Vec3, vel: Vec3, rs: f32) -> Vec3 {
    let r2 = pos.length_sq();
    if r2 < 1e-6 {
        return Vec3::ZERO;
    }
    let h_sq = pos.cross(vel).length_sq();
    let r5 = r2 * r2 * r2.sqrt();
    pos * (-1.5 * rs * h_sq / r5)
}

/// Advances one geodesic step with adaptive step size near the photon sphere.
#[inline]
pub fn step_geodesic(pos: Vec3, dir: Vec3, rs: f32, base_step: f32) -> (Vec3, Vec3) {
    let r = pos.length();
    let eff_r = (r - rs * 0.65).max(0.18);
    let dt = (base_step * eff_r).min(0.42);
    let acc1 = gravitational_acceleration(pos, dir, rs);
    let mid_pos = pos + dir * (0.5 * dt);
    let mid_dir = (dir + acc1 * (0.5 * dt)).normalize();
    let acc2 = gravitational_acceleration(mid_pos, mid_dir, rs);

    let next_pos = pos + mid_dir * dt;
    let next_dir = (dir + acc2 * dt).normalize();
    (next_pos, next_dir)
}

#[inline]
fn hash3(p: Vec3) -> f32 {
    let h = p.dot(Vec3::new(127.1, 311.7, 74.7));
    (h.sin() * 43758.547).fract().abs()
}

#[inline]
fn noise2(x: f32, y: f32) -> f32 {
    let ix = x.floor();
    let iy = y.floor();
    let fx = x - ix;
    let fy = y - iy;
    let ux = fx * fx * (3.0 - 2.0 * fx);
    let uy = fy * fy * (3.0 - 2.0 * fy);

    let h = |gx: f32, gy: f32| -> f32 {
        let s = (gx * 127.1 + gy * 311.7).sin() * 43758.547;
        s.fract().abs()
    };

    let a = h(ix, iy);
    let b = h(ix + 1.0, iy);
    let c = h(ix, iy + 1.0);
    let d = h(ix + 1.0, iy + 1.0);

    a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy
}

fn fbm2(mut x: f32, mut y: f32) -> f32 {
    let mut val = 0.0;
    let mut amp = 0.5;
    for _ in 0..4 {
        val += amp * noise2(x, y);
        x = x * 2.03 + 13.7;
        y = y * 2.03 - 7.3;
        amp *= 0.5;
    }
    val
}

fn sample_starfield(dir: Vec3) -> Vec3 {
    let d = dir.normalize();
    let mut color = Vec3::new(0.008, 0.011, 0.022);

    // Subtle galactic plane glow that visibly bends into an Einstein ring.
    let galactic_plane = (d.y * 0.8 + d.x * 0.35 - d.z * 0.45).abs();
    let band = (-galactic_plane * 5.0).exp();
    let dust = fbm2(d.x * 4.5 + d.z * 2.1, d.y * 4.5 - d.x * 1.7);
    color = color + Vec3::new(0.05, 0.08, 0.16) * (band * (0.35 + 0.65 * dust));

    // Two star density layers.
    for &(scale, threshold, brightness) in &[(75.0_f32, 0.992_f32, 2.4_f32), (140.0, 0.995, 1.8)] {
        let cell = Vec3::new(
            (d.x * scale).floor(),
            (d.y * scale).floor(),
            (d.z * scale).floor(),
        );
        let h = hash3(cell);
        if h > threshold {
            let center = Vec3::new(
                (cell.x + 0.5) / scale,
                (cell.y + 0.5) / scale,
                (cell.z + 0.5) / scale,
            )
            .normalize();
            let dist = (d - center).length() * scale;
            let falloff = (1.0 - dist * 2.2).max(0.0);
            let intensity = falloff * falloff * brightness * ((h - threshold) / (1.0 - threshold));
            let tint = (hash3(cell + Vec3::ONE) * 2.0 - 1.0) * 0.25;
            let star_col = Vec3::new(1.0 + tint, 0.95, 1.0 - tint);
            color = color + star_col * intensity;
        }
    }

    color
}

fn shade_disk(hit: Vec3, ray_dir: Vec3, time: f32) -> (Vec3, f32) {
    let r = (hit.x * hit.x + hit.z * hit.z).sqrt();
    if !(DISK_INNER_RADIUS..=DISK_OUTER_RADIUS).contains(&r) {
        return (Vec3::ZERO, 0.0);
    }

    let radial_t = (r - DISK_INNER_RADIUS) / (DISK_OUTER_RADIUS - DISK_INNER_RADIUS);
    let inner_fade = smoothstep(DISK_INNER_RADIUS, DISK_INNER_RADIUS + 0.28, r);
    let outer_fade = 1.0 - smoothstep(DISK_OUTER_RADIUS - 1.2, DISK_OUTER_RADIUS, r);

    let angle = hit.z.atan2(hit.x);
    let angular_vel = 2.4 / r.powf(1.25);
    let swirl = angle - time * angular_vel + 2.8 / (r + 0.4);
    let rings = 0.65 + 0.35 * (r * 9.5 + fbm2(swirl.cos() * 2.5, swirl.sin() * 2.5) * 3.0).sin();
    let turb = fbm2(r * 2.2, swirl * 1.4);

    let density = (inner_fade * outer_fade * (0.45 + 0.55 * turb) * rings).clamp(0.0, 1.0);

    // Relativistic Doppler beaming: disk rotates counter-clockwise in XZ plane.
    let tangent = Vec3::new(-hit.z / r, 0.0, hit.x / r);
    let beta = (0.52 / r.sqrt()).clamp(0.0, 0.55);
    let doppler = (1.0 / (1.0 - beta * tangent.dot(ray_dir))).clamp(0.55, 1.85);
    let beaming = doppler.powf(2.4);

    let hot = Vec3::new(1.6, 1.35, 1.05);
    let mid = Vec3::new(1.3, 0.58, 0.14);
    let cool = Vec3::new(0.45, 0.08, 0.03);
    let base_col = if radial_t < 0.35 {
        hot.lerp(mid, radial_t / 0.35)
    } else {
        mid.lerp(cool, (radial_t - 0.35) / 0.65)
    };

    let blue_shift = Vec3::new(
        1.0 + (doppler - 1.0) * 0.25,
        1.0,
        1.0 + (doppler - 1.0) * 0.55,
    );
    let emission = base_col.hadamard(blue_shift) * (1.75 * beaming);
    (emission, density)
}

/// Traces a single ray through curved spacetime around the black hole.
pub fn trace_blackhole(origin: Vec3, dir: Vec3, time: f32) -> Vec3 {
    let mut pos = origin;
    let mut vel = dir.normalize();
    let mut accum_color = Vec3::ZERO;
    let mut transmittance = 1.0_f32;
    let mut min_r = pos.length();

    for _ in 0..MAX_STEPS {
        let r2 = pos.length_sq();
        let r = r2.sqrt();
        min_r = min_r.min(r);

        if r <= SCHWARZSCHILD_RADIUS * 1.02 {
            // Absorbed by the event horizon; foreground disk emission remains visible.
            return accum_color;
        }
        if r2 > ESCAPE_RADIUS_SQ && pos.dot(vel) > 0.0 {
            break;
        }

        let (next_pos, next_vel) = step_geodesic(pos, vel, SCHWARZSCHILD_RADIUS, 0.11);

        // Exact equatorial plane crossing check for crisp, gap-free accretion disk.
        if (pos.y > 0.0 && next_pos.y <= 0.0) || (pos.y < 0.0 && next_pos.y >= 0.0) {
            let denom = (pos.y - next_pos.y).abs().max(1e-6);
            let frac = (pos.y.abs() / denom).clamp(0.0, 1.0);
            let hit = pos.lerp(next_pos, frac);
            let (disk_col, density) = shade_disk(hit, vel, time);
            let alpha = (density * 0.92).clamp(0.0, 0.96);
            accum_color = accum_color + disk_col * (transmittance * alpha);
            transmittance *= 1.0 - alpha;
            if transmittance < 0.02 {
                return accum_color;
            }
        }

        // Subtle volumetric glow near the photon sphere (r ~ 1.5 * r_s).
        if r < 2.2 {
            let photon_glow = (-((r - 1.48) * 5.0).powi(2)).exp() * 0.022;
            accum_color = accum_color + Vec3::new(1.1, 0.65, 0.32) * (transmittance * photon_glow);
        }

        pos = next_pos;
        vel = next_vel;
    }

    let bg = sample_starfield(vel);
    let ring_halo = (-((min_r - 1.52) * 3.8).powi(2)).exp() * 0.22;
    accum_color + (bg + Vec3::new(0.85, 0.55, 0.30) * ring_halo) * transmittance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_ray_has_zero_deflection() {
        let pos = Vec3::new(5.0, 0.0, 0.0);
        let vel = Vec3::new(-1.0, 0.0, 0.0);
        let acc = gravitational_acceleration(pos, vel, SCHWARZSCHILD_RADIUS);
        assert!(acc.length() < 1e-6);
    }

    #[test]
    fn grazing_ray_bends_toward_black_hole() {
        let pos = Vec3::new(0.0, 0.0, 6.0);
        let dir = Vec3::new(0.35, 0.0, -1.0).normalize();
        let mut p = pos;
        let mut d = dir;
        for _ in 0..25 {
            let (np, nd) = step_geodesic(p, d, SCHWARZSCHILD_RADIUS, 0.12);
            p = np;
            d = nd;
        }
        // Ray started with +x velocity at x=0, z>0; gravitational pull bends x back toward 0.
        assert!(
            d.x < dir.x,
            "expected ray to deflect toward origin: old x={}, new x={}",
            dir.x,
            d.x
        );
    }

    #[test]
    fn center_ray_hits_event_horizon_as_black() {
        let origin = Vec3::new(0.0, 0.25, 8.0);
        let dir = (Vec3::ZERO - origin).normalize();
        let col = trace_blackhole(origin, dir, 0.0);
        // Should be dark (near zero) since it plunges straight into the shadow.
        assert!(col.length() < 0.25, "expected shadow color, got {col:?}");
    }
}
