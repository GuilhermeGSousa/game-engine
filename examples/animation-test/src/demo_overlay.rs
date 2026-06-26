//! On-screen debug overlay (load state + current FSM state) and one-shot debug-gizmo
//! markers for the important entities (spawner, skinned mesh, root bone).

use color::LinearRgba;
use debug_gizmos::components::GizmoSphere;
use game_engine::{
    animation::{player::AnimationPlayer, root::AnimationRootBone},
    ecs::{command::CommandQueue, component::Component, query::Query, With},
    essential::transform::GlobalTransform,
    gltf_loader::loader::GLTFSpawnerComponent,
    mesh::skeleton::SkeletonComponent,
    ui::{
        node::{UINode, UIRect},
        text::{FontFamily, TextComponent},
        transform::UIValue,
    },
};

use crate::movement_animation::{AnimatedCharacter, AnimationFSMData};

/// Marks the overlay text node so `update_overlay` can find and rewrite it.
#[derive(Component)]
pub(crate) struct OverlayText;

/// Startup: spawn the top-left overlay text node.
pub(crate) fn spawn_overlay(mut cmd: CommandQueue) {
    cmd.spawn((
        UINode {
            // Explicit size: taffy does not measure text, so an Auto-sized node would
            // collapse to 0x0 and clip the glyphs.
            width: UIValue::Px(460.0),
            height: UIValue::Px(64.0),
            margin: UIRect {
                top: 12.0,
                left: 12.0,
                ..Default::default()
            },
            ..Default::default()
        },
        TextComponent {
            text: "Load: starting…\nState: —".to_string(),
            font_size: 16.0,
            line_height: 22.0,
            font_family: FontFamily::Monospace,
            ..Default::default()
        },
        OverlayText,
    ));
}

/// Update: reflect the character's load progress and the live FSM state in the overlay.
pub(crate) fn update_overlay(
    texts: Query<&mut TextComponent, With<OverlayText>>,
    spawners: Query<&GLTFSpawnerComponent, With<AnimatedCharacter>>,
    players: Query<(&AnimationPlayer, &AnimationFSMData)>,
) {
    let (load, state) = if let Some((player, data)) = players.iter().next() {
        // The FSM graph exists -> the character is fully set up.
        let state = match player.current_fsm_state(&data.fsm_node) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => "—".to_string(),
        };
        ("Ready", state)
    } else if spawners.iter().next().is_some() {
        // Still holding the GLTF spawner -> the model is loading.
        ("Loading model…", "—".to_string())
    } else {
        // Spawned, but the animation graph hasn't been built yet.
        ("Setting up animations…", "—".to_string())
    };

    let new_text = format!("Load: {load}\nState: {state}");
    for mut text in texts.iter() {
        // Only assign on change: writing rebuilds the glyphon buffer next frame.
        if text.text != new_text {
            text.text = new_text.clone();
        }
    }
}

/// Update (runs once): drop a coloured marker on the spawner, the skinned-mesh entity, and
/// the root bone once they all have a valid world transform. These are distinct entities
/// (the skinned-mesh node is not the root joint), so the three markers land apart.
pub(crate) fn spawn_entity_gizmos(
    mut cmd: CommandQueue,
    existing: Query<&GizmoSphere>,
    character: Query<&GlobalTransform, With<AnimatedCharacter>>,
    skinned_mesh: Query<&GlobalTransform, With<SkeletonComponent>>,
    root_bone: Query<&GlobalTransform, With<AnimationRootBone>>,
) {
    // Markers persist once spawned; bail if we've already placed them.
    if existing.iter().next().is_some() {
        return;
    }

    let (Some(spawner), Some(mesh), Some(root)) = (
        character.iter().next(),
        skinned_mesh.iter().next(),
        root_bone.iter().next(),
    ) else {
        // Not fully loaded yet (root bone transform not propagated) — try again next frame.
        return;
    };

    // Spawner (character root) — red.
    cmd.spawn(GizmoSphere {
        center: spawner.translation(),
        radius: 0.15,
        color: LinearRgba::new(1.0, 0.2, 0.2, 1.0),
    });
    // Skinned mesh entity — green.
    cmd.spawn(GizmoSphere {
        center: mesh.translation(),
        radius: 0.15,
        color: LinearRgba::new(0.2, 1.0, 0.2, 1.0),
    });
    // Root bone — blue.
    cmd.spawn(GizmoSphere {
        center: root.translation(),
        radius: 0.12,
        color: LinearRgba::new(0.3, 0.5, 1.0, 1.0),
    });
}
