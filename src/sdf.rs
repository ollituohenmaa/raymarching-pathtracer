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
        Eversion { a: *self }
    }

    fn repeat(&self, period: Vec3) -> Repeat<Self> {
        Repeat { a: *self, period }
    }

    fn position(&self, offset: Vec3) -> Translation<Self> {
        Translation { a: *self, offset }
    }

    fn union<Other>(&self, other: Other) -> Union<Self, Other> {
        Union { a: *self, b: other }
    }

    fn subtract<Other>(&self, other: Other) -> Difference<Self, Other> {
        Difference { a: *self, b: other }
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
pub struct Eversion<A> {
    a: A,
}

impl<A: Sdf> Sdf for Eversion<A> {
    fn dist(&self, p: Vec3) -> f32 {
        -self.a.dist(p)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Repeat<A> {
    a: A,
    period: Vec3
}

impl<A: Sdf> Sdf for Repeat<A> {
    fn dist(&self, p: Vec3) -> f32 {
        let p = p - self.period * (p / self.period + 0.5).floor();
        self.a.dist(p)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Translation<A> {
    a: A,
    offset: Vec3
}

impl<A: Sdf> Sdf for Translation<A> {
    fn dist(&self, p: Vec3) -> f32 {
        self.a.dist(p - self.offset)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Union<A, B> {
    a: A,
    b: B
}

impl<A: Sdf, B: Sdf> Sdf for Union<A, B> {
    fn dist(&self, p: Vec3) -> f32 {
        self.a.dist(p).min(self.b.dist(p))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Difference<A, B> {
    a: A,
    b: B
}

impl<A: Sdf, B: Sdf> Sdf for Difference<A, B> {
    fn dist(&self, p: Vec3) -> f32 {
        self.a.dist(p).max(-self.b.dist(p))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Shell<S> {
    sdf: S,
    thickness: f32
}

impl<A: Sdf> Sdf for Shell<A> {
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
            a: *self,
            b: other
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

impl<A: SdfMap, B: SdfMap> SdfMap for Union<A, B> {
    fn dist(&self, p: Vec3) -> f32 {
        self.a.dist(p).min(self.b.dist(p))
    }

    fn distinfo(&self, p: Vec3) -> DistInfo {
        let a_dist = self.a.distinfo(p);
        let b_dist = self.b.distinfo(p);

        if a_dist.distance < b_dist.distance {
            a_dist
        }
        else {
            b_dist
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SdfObject<A: Sdf> {
    sdf: A,
    material: Material
}

impl<A: Sdf> SdfMap for SdfObject<A> {
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