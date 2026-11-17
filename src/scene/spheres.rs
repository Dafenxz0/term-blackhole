use crate::math::Vec3;

const MAX_MARCH_STEPS: usize = 96;
const MAX_DIST: f32 = 60.0;
const HIT_EPS: f32 = 0.0015;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaterialId {
    Floor,
    Chrome,
    Gold,
    Cobalt,
}

#[derive(Debug, Clone, Copy)]
struct Hit {
    dist: f32,
    mat: MaterialId,
}

#[inline]
pub fn sd_sphere(p: Vec3, center: Vec3, radius: f32) -> f32 {
    (p - center).length() - radius
}

#[inline]
pub fn sd_plane_y(p: Vec3, y_level: f32) -> f32 {
    p.y - y_level
}

fn scene_sdf(p: Vec3, time: f32) -> Hit {
    let mut best = Hit {
        dist: sd_plane_y(p, -1.2),
        mat: MaterialId::Floor,
    };

    let bob = (time * 1.4).sin() * 0.18;
    let orbit_x = (time * 0.9).cos() * 2.35;
    let orbit_z = (time * 0.9).sin() * 2.35;

    let candidates = [
        (Vec3::new(0.0, 0.15 + bob * 0.4, 0.0), 1.35, MaterialId::Chrome),
        (Vec3::new(orbit_x, -0.35 + bob, orbit_z), 0.78, MaterialId::Gold),
        (
            Vec3::new(-orbit_x * 0.85, -0.15 - bob * 0.6, -orbit_z * 0.85),
            0.88,
            MaterialId::Cobalt,
        ),
    ];

    for (center, radius, mat) in candidates {
        let d = sd_sphere(p, center, radius);
        if d < best.dist {
            best = Hit { dist: d, mat };
        }
    }

    best
}

fn calc_normal(p: Vec3, time: f32) -> Vec3 {
    let e = 0.001;
    let dx = scene_sdf(p + Vec3::new(e, 0.0, 0.0), time).dist
        - scene_sdf(p - Vec3::new(e, 0.0, 0.0), time).dist;
    let dy = scene_sdf(p + Vec3::new(0.0, e, 0.0), time).dist
        - scene_sdf(p - Vec3::new(0.0, e, 0.0), time).dist;
    let dz = scene_sdf(p + Vec3::new(0.0, 0.0, e), time).dist
        - scene_sdf(p - Vec3::new(0.0, 0.0, e), time).dist;
    Vec3::new(dx, dy, dz).normalize()
}

fn march(origin: Vec3, dir: Vec3, time: f32) -> Option<(f32, MaterialId)> {
    let mut t = 0.0;
    for _ in 0..MAX_MARCH_STEPS {
        let p = origin + dir * t;
        let hit = scene_sdf(p, time);
        if hit.dist < HIT_EPS {
            return Some((t, hit.mat));
        }
        t += hit.dist;
        if t > MAX_DIST {
            break;
        }
    }
    None
}

fn soft_shadow(origin: Vec3, light_dir: Vec3, max_t: f32, time: f32) -> f32 {
    let mut res = 1.0_f32;
    let mut t = 0.02;
    for _ in 0..40 {
        let h = scene_sdf(origin + light_dir * t, time).dist;
        if h < 0.001 {
            return 0.0;
        }
        res = res.min(12.0 * h / t);
        t += h.clamp(0.02, 0.35);
        if t > max_t {
            break;
        }
    }
    res.clamp(0.0, 1.0)
}

fn sky_color(dir: Vec3) -> Vec3 {
    let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0);
    let horizon = Vec3::new(0.14, 0.19, 0.28);
    let zenith = Vec3::new(0.02, 0.05, 0.14);
    horizon.lerp(zenith, t)
}

