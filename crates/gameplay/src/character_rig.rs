//! A procedurally-built placeholder character: a simple blocky humanoid rig (7 bones, each a
//! separately-meshed box) animated entirely through hand-authored [`AnimationClip`]s, driving
//! the same [`AnimationStateMachine`] + 2D blend-space machinery a real imported glTF character
//! would use. This exists because CC0 character assets couldn't be fetched over the network in
//! this environment — swap it for a real rig by replacing [`spawn_character`] with a glTF import
//! that produces an equivalent `SkeletonComponent` + clip set (see the `movement` blackboard
//! keys this module wires up: `"movement"` (Vec2) and `"locomotion_phase"` (Int)).

use std::f32::consts::TAU;

use animation::{
    blackboard::AnimationBlackboard,
    clip::{AnimationChanelOutput, AnimationChannel, AnimationClip},
    graph::AnimationGraph,
    node::state_machine::{AnimationFSMTrigger, AnimationStateMachine},
    player::{AnimationHandleComponent, AnimationPlayer},
};
use color::LinearRgba;
use ecs::{CommandQueue, component::Component, entity::Entity};
use essential::{assets::asset_server::AssetServer, transform::Transform};
use glam::{Quat, Vec3};
use mesh::{MeshComponent, skeleton::SkeletonComponent};
use render::{assets::material::StandardMaterial, components::material::MaterialComponent};
use uuid::Uuid;

use crate::primitives::make_box_mesh_offset;

const HIP_HEIGHT: f32 = 0.9;
const LEG_APART: f32 = 0.16;
const LEG_HALF_LENGTH: f32 = 0.45;
const TORSO_HALF_HEIGHT: f32 = 0.25;
const HEAD_HALF: f32 = 0.15;
const ARM_APART: f32 = 0.34;
const ARM_SHOULDER_HEIGHT: f32 = 0.42;
const ARM_HALF_LENGTH: f32 = 0.27;

/// Marks the entity holding the rig's [`AnimationPlayer`]/[`SkeletonComponent`], for systems
/// that need to find "the animated character" (e.g. camera/debug overlays).
#[derive(Component)]
pub struct CharacterRig;

struct RigBones {
    hips: Uuid,
    torso: Uuid,
    head: Uuid,
    left_upper_leg: Uuid,
    right_upper_leg: Uuid,
    left_upper_arm: Uuid,
    right_upper_arm: Uuid,
}

impl RigBones {
    fn new() -> Self {
        Self {
            hips: Uuid::new_v4(),
            torso: Uuid::new_v4(),
            head: Uuid::new_v4(),
            left_upper_leg: Uuid::new_v4(),
            right_upper_leg: Uuid::new_v4(),
            left_upper_arm: Uuid::new_v4(),
            right_upper_arm: Uuid::new_v4(),
        }
    }

    fn ordered_ids(&self) -> Vec<Uuid> {
        vec![
            self.hips,
            self.torso,
            self.head,
            self.left_upper_leg,
            self.right_upper_leg,
            self.left_upper_arm,
            self.right_upper_arm,
        ]
    }
}

/// Spawns one box-shaped bone entity, parented to `parent`, with a mesh offset from its own
/// pivot so the pivot itself can be driven purely by animated rotation/translation.
#[allow(clippy::too_many_arguments)]
fn spawn_bone(
    cmd: &mut CommandQueue,
    asset_server: &AssetServer,
    parent: Entity,
    local_translation: Vec3,
    mesh_half_extents: Vec3,
    mesh_offset: Vec3,
    color: LinearRgba,
) -> Entity {
    let mesh_handle = asset_server.add(make_box_mesh_offset(mesh_offset, mesh_half_extents));
    let material_handle =
        asset_server.add(StandardMaterial::default().with_base_color_factor(color));

    let entity = *cmd
        .spawn((
            Transform::from_translation(local_translation),
            MeshComponent {
                handle: mesh_handle,
            },
            MaterialComponent::<StandardMaterial> {
                handle: material_handle,
            },
        ))
        .entity();

    cmd.add_child(parent, entity);
    entity
}

