use glam::{vec3, Vec3};
use rand::Rng;

pub fn uniform_disk() -> (f32, f32) {
    let mut rng = rand::thread_rng();
    let mut x: f32;
    let mut y: f32;

    loop {
        x = 2.0 * rng.gen::<f32>() - 1.0;
        y = 2.0 * rng.gen::<f32>() - 1.0;

        if x * x + y * y <= 1.0 {
            return (x, y);
        }
    }
}

pub fn uniform_ball() -> Vec3 {
    let mut rng = rand::thread_rng();
    let mut x: f32;
    let mut y: f32;
    let mut z: f32;

    loop {
        x = 2.0 * rng.gen::<f32>() - 1.0;
        y = 2.0 * rng.gen::<f32>() - 1.0;
        z = 2.0 * rng.gen::<f32>() - 1.0;

        if x * x + y * y + z * z <= 1.0 {
            return vec3(x, y, z);
        }
    }
}

pub fn cos_weighted_hemisphere(normal: Vec3) -> Vec3 {
    let (x, y) = uniform_disk();
    let z = (1.0 - x * x - y * y).sqrt();
    let e1 = if normal.x != 0.0 {
        vec3(normal.y, -normal.x, 0.0).normalize()
    } else {
        vec3(0.0, -normal.z, normal.y).normalize()
    };
    let e2 = Vec3::cross(e1, normal);
    x * e1 + y * e2 + z * normal
}
