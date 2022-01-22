mod sdf;

use sdf::*;
use std::f32::consts:: PI;
use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::time::Instant;
use glam::{vec3, Vec3};
use rand::Rng;
use rayon::prelude::*;

const SAMPLE_COUNT: i32 = 1000;
const MAX_DIST: f32 = 100.0;
const MAX_STEPS: i32 = 100;
const MAX_BOUNCES: i32 = 5;
const WIDTH: i32 = 480;
const HEIGHT: i32 = 640;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;
const GAMMA_INV: f32 = 1.0 / 2.2;

struct HitInfo {
    position: Vec3,
    material: Material
}

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

mod camera {
    use glam::Vec3;
    use super::sampling;

    pub struct Ray {
        pub origin: Vec3,
        pub direction: Vec3
    }

    pub struct Camera {
        position: Vec3,
        left: Vec3,
        forward: Vec3,
        up: Vec3,
        focal_length: f32,
        aspect_ratio: f32,
        focus_dist: f32,
        aperture: f32
    }
    
    impl Camera {
        pub fn new(
            position: Vec3,
            look_at: Vec3,
            up: Vec3,
            angle_of_view: f32,
            aspect_ratio: f32,
            focus_dist: f32,
            aperture: f32
        ) -> Self {
            let focal_length = 0.5 / (0.5 * angle_of_view).tan();
            let forward = (look_at - position).normalize();
            let left = forward.cross(up).normalize();
            let up = left.cross(forward);
            Self { position, left, forward, up, focal_length, aspect_ratio, focus_dist, aperture }
        }

        pub fn get_ray(&self, x: f32, y: f32) -> Ray {
            let (dx, dy) = sampling::uniform_disk();
            let offset = 0.5 * self.aperture * (dx * self.left + dy * self.up);

            let origin = self.position + offset;

            let direction = (self.focus_dist * (
                x / self.focal_length * self.left +
                y / (self.focal_length * self.aspect_ratio) * self.up +
                self.forward
            ) - offset).normalize();

            Ray { origin, direction }
        }
    }
}

struct Scene<A: SdfMap> {
    camera: camera::Camera,
    sdf: A
}

fn get_intersection(sdf: &impl SdfMap, origin: Vec3, ray: Vec3) -> HitInfo {
    let mut acc = 0.0;
    let mut steps = 0;
    let mut position;
    let mut dist;

    loop {
        position = origin + acc * ray;
        dist = sdf.dist(position);
        acc += dist;
        steps += 1;
        if dist < SURFACE_DIST || acc > MAX_DIST || steps > MAX_STEPS {
            break;
        }
    }

    HitInfo {
        position: origin + acc * ray,
        material: sdf.distinfo(origin + acc * ray).material
    }
}

fn cast_ray(sdf: &impl SdfMap, mut origin: Vec3, mut ray: Vec3) -> Vec3 {
    let mut acc = Vec3::ONE;
    let mut bounces = 0;

    loop {
        if bounces >= MAX_BOUNCES {
            acc = Vec3::ZERO;
            break;
        }

        let hitinfo = get_intersection(sdf, origin, ray);

        match hitinfo.material {
            Material::Lambertian(color) => {
                acc = color * acc;
                let normal = sdf.normal(hitinfo.position);
                origin = hitinfo.position + 1.1 * SURFACE_DIST * normal;
                ray = sampling::cos_weighted_hemisphere(normal);
            },
            Material::Emissive(color) => {
                acc = color * acc;
                break;
            }
        }

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
                cast_ray(&scene.sdf, ray.origin, ray.direction)
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

fn main() {

    let lamp_height = 2.502;

    let camera_position = vec3(-8.0, -6.0, 6.0);
    let camera = camera::Camera::new(
        camera_position,
        vec3(0.0, 0.0, 1.5),
        Vec3::Z,
        0.12 * PI,
        ASPECT_RATIO,
        (camera_position - vec3(-0.5, -0.5, lamp_height)).length(),
        0.05
    );

    let cube = Cuboid {
        size: Vec3::splat(1.0),
        center: vec3(0.0, 0.0, 1.0)
    };

    let clipper = (Cuboid {
        size: vec3(0.9, f32::INFINITY, 1.0),
        center: vec3(0.0, 0.0, 0.9)
    }).union(Cuboid {
        size: vec3(f32::INFINITY, 0.9, 1.0),
        center: vec3(0.0, 0.0, 0.9)
    });

    let table = SdfWithMaterial::new(
        cube.difference(clipper),
        Material::Lambertian(Vec3::splat(0.95))
    );

    let lamp = SdfWithMaterial::new(
        (Sphere { center: vec3(0.0, 0.0, lamp_height), radius: 0.5 })
            .shell(0.02)
            .difference(Cuboid { size: vec3(1.0, 1.0, 0.03), center: lamp_height * Vec3::Z })
            .difference(Cuboid { size: vec3(0.03, 1.0, 1.0), center: lamp_height * Vec3::Z })
            .difference(Cuboid { size: vec3(1.0, 0.03, 1.0), center: lamp_height * Vec3::Z }),
        Material::Lambertian(Vec3::splat(0.4))
    ).union(SdfWithMaterial::new(Sphere {
        center: vec3(0.0, 0.0, lamp_height),
        radius: 0.4,
    }, Material::Emissive(4.0 * vec3(1.0, 0.3, 0.1))));

    let floor = SdfWithMaterial::new(Plane {
        normal: Vec3::Z,
        point_in_plane: Vec3::ZERO
    }, Material::Lambertian(Vec3::splat(0.5)));

    let left_wall = SdfWithMaterial::new(Plane {
        normal: -Vec3::Y,
        point_in_plane: vec3(0.0, 1.3, 0.0)
    }, Material::Lambertian(Vec3::splat(0.4)));

    let right_wall = SdfWithMaterial::new(Plane {
        normal: -Vec3::X,
        point_in_plane: vec3(1.3, 0.0, 0.0)
    }, Material::Lambertian(Vec3::splat(0.8)));

    let window = SdfWithMaterial::new(Cuboid {
        size: vec3(4.0, 4.0, 0.0),
        center: vec3(-8.0, -4.0, 8.0)
    }, Material::Emissive(0.5 * vec3(0.25, 0.5, 0.75)));

    let sdf = window
        .union(floor)
        .union(left_wall)
        .union(right_wall)
        .union(table)
        .union(lamp);

    let scene = Scene { camera, sdf };

    let now = Instant::now();
    let pixels = render(WIDTH, HEIGHT, &scene);
    match export_ppm("scene.ppm", &pixels) {
        Ok(()) => {},
        Err(error) => { println!("{}", error) }
    }
    println!("Rendering time: {:.1} s", now.elapsed().as_micros() as f32 / 1000_000.0);
}