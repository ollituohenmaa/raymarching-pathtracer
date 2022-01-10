mod sdf;

use sdf::*;
use std::f32::consts::{TAU, PI};
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

mod camera {
    use glam::Vec3;

    pub struct Camera {
        pub position: Vec3,
        left: Vec3,
        forward: Vec3,
        up: Vec3,
        focal_length: f32,
        aspect_ratio: f32
    }
    
    impl Camera {
        pub fn new(position: Vec3, look_at: Vec3, up: Vec3, angle_of_view: f32, aspect_ratio: f32) -> Self {
            let forward = (look_at - position).normalize();
            let left = forward.cross(up).normalize();
            Self {
                position,
                left,
                forward,
                up: left.cross(forward),
                focal_length: 0.5 / (0.5 * angle_of_view).tan(),
                aspect_ratio
            }
        }

        pub fn get_camera_ray(&self, x: f32, y: f32) -> Vec3 {
            x * self.left + y / self.aspect_ratio * self.up + self.focal_length * self.forward
        }
    }
}

struct Scene<A> where A: Sdf {
    camera: camera::Camera,
    sdf: A
}

fn get_normal<A>(sdf: &A, p: Vec3) -> Vec3 where A: Sdf {
    let dx = Vec3::new(SURFACE_DIST, 0.0, 0.0);
    let dy = dx.yxy();
    let dz = dx.yyx();

    let x = sdf.dist(p + dx).distance - sdf.dist(p - dx).distance;
    let y = sdf.dist(p + dy).distance - sdf.dist(p - dy).distance;
    let z = sdf.dist(p + dz).distance - sdf.dist(p - dz).distance;

    Vec3::new(x, y, z).normalize()
}

fn get_intersection<A>(sdf: &A, origin: Vec3, ray: Vec3) -> HitInfo where A: Sdf {
    let ray = ray.normalize();
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

fn cos_weighted_hemi_sample(rng: &mut rand::prelude::ThreadRng, normal: Vec3) -> Vec3 {
    let u: f32 = rng.gen();
    let v: f32 = rng.gen();
    let r = u.sqrt();
    let (sin_phi, cos_phi) = (TAU * v).sin_cos();
    let e1 = 
        if normal.x != 0.0 { Vec3::new(normal.y, -normal.x, 0.0).normalize() }
        else { Vec3::new(0.0, -normal.z, normal.y).normalize() };
    let e2 = Vec3::cross(e1, normal);
    r * (cos_phi * e1 + sin_phi * e2) + (1.0 - u).sqrt() * normal
}

fn cast_ray<A>(rng: &mut rand::prelude::ThreadRng, sdf: &A, origin: Vec3, ray: Vec3) -> Vec3 where A: Sdf {
    let mut origin = origin;
    let mut ray = ray;
    let mut acc = Vec3::ONE;
    let mut bounces = 0;

    while bounces < MAX_BOUNCES {
        let hitinfo = get_intersection(sdf, origin, ray);

        match hitinfo.material {
            Material::Lambertian(color) => {
                acc = color * acc;
                let normal = get_normal(sdf, hitinfo.position);
                origin = hitinfo.position + 1.1 * SURFACE_DIST * normal;
                ray = cos_weighted_hemi_sample(rng, normal);
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

fn render<A>(width: i32, height: i32, scene: &Scene<A>) -> Vec<Vec<Vec3>> where A: Sdf {
    (0..height).into_par_iter().map(|i| {
        let mut rng = rand::thread_rng();
        (0..width).map(|j|
            (0..SAMPLE_COUNT).map(|_| {
                let x = -0.5 + (j as f32 + rng.gen::<f32>() - 0.5) / (width as f32 - 1.0);
                let y = 0.5 - (i as f32 + rng.gen::<f32>() - 0.5) / (height as f32 - 1.0);
                let ray = scene.camera.get_camera_ray(x, y);
                cast_ray(&mut rng, &scene.sdf, scene.camera.position, ray)
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
        Vec3::new(0.8, 0.0, 1.5),
        Vec3::Z,
        0.2 * PI,
        ASPECT_RATIO
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