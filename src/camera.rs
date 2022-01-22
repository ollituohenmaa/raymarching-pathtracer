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