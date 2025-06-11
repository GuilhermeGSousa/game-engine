use app::plugins::Plugin;

use crate::{
    collision::contact::resolve_contacts,
    physics_server::PhysicsServer,
    simulation::{simulate_gravity, update_physics_bodies},
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.add_system(
            app::update_group::UpdateGroup::FixedUpdate,
            simulate_gravity,
        )
        .insert_resource(PhysicsServer::new())
        .add_system(
            app::update_group::UpdateGroup::LateFixedUpdate,
            resolve_contacts,
        )
        .add_system(
            app::update_group::UpdateGroup::LateFixedUpdate,
            update_physics_bodies,
        );
    }
}
