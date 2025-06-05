use ecs::query::Query;
use essential::{time::Time, transform::Transform};

use crate::physics_body::PhysicsBody;

pub(crate) fn simulate_gravity(physics_bodies: Query<(&mut PhysicsBody, &mut Transform)>) {
    for (body, _) in physics_bodies.iter() {
        // Apply a constant downward force to simulate gravity
        let gravity_force = -9.81; // Gravity in m/s^2
        body.apply_force(glam::Vec3::new(0.0, gravity_force * body.mass, 0.0));
    }
}

pub(crate) fn update_physics_bodies(physics_bodies: Query<(&mut PhysicsBody, &mut Transform)>) {
    for (body, transform) in physics_bodies.iter() {
        let delta_time = Time::fixed_delta_time();

        // Update the physics body based on the elapsed time
        body.update(delta_time);

        // Update the transform position based on the velocity
        let velocity = body.get_velocity();
        transform.translation += velocity * delta_time;
    }
}
