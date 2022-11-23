mod camera;
mod ppm;
mod renderer;
mod sampling;
mod scene1;
mod sdf;

use std::time::Instant;

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const SAMPLE_COUNT: i32 = 50;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

fn main() {
    let now = Instant::now();
    let scene = scene1::create_scene(ASPECT_RATIO);
    let pixels = renderer::render(WIDTH, HEIGHT, SAMPLE_COUNT, &scene);

    match ppm::export_ppm("scene.ppm", &pixels) {
        Ok(()) => {}
        Err(error) => {
            println!("{}", error)
        }
    }

    println!(
        "Rendering time: {:.1} s",
        now.elapsed().as_micros() as f32 / 1_000_000.0
    );
}
