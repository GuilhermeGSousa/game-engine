use ecs::resource::Resource;

use crate::physics_state::PhysicsState;
use rapier3d::prelude::*;

#[derive(Resource)]
pub struct PhysicsPipeline {
    pipeline: rapier3d::pipeline::PhysicsPipeline,
}

impl PhysicsPipeline {
    pub fn new() -> Self {
        PhysicsPipeline {
            pipeline: rapier3d::pipeline::PhysicsPipeline::new(),
        }
    }

    pub fn step(&mut self, state: &mut PhysicsState) {
        let gravity = vector![0.0, -9.81, 0.0];

        self.pipeline.step(
            &gravity,
            &state.integration_parameters,
            &mut state.island_manager,
            &mut state.broad_phase,
            &mut state.narrow_phase,
            &mut state.rigid_body_set,
            &mut state.collider_set,
            &mut state.impulse_joint_set,
            &mut state.multibody_joint_set,
            &mut state.ccd_solver,
            Some(&mut state.query_pipeline),
            &(),
            &(),
        );
    }
}
