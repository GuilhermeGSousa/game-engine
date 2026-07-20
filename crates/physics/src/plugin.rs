use app::plugins::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::{
    collider::{register_colliders, Collider},
    ground::probe_ground,
    interpolation::interpolate_body_transforms,
    movement::apply_character_movement,
    physics_pipeline::PhysicsPipeline,
    physics_state::PhysicsState,
    simulation::step_simulation,
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_component_lifecycle::<Collider>();
        app.insert_resource(PhysicsPipeline::new())
            .insert_resource(PhysicsState::new())
            .add_system(UpdateGroup::FixedUpdate, apply_character_movement)
            .add_system(UpdateGroup::LateFixedUpdate, step_simulation)
            .add_system(UpdateGroup::LateFixedUpdate, probe_ground)
            .add_system(UpdateGroup::Update, register_colliders)
            .add_system(UpdateGroup::Update, interpolate_body_transforms);
    }
}