fn shade_surface(
    pos: Vec3,
    normal: Vec3,
    view_dir: Vec3,
    mat: MaterialId,
    light_pos: Vec3,
    time: f32,
) -> (Vec3, f32, Vec3) {
    let to_light = light_pos - pos;
    let light_dist = to_light.length();
    let light_dir = to_light / light_dist;

    let shadow = soft_shadow(pos + normal * 0.01, light_dir, light_dist, time);
    let diff = normal.dot(light_dir).max(0.0);
    let half_vec = (light_dir - view_dir).normalize();
    let spec_pow = match mat {
        MaterialId::Floor => 24.0,
        _ => 96.0,
    };
    let spec = normal.dot(half_vec).max(0.0).powf(spec_pow);
    let attenuation = 1.0 / (1.0 + 0.025 * light_dist * light_dist);

    let (albedo, reflectivity, tint) = match mat {
        MaterialId::Floor => {
            let cx = (pos.x * 1.1).floor() as i32;
            let cz = (pos.z * 1.1).floor() as i32;
            let checker = if (cx + cz) & 1 == 0 {
                Vec3::new(0.78, 0.80, 0.84)
            } else {
                Vec3::new(0.11, 0.13, 0.17)
            };
            (checker, 0.22, Vec3::ONE)
        }
        MaterialId::Chrome => (Vec3::new(0.75, 0.78, 0.85), 0.78, Vec3::new(0.95, 0.97, 1.0)),
        MaterialId::Gold => (Vec3::new(0.95, 0.70, 0.24), 0.68, Vec3::new(1.0, 0.84, 0.45)),
        MaterialId::Cobalt => (Vec3::new(0.20, 0.58, 0.95), 0.64, Vec3::new(0.55, 0.82, 1.0)),
    };

    let ambient = albedo * 0.14;
    let light_col = Vec3::new(1.8, 1.72, 1.55) * attenuation;
    let direct = (albedo * diff + Vec3::splat(spec * 0.9)).hadamard(light_col) * shadow;
    let ndotv = (-view_dir).dot(normal).max(0.0);
    let fresnel = reflectivity + (1.0 - reflectivity) * (1.0 - ndotv).powi(4);

    (ambient + direct, fresnel.clamp(0.0, 0.92), tint)
}

/// Renders the reflective metallic spheres scene with 2-bounce reflections and soft shadows.
pub fn trace_spheres(origin: Vec3, dir: Vec3, time: f32) -> Vec3 {
    let light_pos = Vec3::new((time * 0.5).cos() * 4.5, 5.2, (time * 0.5).sin() * 4.5);
    let mut accum = Vec3::ZERO;
    let mut weight = Vec3::ONE;
    let mut ray_origin = origin;
    let mut ray_dir = dir.normalize();

    for bounce in 0..3 {
        if let Some((t, mat)) = march(ray_origin, ray_dir, time) {
            let pos = ray_origin + ray_dir * t;
            let normal = calc_normal(pos, time);
            let (local_col, refl, tint) = shade_surface(pos, normal, ray_dir, mat, light_pos, time);

            let fog = (-t * 0.018).exp();
            let sky = sky_color(ray_dir);
            let shaded = sky.lerp(local_col, fog);

            accum = accum + weight.hadamard(shaded * (1.0 - refl));
            if bounce == 2 || refl < 0.05 {
                break;
            }
            weight = weight.hadamard(tint * (refl * fog));
            ray_origin = pos + normal * 0.015;
            ray_dir = ray_dir.reflect(normal).normalize();
        } else {
            accum = accum + weight.hadamard(sky_color(ray_dir));
            break;
        }
    }

    accum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdf_sphere_and_plane_distances() {
        let s = sd_sphere(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, 1.0);
        assert!((s - 2.0).abs() < 1e-6);
        assert!((sd_sphere(Vec3::ZERO, Vec3::ZERO, 1.5) + 1.5).abs() < 1e-6);

        let p = sd_plane_y(Vec3::new(4.0, 0.8, -2.0), -1.2);
        assert!((p - 2.0).abs() < 1e-6);
    }

    #[test]
    fn sphere_ray_hits_central_sphere() {
        let hit = march(Vec3::new(0.0, 0.15, 6.0), Vec3::new(0.0, 0.0, -1.0), 0.0);
        assert!(hit.is_some());
        let (t, mat) = hit.unwrap();
        assert!((t - 4.65).abs() < 0.1, "unexpected hit distance {t}");
        assert_eq!(mat, MaterialId::Chrome);
    }
}
