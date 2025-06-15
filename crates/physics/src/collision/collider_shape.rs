pub struct Aabb {}

pub enum CollisionShape {
    Sphere(Sphere),
}

impl CollisionShape {
    pub fn aabb(&self) -> Aabb {
        match self {
            _ => Aabb {},
        }
    }

    pub fn make_sphere(radius: f32) -> Self {
        Self::Sphere(Sphere { radius })
    }
}

pub struct Sphere {
    pub radius: f32,
}