/// Builds translation/rotation/scale channels for one bone across `times`, from per-sample
/// closures. Always emits all three channels so an unanimated bone still gets its correct bind
/// pose rather than defaulting to the pose pool's identity/stale values.
fn build_channels(
    times: &[f32],
    translation: impl Fn(f32) -> Vec3,
    rotation: impl Fn(f32) -> Quat,
) -> [AnimationChannel; 3] {
    let translations: Vec<Vec3> = times.iter().map(|&t| translation(t)).collect();
    let rotations: Vec<Quat> = times.iter().map(|&t| rotation(t)).collect();
    let scales: Vec<Vec3> = times.iter().map(|_| Vec3::ONE).collect();

    [
        AnimationChannel::new(
            times.to_vec(),
            AnimationChanelOutput::Translation(translations),
        ),
        AnimationChannel::new(times.to_vec(), AnimationChanelOutput::Rotation(rotations)),
        AnimationChannel::new(times.to_vec(), AnimationChanelOutput::Scale(scales)),
    ]
}

fn add_bone_channels(
    clip: &mut AnimationClip,
    bone: Uuid,
    times: &[f32],
    translation: impl Fn(f32) -> Vec3,
    rotation: impl Fn(f32) -> Quat,
) {
    for channel in build_channels(times, translation, rotation) {
        clip.add_channel(bone, channel);
    }
}

/// A cyclic locomotion clip (idle/walk/run all share this shape): legs swing opposite-phase,
/// arms swing opposite their same-side leg, torso leans subtly, hips bob twice per stride.
fn locomotion_clip(
    bones: &RigBones,
    duration: f32,
    leg_amp_deg: f32,
    arm_amp_deg: f32,
    torso_amp_deg: f32,
    hip_bob: f32,
) -> AnimationClip {
    const SAMPLES: usize = 16;
    let times: Vec<f32> = (0..=SAMPLES)
        .map(|i| i as f32 / SAMPLES as f32 * duration)
        .collect();

    let leg_amp = leg_amp_deg.to_radians();
    let arm_amp = arm_amp_deg.to_radians();
    let torso_amp = torso_amp_deg.to_radians();
    let phase = move |t: f32| t / duration * TAU;

    let mut clip = AnimationClip::default();

    add_bone_channels(
        &mut clip,
        bones.hips,
        &times,
        move |t| {
            Vec3::new(
                0.0,
                HIP_HEIGHT + (phase(t) * 2.0).sin().abs() * hip_bob,
                0.0,
            )
        },
        |_| Quat::IDENTITY,
    );
    add_bone_channels(
        &mut clip,
        bones.torso,
        &times,
        |_| Vec3::ZERO,
        move |t| Quat::from_rotation_x(-phase(t).sin() * torso_amp * 0.3),
    );
    add_bone_channels(
        &mut clip,
        bones.head,
        &times,
        |_| Vec3::new(0.0, TORSO_HALF_HEIGHT * 2.0 + HEAD_HALF, 0.0),
        |_| Quat::IDENTITY,
    );
    add_bone_channels(
        &mut clip,
        bones.left_upper_leg,
        &times,
        |_| Vec3::new(-LEG_APART, 0.0, 0.0),
        move |t| Quat::from_rotation_x(phase(t).sin() * leg_amp),
    );
    add_bone_channels(
        &mut clip,
        bones.right_upper_leg,
        &times,
        |_| Vec3::new(LEG_APART, 0.0, 0.0),
        move |t| Quat::from_rotation_x((phase(t) + std::f32::consts::PI).sin() * leg_amp),
    );
    add_bone_channels(
        &mut clip,
        bones.left_upper_arm,
        &times,
        |_| Vec3::new(-ARM_APART, ARM_SHOULDER_HEIGHT, 0.0),
        move |t| Quat::from_rotation_x((phase(t) + std::f32::consts::PI).sin() * arm_amp),
    );
    add_bone_channels(
        &mut clip,
        bones.right_upper_arm,
        &times,
        |_| Vec3::new(ARM_APART, ARM_SHOULDER_HEIGHT, 0.0),
        move |t| Quat::from_rotation_x(phase(t).sin() * arm_amp),
    );

    clip
}

