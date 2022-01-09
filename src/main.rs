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
const WIDTH: i32 = 640;
const HEIGHT: i32 = 480;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

enum Material {
    Lambertian(Vec3),
    Emissive(Vec3)
}

struct HitInfo {
    position: Vec3,
    material: Material
}

struct DistInfo {
    distance: f32,
    material: Material
}

mod camera {
    use glam::Vec3;

    pub struct Camera {
        pub position: Vec3,
        left: Vec3,
        forward: Vec3,
        up: Vec3,
        focal_length: f32
    }
    
    impl Camera {
        pub fn new(position: Vec3, look_at: Vec3, up: Vec3, angle_of_view: f32) -> Self {
            let forward = (look_at - position).normalize();
            let left = forward.cross(up).normalize();
            Self {
                position: position,
                left: left,
                forward: forward,
                up: left.cross(forward),
                focal_length: 0.5 / (0.5 * angle_of_view).tan()
            }
        }

        pub fn get_camera_ray(&self, x: f32, y: f32) -> Vec3 {
            x * self.left + y * self.up + self.focal_length * self.forward
        }
    }
}

fn plane(point_in_plane: Vec3, normal: Vec3, p: Vec3) -> f32 {
    normal.dot(p - point_in_plane)
}

fn cuboid(size: Vec3, center: Vec3, p: Vec3) -> f32 {
    let q = (p - center).abs() - size;
    q.max(Vec3::ZERO).length() + (q.x.max(q.y.max(q.z))).min(0.0)
}

fn sdf(p: Vec3) -> DistInfo {
    let object_dist = cuboid(Vec3::ONE, Vec3::new(0.0, 0.0, 1.0), p);
    let ground_dist = plane(Vec3::ZERO, Vec3::Z, p);
    let wall_dist = plane(Vec3::new(0.0, 2.0, 0.0), -Vec3::Y, p);
    let sky_dist = plane(Vec3::new(0.0, 0.0, 20.0), -Vec3::Z, p);

    if wall_dist < ground_dist && wall_dist < object_dist && wall_dist < sky_dist {
        DistInfo {
            distance: wall_dist,
            material: Material::Lambertian(Vec3::new(0.1, 0.2, 0.3))
        }
    }
    else if object_dist < ground_dist && object_dist < sky_dist {
        DistInfo {
            distance: object_dist,
            material: Material::Lambertian(Vec3::new(0.2, 0.2, 0.2))
        }
    }
    else if ground_dist < sky_dist {
        DistInfo {
            distance: ground_dist,
            material: Material::Lambertian(Vec3::new(0.5, 0.5, 0.5))
        }
    }
    else {
        DistInfo {
            distance: sky_dist,
            material: Material::Emissive(Vec3::splat(2.0))
        }
    }
}

fn get_normal(p: Vec3) -> Vec3 {
    let dx = Vec3::new(SURFACE_DIST, 0.0, 0.0);
    let dy = dx.yxy();
    let dz = dx.yyx();

    let x = sdf(p + dx).distance - sdf(p - dx).distance;
    let y = sdf(p + dy).distance - sdf(p - dy).distance;
    let z = sdf(p + dz).distance - sdf(p - dz).distance;

    Vec3::new(x, y, z).normalize()
}

fn get_intersection(origin: Vec3, ray: Vec3) -> HitInfo {
    let ray = ray.normalize();
    let mut acc = 0.0;
    let mut steps = 0;
    let mut position;
    let mut distinfo;

    loop {
        position = origin + acc * ray;
        distinfo = sdf(position);
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

fn cos_weighted_hemi_sample(normal: Vec3) -> Vec3 {
    let mut rng = rand::thread_rng();
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

fn cast_ray(origin: Vec3, ray: Vec3) -> Vec3 {
    if rand::random::<f32>() < 0.05 {
        Vec3::ZERO
    }
    else {
        let hitinfo = get_intersection(origin, ray);

        match hitinfo.material {
            Material::Lambertian(color) => {
                let normal = get_normal(hitinfo.position);
                let origin = hitinfo.position + 1.1 * SURFACE_DIST * normal;
                let ray = cos_weighted_hemi_sample(normal);
                color * cast_ray(origin, ray)
            },
            Material::Emissive(color) => {
                color
            }
        }
    }
}

fn render(camera: &camera::Camera) -> Vec<Vec<Vec3>> {
    (0..HEIGHT).into_par_iter().map(|i| {
        let mut rng = rand::thread_rng();
        (0..WIDTH).map(|j|
            (0..SAMPLE_COUNT).map(|_| {
                let x = -0.5 + (j as f32 + rng.gen::<f32>() - 0.5) / (WIDTH as f32 - 1.0);
                let y = (0.5 - (i as f32 + rng.gen::<f32>() - 0.5) / (HEIGHT as f32 - 1.0)) / ASPECT_RATIO;
                let ray = camera.get_camera_ray(x, y);
                cast_ray(camera.position, ray)
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
        Vec3::new(-8.0, -8.0, 6.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::Z,
        0.2 * PI
    );

    let now = Instant::now();
    let pixels = render(&camera);
    match export_ppm("scene.ppm", &pixels) {
        Ok(()) => {},
        Err(error) => { println!("{}", error) }
    }
    println!("Rendering time: {:.1} s", now.elapsed().as_micros() as f32 / 1000_000.0);
}