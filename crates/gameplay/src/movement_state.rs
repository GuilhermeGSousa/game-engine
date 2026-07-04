use animation::player::AnimationPlayer;
use ecs::{component::Component, query::Query, resource::Res};
use essential::time::Time;
use glam::Vec2;
use physics::character_controller::CharacterController;
use window::input::{Input, KeyCode, PhysicalKey};

use crate::movement::RUN_SPEED;

/// How long the `JumpStart` pose plays before falling back to the shared air loop.
pub const JUMP_START_DURATION: f32 = 0.12;
/// How long the `Landing` pose plays before returning control to `Grounded`.
pub const LANDING_DURATION: f32 = 0.2;

/// The character's coarse movement phase, driving both physics (jump requests) and the
/// animation state machine (via the `"locomotion_phase"` blackboard key).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CharacterMovementPhase {
    Grounded,
    JumpStart,
    Airborne,
    Landing,
}

impl CharacterMovementPhase {
    /// The integer value written to the `"locomotion_phase"` animation blackboard key.
    pub fn as_blackboard_index(self) -> u32 {
        match self {
            CharacterMovementPhase::Grounded => 0,
            CharacterMovementPhase::JumpStart => 1,
            CharacterMovementPhase::Airborne => 2,
            CharacterMovementPhase::Landing => 3,
        }
    }
}

#[derive(Component)]
pub struct CharacterMovementState {
    pub phase: CharacterMovementPhase,
    /// Seconds spent in the current phase.
    pub phase_timer: f32,
}

impl Default for CharacterMovementState {
    fn default() -> Self {
        Self {
            phase: CharacterMovementPhase::Grounded,
            phase_timer: 0.0,
        }
    }
}

/// Advances each character's movement phase and consumes jump input.
///
/// This is the single place jump input is read and turned into a physics jump request —
/// [`CharacterController::request_jump`] is only ever called from here.
pub fn advance_movement_state(
    query: Query<(&mut CharacterMovementState, &mut CharacterController)>,
    input: Res<Input>,
    time: Res<Time>,
) {
    let dt = time.delta().as_secs_f32();

    for (mut state, mut controller) in query.iter() {
        state.phase_timer += dt;

        match state.phase {
            CharacterMovementPhase::Grounded => {
                if controller.grounded && input.is_just_pressed(PhysicalKey::Code(KeyCode::Space)) {
                    controller.request_jump();
                    state.phase = CharacterMovementPhase::JumpStart;
                    state.phase_timer = 0.0;
                } else if !controller.grounded {
                    state.phase = CharacterMovementPhase::Airborne;
                    state.phase_timer = 0.0;
                }
            }
            CharacterMovementPhase::JumpStart => {
                if state.phase_timer >= JUMP_START_DURATION {
                    state.phase = CharacterMovementPhase::Airborne;
                    state.phase_timer = 0.0;
                }
            }
            CharacterMovementPhase::Airborne => {
                if controller.grounded {
                    state.phase = CharacterMovementPhase::Landing;
                    state.phase_timer = 0.0;
                }
            }
            CharacterMovementPhase::Landing => {
                if !controller.grounded {
                    state.phase = CharacterMovementPhase::Airborne;
                    state.phase_timer = 0.0;
                } else if state.phase_timer >= LANDING_DURATION {
                    state.phase = CharacterMovementPhase::Grounded;
                    state.phase_timer = 0.0;
                }
            }
        }
    }
}

/// Bridges gameplay movement state onto the `AnimationBlackboard`, via `AnimationPlayer`'s
/// param setters. This is the only system that writes the `"movement"`/`"locomotion_phase"`
/// keys the animation graph (see the animation crate's locomotion state machine) reads.
pub fn write_animation_params(
    query: Query<(
        &CharacterMovementState,
        &CharacterController,
        &mut AnimationPlayer,
    )>,
) {
    for (state, controller, mut anim_player) in query.iter() {
        let planar_speed = controller.desired_translation.length();
        let normalized_speed = (planar_speed / RUN_SPEED).clamp(0.0, 1.0);

        anim_player.set_vec2_param("movement", Vec2::new(0.0, normalized_speed));
        anim_player.set_int_param("locomotion_phase", state.phase.as_blackboard_index());
    }
}
