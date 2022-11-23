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
        vec3(0.0, -12.0, 8.0),
        vec3(0.0, -1.0, 1.5),
        Vec3::Z,
        0.15 * PI,
        aspect_ratio,
        0.1,
    );

    let ground = plane(Vec3::Z)
        .subtract(
            sphere(0.04)
                .repeat(vec3(0.2, 0.2, 1.0))
                .rotate(Vec3::Z, 0.05 * PI),
        )
        .material(Material::Lambertian {
            color: Vec3::splat(0.6),
        });

    let frame = {
        let thickness = 0.9;

        cuboid(Vec3::ONE)
            .subtract(
                cuboid(vec3(thickness, 100.0, thickness))
                    .merge(cuboid(vec3(100.0, thickness, thickness)))
                    .merge(cuboid(vec3(thickness, thickness, 100.0))),
            )
            .rotate(Vec3::Z, -0.1 * PI)
            .position(vec3(-1.5, 0.0, 1.0))
            .material(Material::Lambertian {
                color: Vec3::splat(0.075),
            })
    };

    let tube = torus(1.5, 0.37)
        .shell(0.03)
        .subtract(plane(vec3(-1.0, 1.0, 0.0).normalize()))
        .position(vec3(2.0, 0.0, 0.4))
        .material(Material::Lambertian {
            color: Vec3::splat(0.3),
        });

    let map: Box<dyn SdfMap> = Box::new(ground.merge(frame).merge(tube));

    let background_color = Box::new(background_color);

    renderer::Scene {
        camera,
        map,
        background_color,
    }
}
