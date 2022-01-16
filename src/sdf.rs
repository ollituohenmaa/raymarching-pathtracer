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
    fn dist(&self, p: Vec3) -> f32;
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32
}

impl Sdf for Sphere {
    fn dist(&self, p: Vec3) -> f32 {
        (p - self.center).length() - self.radius
    }
}

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

pub struct Plane {
    pub point_in_plane: Vec3,
    pub normal: Vec3
}

impl Sdf for Plane {
    fn dist(&self, p: Vec3) -> f32 {
        self.normal.dot(p - self.point_in_plane)
    }
}

pub struct Union<A, B> {
    a: A,
    b: B
}

impl<A:Sdf, B: Sdf> Sdf for Union<A, B> {
    fn dist(&self, p: Vec3) -> f32 {
        self.a.dist(p).min(self.b.dist(p))
    }
}

pub fn union<A, B>(a: A, b: B) -> Union<A, B> {
    Union { a, b }
}

pub struct Difference<A, B> {
    a: A,
    b: B
}

impl<A: Sdf, B: Sdf> Sdf for Difference<A, B> {
    fn dist(&self, p: Vec3) -> f32 {
        self.a.dist(p).max(-self.b.dist(p))
    }
}

pub fn difference<A, B>(a: A, b: B) -> Difference<A, B> {
    Difference { a, b }
}

pub trait SdfMap: Sync {
    fn dist(&self, p: Vec3) -> DistInfo;
}

pub struct SdfWithMaterial<A> {
    sdf: A,
    material: Material
}

impl<A> SdfWithMaterial<A> {
    pub fn new(sdf: A, material: Material) -> SdfWithMaterial<A> {
        SdfWithMaterial { sdf, material }
    }
}

impl<A: Sdf> SdfMap for SdfWithMaterial<A> {
    fn dist(&self, p: Vec3) -> DistInfo {
        DistInfo {
            distance: self.sdf.dist(p),
            material: self.material
        }
    }
}

pub struct SdfMapUnion<A, B> {
    a: A,
    b: B
}

pub fn mapunion<A, B>(a: A, b: B) -> SdfMapUnion<A, B> {
    SdfMapUnion { a, b }
}

impl<A: SdfMap, B: SdfMap> SdfMap for SdfMapUnion<A, B> {
    fn dist(&self, p: Vec3) -> DistInfo {
        let a_dist = self.a.dist(p);
        let b_dist = self.b.dist(p);

        if a_dist.distance < b_dist.distance {
            a_dist
        }
        else {
            b_dist
        }
    }
}