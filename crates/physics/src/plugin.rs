use app::plugins::Plugin;

use crate::simulation::{simulate_gravity, update_physics_bodies};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.add_system(
            app::update_group::UpdateGroup::FixedUpdate,
            simulate_gravity,
        );

        app.add_system(
            app::update_group::UpdateGroup::FixedUpdate,
            update_physics_bodies,
        );
    }
}
