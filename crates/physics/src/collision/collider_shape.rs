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
}

pub struct Sphere {
    pub radius: f32,
}
