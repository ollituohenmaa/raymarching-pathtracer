use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub enum Material {
    Lambertian(Vec3),
    Emissive(Vec3)
}

pub struct DistInfo {
    pub distance: f32,
    pub material: Material
}

pub trait Sdf: Sync {
    fn dist(&self, p: Vec3) -> DistInfo;
}

pub struct Cuboid {
    pub size: Vec3,
    pub center: Vec3,
    pub material: Material
}

impl Sdf for Cuboid {
    fn dist(&self, p: Vec3) -> DistInfo {
        let p = (p - self.center).abs() - self.size;
        DistInfo {
            distance: p.max(Vec3::ZERO).length() + p.x.max(p.y).max(p.z).min(0.0),
            material: self.material
        }
    }
}

pub struct Plane {
    pub point_in_plane: Vec3,
    pub normal: Vec3,
    pub material: Material
}

impl Sdf for Plane {
    fn dist(&self, p: Vec3) -> DistInfo {
        DistInfo {
            distance: self.normal.dot(p - self.point_in_plane),
            material: self.material
        }   
    }
}

pub struct Union<A, B> {
    a: A,
    b: B
}

impl<A, B> Sdf for Union<A, B> where A: Sdf, B: Sdf {
    fn dist(&self, p: Vec3) -> DistInfo {
        let a_dist = self.a.dist(p);
        let b_dist = self.b.dist(p);

        if a_dist.distance < b_dist.distance { a_dist } else { b_dist }
    }
}

pub fn union<A, B>(a: A, b: B) -> Union<A, B> {
    Union { a, b }
}