use glam::Vec3;
use std::fs::File;
use std::io::{prelude::*, BufWriter};

const GAMMA_INV: f32 = 1.0 / 2.2;

fn gamma_encode(pixel: Vec3) -> Vec3 {
    pixel.clamp(Vec3::ZERO, Vec3::ONE).powf(GAMMA_INV)
}

pub fn export_ppm(path: &str, pixels: &Vec<Vec<Vec3>>) -> Result<(), std::io::Error> {
    const MAX_PIXEL_VALUE: f32 = 255.0;

    let width = pixels[0].len();
    let height = pixels.len();

    let file = File::create(path).unwrap();
    let mut writer = BufWriter::new(file);

    writeln!(
        writer,
        "P3\n{width} {height}\n{max_pixel_value}\n",
        width = width,
        height = height,
        max_pixel_value = MAX_PIXEL_VALUE
    )?;

    for row in pixels {
        for pixel in row {
            let pixel = MAX_PIXEL_VALUE * gamma_encode(*pixel);
            writeln!(writer, "{:.0} {:.0} {:.0}", pixel.x, pixel.y, pixel.z)?;
        }
    }

    writer.flush()
}
