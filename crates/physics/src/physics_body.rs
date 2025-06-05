use ecs::component::Component;
use glam::Vec3;

#[derive(Component)]
pub struct PhysicsBody {
    pub mass: f32,
    velocity: Vec3,
    acceleration: Vec3,
}

impl PhysicsBody {
    pub fn new(mass: f32) -> Self {
        Self {
            mass,
            velocity: Vec3::ZERO,
            acceleration: Vec3::ZERO,
        }
    }

    pub fn apply_force(&mut self, force: Vec3) {
        self.acceleration += force / self.mass;
    }

    pub fn apply_impulse(&mut self, impulse: Vec3) {
        self.velocity += impulse / self.mass;
    }

    pub fn update(&mut self, delta_time: f32) {
        self.velocity += self.acceleration * delta_time;
        self.acceleration = Vec3::ZERO; // Reset acceleration after applying
    }

    pub fn get_velocity(&self) -> Vec3 {
        self.velocity
    }
}
