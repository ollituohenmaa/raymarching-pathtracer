use glam::{vec3, Vec3, swizzles::Vec3Swizzles};

pub const SURFACE_DIST: f32 = 0.01;

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

    fn union<Other>(&self, other: Other) -> Union<Self, Other> {
        Union { a: *self, b: other }
    }

    fn difference<Other>(&self, other: Other) -> Difference<Self, Other> {
        Difference { a: *self, b: other }
    }

    fn shell(&self, thickness: f32) -> Shell<Self> {
        Shell { sdf: *self, thickness }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32
}

impl Sdf for Sphere {
    fn dist(&self, p: Vec3) -> f32 {
        (p - self.center).length() - self.radius
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Cuboid {
    pub size: Vec3,
    pub center: Vec3
}

impl Sdf for Cuboid {
    fn dist(&self, p: Vec3) -> f32 {
        let p = (p - self.center).abs() - self.size;
        p.max(Vec3::ZERO).length() + p.x.max(p.y).max(p.z).min(0.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub point_in_plane: Vec3,
    pub normal: Vec3
}

impl Sdf for Plane {
    fn dist(&self, p: Vec3) -> f32 {
        self.normal.dot(p - self.point_in_plane)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Union<A, B> {
    a: A,
    b: B
}

impl<A:Sdf, B: Sdf> Sdf for Union<A, B> {
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

    fn union<Other>(&self, other: Other) -> SdfMapUnion<Self, Other> {
        SdfMapUnion {
            a: *self,
            b: other
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SdfMapUnion<A, B> {
    a: A,
    b: B
}

impl<A: SdfMap, B: SdfMap> SdfMap for SdfMapUnion<A, B> {
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
pub struct SdfWithMaterial<A: Copy> {
    sdf: A,
    material: Material
}

impl<A: Copy> SdfWithMaterial<A> {
    pub fn new(sdf: A, material: Material) -> SdfWithMaterial<A> {
        SdfWithMaterial { sdf, material }
    }
}

impl<A: Sdf + Copy> SdfMap for SdfWithMaterial<A> {
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