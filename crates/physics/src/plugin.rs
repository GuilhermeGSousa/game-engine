use app::plugins::Plugin;
use ecs::system::schedule::UpdateGroup;

use crate::{
    collider::{register_colliders, Collider},
    ground::probe_ground,
    interpolation::interpolate_body_transforms,
    movement::apply_character_movement,
    physics_pipeline::PhysicsPipeline,
    physics_state::PhysicsState,
    shape::{clean_shapes_for_dropped_meshes, generate_mesh_shapes, PhysicsMeshShapes},
    simulation::step_simulation,
};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut app::App) {
        app.register_component_lifecycle::<Collider>();
        app.insert_resource(PhysicsPipeline::new())
            .insert_resource(PhysicsState::new())
            .insert_resource(PhysicsMeshShapes::default())
            .add_system(UpdateGroup::FixedUpdate, apply_character_movement)
            .add_system(UpdateGroup::LateFixedUpdate, step_simulation)
            .add_system(UpdateGroup::LateFixedUpdate, probe_ground)
            .add_system(UpdateGroup::Update, generate_mesh_shapes)
            // LateUpdate, so `TransformPlugin` has already propagated global
            // transforms this frame: bodies are placed from world space, and
            // a collider on a nested entity inherits its parent's scale.
            .add_system(UpdateGroup::LateUpdate, register_colliders)
            .add_system(UpdateGroup::Update, interpolate_body_transforms)
            .add_system(UpdateGroup::Update, clean_shapes_for_dropped_meshes);
    }
}
