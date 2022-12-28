use super::camera;
use super::renderer;
use super::sdf::*;

use glam::{vec3, Vec3};
use std::f32::consts::PI;

pub fn create_scene(aspect_ratio: f32) -> renderer::Scene {
    fn background_color(direction: Vec3) -> Vec3 {
        if direction.dot(vec3(1.0, 0.0, 0.5).normalize()) > 0.95 {
            15.0 * vec3(1.0, 0.85, 0.75)
        } else {
            0.5 * vec3(0.4, 0.7, 1.0)
        }
    }

    let camera = camera::Camera::new(
        vec3(0.0, -6.0, 4.0),
        vec3(0.0, -1.0, 1.5),
        Vec3::Z,
        0.15 * PI,
        aspect_ratio,
        0.075,
    );

    let ground = plane(Vec3::Z).material(Material::Lambertian {
        color: Vec3::splat(0.5),
    });

    let mandelbulb = Mandelbulb
        .rotate(Vec3::Z, 0.25 * PI)
        .position(Vec3::Z)
        .material(Material::Lambertian {
            color: Vec3::splat(0.25),
        });

    let map: Box<dyn SdfMap> = Box::new(ground.merge(mandelbulb));

    let background_color = Box::new(background_color);

    renderer::Scene {
        camera,
        map,
        background_color,
    }
}
