mod sdf;
mod camera;

use sdf::*;
use std::f32::consts:: PI;
use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::time::Instant;
use glam::{vec3, Vec3};
use rand::Rng;
use rayon::prelude::*;

const SAMPLE_COUNT: i32 = 100;
const MAX_BOUNCES: i32 = 5;
const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;
const GAMMA_INV: f32 = 1.0 / 2.2;

mod sampling {
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

    pub fn cos_weighted_hemisphere(normal: Vec3) -> Vec3 {
        let (x, y) = uniform_disk();
        let z = (1.0 - x * x - y * y).sqrt();
        let e1 = 
            if normal.x != 0.0 { vec3(normal.y, -normal.x, 0.0).normalize() }
            else { vec3(0.0, -normal.z, normal.y).normalize() };
        let e2 = Vec3::cross(e1, normal);
        x * e1 + y * e2 + z * normal
    }
}

struct Scene<A: SdfMap> {
    camera: camera::Camera,
    map: A,
    background_color: Box<dyn Fn(Vec3) -> Vec3 + Sync>
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

fn render(width: i32, height: i32, scene: &Scene<impl SdfMap>) -> Vec<Vec<Vec3>> {
    (0..height).into_par_iter().map(|i| {
        let mut rng = rand::thread_rng();
        (0..width).map(|j|
            (0..SAMPLE_COUNT).map(|_| {
                let x = -0.5 + (j as f32 + rng.gen::<f32>() - 0.5) / (width as f32 - 1.0);
                let y = 0.5 - (i as f32 + rng.gen::<f32>() - 0.5) / (height as f32 - 1.0);
                let ray = scene.camera.get_ray(x, y);
                cast_ray(scene, ray.origin, ray.direction)
            }).reduce(|u, v| u + v).unwrap() / SAMPLE_COUNT as f32
        ).collect()
    }).collect()
}

fn gamma_encode(pixel: Vec3) -> Vec3 {
    pixel.clamp(Vec3::ZERO, Vec3::ONE).powf(GAMMA_INV)
}

fn export_ppm(path: &str, pixels: &Vec<Vec<Vec3>>) -> Result<(), std::io::Error> {
    const MAX_PIXEL_VALUE: f32 = 255.0;

    let width = pixels[0].len();
    let height = pixels.len();

    let file = File::create(path).unwrap();
    let mut writer = BufWriter::new(file);

    writeln!(writer, "P3\n{width} {height}\n{max_pixel_value}\n",
        width = width, height = height, max_pixel_value = MAX_PIXEL_VALUE)?;

    for row in pixels {
        for pixel in row {
            let pixel = MAX_PIXEL_VALUE * gamma_encode(*pixel);
            writeln!(writer, "{:.0} {:.0} {:.0}", pixel.x, pixel.y, pixel.z)?;
        }
    }

    writer.flush()
}

fn background_color(direction: Vec3) -> Vec3 {
    if direction.dot(vec3(1.0, 1.0, 0.5)) > 0.95 {
        4.0 * vec3(1.0, 0.9, 0.8)
    }
    else {
        0.6 * vec3(0.4, 0.7, 1.0)
    }
}

fn main() {

    let camera = camera::Camera::new(
        vec3(0.0, -12.0, 8.0),
        vec3(0.0, 0.0, 1.0),
        Vec3::Z,
        0.2 * PI,
        ASPECT_RATIO,
        0.05
    );

    let ground =
        plane(Vec3::Z)
        .subtract(
            sphere(0.04)
            .repeat(vec3(0.2, 0.2, 1.0))
            .rotate(Vec3::Z, 0.05 * PI)
        )
        .material(Material::Lambertian(Vec3::splat(0.6)));
    
    let x_offset = 2.0;
    
    let frame = {
        let thickness = 0.9;

        cuboid(Vec3::ONE)
        .subtract(
            cuboid(vec3(thickness, 100.0, thickness))
            .union(cuboid(vec3(100.0, thickness, thickness)))
            .union(cuboid(vec3(thickness, thickness, 100.0)))
        )
        .rotate(Vec3::Z, -0.1 * PI)
        .position(vec3(-x_offset, 0.0, 1.0))
        .material(Material::Lambertian(Vec3::splat(0.1)))
    };

    let blob = {
        let size = 0.5;
        let roundness = 0.5;
        let radius = 0.7;

        sphere(radius)
        .position(vec3(size + x_offset, size, 2.0 * size + radius))
        .smooth_union(roundness,
            sphere(radius)
            .position(vec3(-size + x_offset, -size, 2.0 * size + radius))
        )
        .smooth_union(roundness,
            sphere(radius)
            .position(vec3(size + x_offset, -size, radius))
        )
        .smooth_union(roundness,
            sphere(radius)
            .position(vec3(-size + x_offset, size, radius))
        )
        .material(Material::Lambertian(Vec3::splat(0.3)))
    };

    let map = ground.union(frame).union(blob);

    let scene = Scene { camera, map, background_color: Box::new(background_color) };

    let now = Instant::now();
    let pixels = render(WIDTH, HEIGHT, &scene);
    match export_ppm("scene.ppm", &pixels) {
        Ok(()) => {},
        Err(error) => { println!("{}", error) }
    }
    println!("Rendering time: {:.1} s", now.elapsed().as_micros() as f32 / 1000_000.0);
}