/// A short, mostly-static held pose (jump-start/airborne/land): legs and arms tilt to a fixed
/// angle and hips shift up/down slightly, giving each state a distinct silhouette without
/// needing per-frame keyframes.
fn pose_clip(
    bones: &RigBones,
    duration: f32,
    leg_angle_deg: f32,
    arm_angle_deg: f32,
    hip_offset: f32,
) -> AnimationClip {
    let times = vec![0.0, duration.max(0.05)];
    let leg_angle = leg_angle_deg.to_radians();
    let arm_angle = arm_angle_deg.to_radians();

    let mut clip = AnimationClip::default();

    add_bone_channels(
        &mut clip,
        bones.hips,
        &times,
        move |_| Vec3::new(0.0, HIP_HEIGHT + hip_offset, 0.0),
        |_| Quat::IDENTITY,
    );
    add_bone_channels(
        &mut clip,
        bones.torso,
        &times,
        |_| Vec3::ZERO,
        |_| Quat::IDENTITY,
    );
    add_bone_channels(
        &mut clip,
        bones.head,
        &times,
        |_| Vec3::new(0.0, TORSO_HALF_HEIGHT * 2.0 + HEAD_HALF, 0.0),
        |_| Quat::IDENTITY,
    );
    add_bone_channels(
        &mut clip,
        bones.left_upper_leg,
        &times,
        |_| Vec3::new(-LEG_APART, 0.0, 0.0),
        move |_| Quat::from_rotation_x(leg_angle),
    );
    add_bone_channels(
        &mut clip,
        bones.right_upper_leg,
        &times,
        |_| Vec3::new(LEG_APART, 0.0, 0.0),
        move |_| Quat::from_rotation_x(leg_angle),
    );
    add_bone_channels(
        &mut clip,
        bones.left_upper_arm,
        &times,
        |_| Vec3::new(-ARM_APART, ARM_SHOULDER_HEIGHT, 0.0),
        move |_| Quat::from_rotation_x(-arm_angle),
    );
    add_bone_channels(
        &mut clip,
        bones.right_upper_arm,
        &times,
        |_| Vec3::new(ARM_APART, ARM_SHOULDER_HEIGHT, 0.0),
        move |_| Quat::from_rotation_x(-arm_angle),
    );

    clip
}

