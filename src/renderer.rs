use super::sdf::*;
use super::camera::*;
use super::sampling;
use glam::{Vec3};
use rand::Rng;
use rayon::prelude::*;

const MAX_BOUNCES: i32 = 5;

pub struct Scene<A: SdfMap> {
    pub camera: Camera,
    pub map: A,
    pub background_color: Box<dyn Fn(Vec3) -> Vec3 + Sync>
}

fn cast_ray(scene: &Scene<impl SdfMap>, mut origin: Vec3, mut direction: Vec3) -> Vec3 {
    let mut acc = Vec3::ONE;
    let mut bounces = 0;

    loop {
        if bounces >= MAX_BOUNCES {
            acc = Vec3::ZERO;
            break;
        }

        match scene.map.ray_intersection(origin, direction) {
            Some(hitinfo) => {
                match hitinfo.material {
                    Material::Lambertian(color) => {
                        acc = color * acc;
                        let normal = scene.map.normal(hitinfo.position);
                        origin = hitinfo.position + 2.0 * SURFACE_DIST * normal;
                        direction = sampling::cos_weighted_hemisphere(normal);
                    },
                    Material::Emissive(color) => {
                        acc = color * acc;
                        break;
                    }
                }
            },
            None => {
                acc = (scene.background_color)(direction) * acc;
                break;
            }
        };

        bounces = bounces + 1;
    }

    acc
}

pub fn render(width: i32, height: i32, sample_count: i32, scene: &Scene<impl SdfMap>) -> Vec<Vec<Vec3>> {
    (0..height).into_par_iter().map(|i| {
        let mut rng = rand::thread_rng();
        (0..width).map(|j|
            (0..sample_count).map(|_| {
                let x = -0.5 + (j as f32 + rng.gen::<f32>() - 0.5) / (width as f32 - 1.0);
                let y = 0.5 - (i as f32 + rng.gen::<f32>() - 0.5) / (height as f32 - 1.0);
                let ray = scene.camera.get_ray(x, y);
                cast_ray(scene, ray.origin, ray.direction)
            }).reduce(|u, v| u + v).unwrap() / sample_count as f32
        ).collect()
    }).collect()
}