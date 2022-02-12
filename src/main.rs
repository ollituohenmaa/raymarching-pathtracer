mod sdf;
mod sampling;
mod camera;
mod renderer;
mod ppm;

use sdf::*;
use std::f32::consts:: PI;
use std::time::Instant;
use glam::{vec3, Vec3};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const SAMPLE_COUNT: i32 = 50;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

fn background_color(direction: Vec3) -> Vec3 {
    if direction.dot(vec3(1.0, 0.0, 0.5).normalize()) > 0.95 {
        15.0 * vec3(1.0, 0.85, 0.75)
    }
    else {
        0.5 * vec3(0.4, 0.7, 1.0)
    }
}

fn main() {
    let camera = camera::Camera::new(
        vec3(0.0, -12.0, 8.0),
        vec3(0.0, -1.0, 1.5),
        Vec3::Z,
        0.15 * PI,
        ASPECT_RATIO,
        0.1
    );

    let ground =
        plane(Vec3::Z)
        .subtract(
            sphere(0.04)
            .repeat(vec3(0.2, 0.2, 1.0))
            .rotate(Vec3::Z, 0.05 * PI)
        )
        .material(Material::Lambertian(Vec3::splat(0.6)));

    let frame = {
        let thickness = 0.9;

        cuboid(Vec3::ONE)
        .subtract(
            cuboid(vec3(thickness, 100.0, thickness))
            .union(cuboid(vec3(100.0, thickness, thickness)))
            .union(cuboid(vec3(thickness, thickness, 100.0)))
        )
        .rotate(Vec3::Z, -0.1 * PI)
        .position(vec3(-1.5, 0.0, 1.0))
        .material(Material::Lambertian(Vec3::splat(0.075)))
    };

    let tube =
        torus(1.5, 0.37)
        .shell(0.03)
        .subtract(
            plane(vec3(-1.0, 1.0, 0.0). normalize())
        )
        .position(vec3(2.0, 0.0, 0.4))
        .material(Material::Lambertian(Vec3::splat(0.3)));

    let map = ground.union(frame).union(tube);

    let scene = renderer::Scene { camera, map, background_color: Box::new(background_color) };

    let now = Instant::now();
    let pixels = renderer::render(WIDTH, HEIGHT, SAMPLE_COUNT, &scene);
    match ppm::export_ppm("scene.ppm", &pixels) {
        Ok(()) => {},
        Err(error) => { println!("{}", error) }
    }
    println!("Rendering time: {:.1} s", now.elapsed().as_micros() as f32 / 1000_000.0);
}