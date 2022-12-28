mod camera;
mod ppm;
mod renderer;
mod sampling;
mod scene1;
mod scene2;
mod sdf;

use std::env;
use std::time::Instant;

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const SAMPLE_COUNT: i32 = 100;
const ASPECT_RATIO: f32 = WIDTH as f32 / HEIGHT as f32;

fn main() {
    let args: Vec<String> = env::args().collect();

    let now = Instant::now();

    let scene_name = &(args[1])[..];

    let scene = match scene_name {
        "scene1" => scene1::create_scene(ASPECT_RATIO),
        "scene2" => scene2::create_scene(ASPECT_RATIO),
        _ => panic!("Scene \"{}\" not found.", scene_name),
    };

    let pixels = renderer::render(WIDTH, HEIGHT, SAMPLE_COUNT, &scene);

    let output_path = format!("{}.ppm", scene_name);

    match ppm::export_ppm(output_path.as_str(), &pixels) {
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
