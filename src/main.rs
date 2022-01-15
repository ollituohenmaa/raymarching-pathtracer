mod sdf;

use sdf::*;
use std::f32::consts:: PI;
use std::fs::File;
use std::io::{prelude::*, BufWriter};
use std::time::Instant;
use glam::{Vec3, swizzles::Vec3Swizzles};
use rand::Rng;
use rayon::prelude::*;

const SAMPLE_COUNT: i32 = 50;
const SURFACE_DIST: f32 = 0.01;
const MAX_DIST: f32 = 100.0;
const MAX_STEPS: i32 = 100;
const MAX_BOUNCES: i32 = 5;
const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

struct HitInfo {
    position: Vec3,
    material: Material
}

mod sampling {
    use glam::Vec3;
    use rand::Rng;

    pub fn uniform_disk() -> [f32; 2] {
        let mut rng = rand::thread_rng();
        let mut x: f32;
        let mut y: f32;

        loop {
            x = 2.0 * rng.gen::<f32>() - 1.0;
            y = 2.0 * rng.gen::<f32>() - 1.0;

            if x * x + y * y <= 1.0 {
                return [x, y];
            }
        }
    }

    pub fn cos_weighted_hemisphere(normal: Vec3) -> Vec3 {
        let [x, y] = uniform_disk();
        let z = (1.0 - x * x - y * y).sqrt();
        let e1 = 
            if normal.x != 0.0 { Vec3::new(normal.y, -normal.x, 0.0).normalize() }
            else { Vec3::new(0.0, -normal.z, normal.y).normalize() };
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
            let [dx, dy] = sampling::uniform_disk();
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

struct Scene<A: Sdf> {
    camera: camera::Camera,
    sdf: A
}

fn get_normal(sdf: &impl Sdf, p: Vec3) -> Vec3 {
    let dx = Vec3::new(SURFACE_DIST, 0.0, 0.0);
    let dy = dx.yxy();
    let dz = dx.yyx();

    let x = sdf.dist(p + dx).distance - sdf.dist(p - dx).distance;
    let y = sdf.dist(p + dy).distance - sdf.dist(p - dy).distance;
    let z = sdf.dist(p + dz).distance - sdf.dist(p - dz).distance;

    Vec3::new(x, y, z).normalize()
}

fn get_intersection(sdf: &impl Sdf, origin: Vec3, ray: Vec3) -> HitInfo {
    let mut acc = 0.0;
    let mut steps = 0;
    let mut position;
    let mut distinfo;

    loop {
        position = origin + acc * ray;
        distinfo = sdf.dist(position);
        if distinfo.distance < SURFACE_DIST || acc > MAX_DIST || steps > MAX_STEPS {
            break;
        }
        acc += distinfo.distance;
        steps += 1;
    }

    HitInfo {
        position: position,
        material: distinfo.material
    }
}

fn cast_ray(sdf: &impl Sdf, mut origin: Vec3, mut ray: Vec3) -> Vec3 {
    let mut acc = Vec3::ONE;
    let mut bounces = 0;

    while bounces < MAX_BOUNCES {
        let hitinfo = get_intersection(sdf, origin, ray);

        match hitinfo.material {
            Material::Lambertian(color) => {
                acc = color * acc;
                let normal = get_normal(sdf, hitinfo.position);
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

fn render(width: i32, height: i32, scene: &Scene<impl Sdf>) -> Vec<Vec<Vec3>> {
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
            let pixel = MAX_PIXEL_VALUE * pixel.clamp(Vec3::ZERO, Vec3::ONE);
            writeln!(writer, "{:.0} {:.0} {:.0}", pixel.x, pixel.y, pixel.z)?;
        }
    }

    writer.flush()
}

fn main() {
    let camera = camera::Camera::new(
        Vec3::new(-8.0, -10.0, 6.5),
        Vec3::new(0.0, 0.0, 1.5),
        Vec3::Z,
        0.2 * PI,
        ASPECT_RATIO,
        14.0,
        0.5
    );

    let wall = Plane {
        normal: -Vec3::Y,
        point_in_plane: Vec3::new(0.0, 2.0, 0.0),
        material: Material::Lambertian(Vec3::new(0.1, 0.2, 0.3))
    };

    let cube1 = Cuboid {
        size: Vec3::splat(0.5),
        center: Vec3::new(-2.2, 0.0, 0.5),
        material: Material::Lambertian(Vec3::splat(0.2))
    };

    let cube2 = Cuboid {
        size: Vec3::splat(1.0),
        center: Vec3::new(0.0, 0.0, 1.0),
        material: Material::Lambertian(Vec3::splat(0.4))
    };

    let cube3 = Cuboid {
        size: Vec3::splat(1.5),
        center: Vec3::new(3.2, 0.0, 1.5),
        material: Material::Lambertian(Vec3::splat(0.6))
    };

    let ground = Plane {
        normal: Vec3::Z,
        point_in_plane: Vec3::ZERO,
        material: Material::Lambertian(Vec3::splat(0.5))
    };
    
    let sky = Plane {
        normal: -Vec3::Z,
        point_in_plane: Vec3::new(0.0, 0.0, 20.0),
        material: Material::Emissive(Vec3::splat(2.0))
    };

    let sdf = union(union(cube1, union(cube2, cube3)), union(wall, union(ground, sky)));

    let scene = Scene { camera, sdf };

    let now = Instant::now();
    let pixels = render(WIDTH, HEIGHT, &scene);
    match export_ppm("scene.ppm", &pixels) {
        Ok(()) => {},
        Err(error) => { println!("{}", error) }
    }
    println!("Rendering time: {:.1} s", now.elapsed().as_micros() as f32 / 1000_000.0);
}