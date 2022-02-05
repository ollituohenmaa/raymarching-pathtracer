use glam::{vec3, Vec3, swizzles::Vec3Swizzles};

pub const SURFACE_DIST: f32 = 0.01;
const MAX_DIST: f32 = 100.0;
const MAX_STEPS: i32 = 100;

#[derive(Clone, Copy, Debug)]
pub enum Material {
    Lambertian(Vec3),
    Emissive(Vec3)
}

pub struct DistInfo {
    pub distance: f32,
    pub material: Material
}

pub trait Sdf: Sync + Copy {
    fn dist(&self, p: Vec3) -> f32;

    fn evert(&self) -> Eversion<Self> {
        Eversion { sdf: *self }
    }

    fn repeat(&self, period: Vec3) -> Repeat<Self> {
        Repeat { sdf: *self, period }
    }

    fn position(&self, offset: Vec3) -> Translation<Self> {
        Translation { sdf: *self, offset }
    }

    fn union<Other>(&self, other: Other) -> Union<Self, Other> {
        Union { sdf1: *self, sdf2: other }
    }

    fn subtract<Other>(&self, other: Other) -> Difference<Self, Other> {
        Difference { sdf1: *self, sdf2: other }
    }

    fn shell(&self, thickness: f32) -> Shell<Self> {
        Shell { sdf: *self, thickness }
    }

    fn material(&self, material: Material) -> SdfObject<Self> {
        SdfObject { sdf: *self, material }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    pub radius: f32
}

impl Sdf for Sphere {
    fn dist(&self, p: Vec3) -> f32 {
        p.length() - self.radius
    }
}

pub fn sphere(radius: f32) ->  Sphere {
    Sphere { radius }
}

#[derive(Clone, Copy, Debug)]
pub struct Cuboid {
    pub dimensions: Vec3
}

impl Sdf for Cuboid {
    fn dist(&self, p: Vec3) -> f32 {
        let p = p.abs() - self.dimensions;
        p.max(Vec3::ZERO).length() + p.x.max(p.y).max(p.z).min(0.0)
    }
}

pub fn cuboid(dimensions: Vec3) ->  Cuboid {
    Cuboid { dimensions }
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3
}

impl Sdf for Plane {
    fn dist(&self, p: Vec3) -> f32 {
        self.normal.dot(p)
    }
}

pub fn plane(normal: Vec3) ->  Plane {
    Plane { normal }
}

#[derive(Clone, Copy, Debug)]
pub struct Eversion<S> {
    sdf: S,
}

impl<S: Sdf> Sdf for Eversion<S> {
    fn dist(&self, p: Vec3) -> f32 {
        -self.sdf.dist(p)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Repeat<S> {
    sdf: S,
    period: Vec3
}

impl<S: Sdf> Sdf for Repeat<S> {
    fn dist(&self, p: Vec3) -> f32 {
        let p = p - self.period * (p / self.period + 0.5).floor();
        self.sdf.dist(p)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Translation<S> {
    sdf: S,
    offset: Vec3
}

impl<S: Sdf> Sdf for Translation<S> {
    fn dist(&self, p: Vec3) -> f32 {
        self.sdf.dist(p - self.offset)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Union<S1, S2> {
    sdf1: S1,
    sdf2: S2
}

impl<S1: Sdf, S2: Sdf> Sdf for Union<S1, S2> {
    fn dist(&self, p: Vec3) -> f32 {
        self.sdf1.dist(p).min(self.sdf2.dist(p))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Difference<S1, S2> {
    sdf1: S1,
    sdf2: S2
}

impl<S1: Sdf, S2: Sdf> Sdf for Difference<S1, S2> {
    fn dist(&self, p: Vec3) -> f32 {
        self.sdf1.dist(p).max(-self.sdf2.dist(p))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Shell<S> {
    sdf: S,
    thickness: f32
}

impl<S: Sdf> Sdf for Shell<S> {
    fn dist(&self, p: Vec3) -> f32 {
        self.sdf.dist(p).abs() - self.thickness
    }
}

pub struct HitInfo {
    pub position: Vec3,
    pub material: Material
}

pub trait SdfMap: Sync + Copy {
    fn dist(&self, p: Vec3) -> f32;

    fn distinfo(&self, p: Vec3) -> DistInfo;

    fn normal(&self, p: Vec3) -> Vec3 {
        let dx = vec3(SURFACE_DIST, 0.0, 0.0);
        let dy = dx.yxy();
        let dz = dx.yyx();
    
        let x = self.dist(p + dx) - self.dist(p - dx);
        let y = self.dist(p + dy) - self.dist(p - dy);
        let z = self.dist(p + dz) - self.dist(p - dz);
    
        vec3(x, y, z).normalize()
    }

    fn union<Other>(&self, other: Other) -> Union<Self, Other> {
        Union {
            sdf1: *self,
            sdf2: other
        }
    }
    
    fn ray_intersection(&self, origin: Vec3, ray: Vec3) -> HitInfo {
        let mut acc = 0.0;
        let mut steps = 0;
        let mut position;
        let mut dist;
    
        loop {
            position = origin + acc * ray;
            dist = self.dist(position);
            acc += dist;
            steps += 1;
            if dist < SURFACE_DIST || acc > MAX_DIST || steps > MAX_STEPS {
                break;
            }
        }
    
        HitInfo {
            position: origin + acc * ray,
            material: self.distinfo(origin + acc * ray).material
        }
    }
}

impl<S1: SdfMap, S2: SdfMap> SdfMap for Union<S1, S2> {
    fn dist(&self, p: Vec3) -> f32 {
        self.sdf1.dist(p).min(self.sdf2.dist(p))
    }

    fn distinfo(&self, p: Vec3) -> DistInfo {
        let distinfo1 = self.sdf1.distinfo(p);
        let distinfo2 = self.sdf2.distinfo(p);

        if distinfo1.distance < distinfo2.distance {
            distinfo1
        }
        else {
            distinfo2
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SdfObject<S: Sdf> {
    sdf: S,
    material: Material
}

impl<S: Sdf> SdfMap for SdfObject<S> {
    fn dist(&self, p: Vec3) -> f32 {
        self.sdf.dist(p)
    }

    fn distinfo(&self, p: Vec3) -> DistInfo {
        DistInfo {
            distance: self.sdf.dist(p),
            material: self.material
        }
    }
}