/// Spawns the placeholder character's visual rig (as a child of `parent`, typically the
/// player's capsule entity) and wires up its animation state machine:
/// `Grounded` (idle/walk/run 2D blend space, keyed on the `"movement"` blackboard Vec2) ->
/// `JumpStart` -> `Airborne` -> `Landing` -> back to `Grounded`, all transitions gated on the
/// `"locomotion_phase"` blackboard Int (written by [`crate::movement_state::write_animation_params`]).
pub fn spawn_character(
    cmd: &mut CommandQueue,
    asset_server: &AssetServer,
    parent: Entity,
) -> Entity {
    let skin_color = LinearRgba::new(0.85, 0.65, 0.5, 1.0);
    let clothes_color = LinearRgba::new(0.2, 0.35, 0.6, 1.0);

    let rig_root = *cmd
        .spawn(Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)))
        .entity();
    cmd.add_child(parent, rig_root);

    let bones = RigBones::new();

    let hips = spawn_bone(
        cmd,
        asset_server,
        rig_root,
        Vec3::new(0.0, HIP_HEIGHT, 0.0),
        Vec3::new(0.2, 0.15, 0.14),
        Vec3::ZERO,
        clothes_color,
    );
    let torso = spawn_bone(
        cmd,
        asset_server,
        hips,
        Vec3::ZERO,
        Vec3::new(0.22, TORSO_HALF_HEIGHT, 0.13),
        Vec3::new(0.0, TORSO_HALF_HEIGHT, 0.0),
        clothes_color,
    );
    let head = spawn_bone(
        cmd,
        asset_server,
        torso,
        Vec3::new(0.0, TORSO_HALF_HEIGHT * 2.0 + HEAD_HALF, 0.0),
        Vec3::splat(HEAD_HALF),
        Vec3::ZERO,
        skin_color,
    );
    let left_upper_leg = spawn_bone(
        cmd,
        asset_server,
        hips,
        Vec3::new(-LEG_APART, 0.0, 0.0),
        Vec3::new(0.11, LEG_HALF_LENGTH, 0.11),
        Vec3::new(0.0, -LEG_HALF_LENGTH, 0.0),
        clothes_color,
    );
    let right_upper_leg = spawn_bone(
        cmd,
        asset_server,
        hips,
        Vec3::new(LEG_APART, 0.0, 0.0),
        Vec3::new(0.11, LEG_HALF_LENGTH, 0.11),
        Vec3::new(0.0, -LEG_HALF_LENGTH, 0.0),
        clothes_color,
    );
    let left_upper_arm = spawn_bone(
        cmd,
        asset_server,
        torso,
        Vec3::new(-ARM_APART, ARM_SHOULDER_HEIGHT, 0.0),
        Vec3::new(0.09, ARM_HALF_LENGTH, 0.09),
        Vec3::new(0.0, -ARM_HALF_LENGTH, 0.0),
        skin_color,
    );
    let right_upper_arm = spawn_bone(
        cmd,
        asset_server,
        torso,
        Vec3::new(ARM_APART, ARM_SHOULDER_HEIGHT, 0.0),
        Vec3::new(0.09, ARM_HALF_LENGTH, 0.09),
        Vec3::new(0.0, -ARM_HALF_LENGTH, 0.0),
        skin_color,
    );

    // Sanity: entity spawn order above must match `RigBones::ordered_ids()`'s field order.
    let bone_entities = vec![
        hips,
        torso,
        head,
        left_upper_leg,
        right_upper_leg,
        left_upper_arm,
        right_upper_arm,
    ];
    let bone_ids = bones.ordered_ids();

    let skeleton_handle = asset_server.add(mesh::skeleton::Skeleton::from(vec![
            glam::Mat4::IDENTITY;
            bone_entities.len()
        ]));

    let idle_clip = asset_server.add(locomotion_clip(&bones, 2.5, 0.0, 3.0, 2.0, 0.015));
    let walk_clip = asset_server.add(locomotion_clip(&bones, 0.9, 25.0, 18.0, 4.0, 0.04));
    let run_clip = asset_server.add(locomotion_clip(&bones, 0.5, 45.0, 35.0, 8.0, 0.07));
    let jump_start_clip = asset_server.add(pose_clip(&bones, 0.12, 15.0, -20.0, -0.15));
    let airborne_clip = asset_server.add(pose_clip(&bones, 0.5, -20.0, 30.0, 0.05));
    let land_clip = asset_server.add(pose_clip(&bones, 0.2, 10.0, -10.0, -0.1));

    let mut grounded_graph = AnimationGraph::new();
    grounded_graph.result_node().with_blend_space_2d_input(
        |blackboard: &AnimationBlackboard| blackboard.get_vec2("movement").unwrap_or_default(),
        |ctx| {
            ctx.animation_clip_input(&idle_clip, glam::Vec2::new(0.0, 0.0))
                .animation_clip_input(&walk_clip, glam::Vec2::new(-0.3, 0.5))
                .animation_clip_input(&walk_clip, glam::Vec2::new(0.3, 0.5))
                .animation_clip_input(&walk_clip, glam::Vec2::new(0.0, 0.5))
                .animation_clip_input(&run_clip, glam::Vec2::new(0.0, 1.0));
        },
    );
    let grounded_graph_handle = asset_server.add(grounded_graph);
    let jump_start_graph_handle = asset_server.add(AnimationGraph::from_clip(jump_start_clip));
    let airborne_graph_handle = asset_server.add(AnimationGraph::from_clip(airborne_clip));
    let land_graph_handle = asset_server.add(AnimationGraph::from_clip(land_clip));

    let locomotion_phase = |n: u32| {
        AnimationFSMTrigger::from_condition(move |bb| bb.get_int("locomotion_phase") == Some(n))
    };

    let fsm = AnimationStateMachine::from_initial_state("Grounded", grounded_graph_handle, |tb| {
        tb.to("JumpStart", locomotion_phase(1), 0.15);
    })
    .state("JumpStart", jump_start_graph_handle, |tb| {
        tb.to("Airborne", locomotion_phase(2), 0.1);
    })
    .state("Airborne", airborne_graph_handle, |tb| {
        tb.to("Landing", locomotion_phase(3), 0.15);
    })
    .state("Landing", land_graph_handle, |tb| {
        tb.to("Grounded", locomotion_phase(0), 0.2);
    })
    .build();

    let mut top_graph = AnimationGraph::new();
    let result_index = top_graph.result_node().index();
    top_graph.add_node(fsm, result_index);
    let top_graph_handle = asset_server.add(top_graph);

    cmd.insert(
        SkeletonComponent::new(skeleton_handle, bone_entities, bone_ids),
        rig_root,
    );
    cmd.insert(AnimationPlayer::new(7), rig_root);
    cmd.insert(
        AnimationHandleComponent {
            handle: top_graph_handle,
        },
        rig_root,
    );
    cmd.insert(CharacterRig, rig_root);

    rig_root
}
