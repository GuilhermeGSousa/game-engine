use derive_more::{Deref, DerefMut};
use ecs::events::event_writer::EventWriter;
use ecs::{
    component::Component,
    entity::Entity,
    events::Event,
    query::{Query, query_filter::With},
    resource::{Res, ResMut, Resource},
};
use glam::Vec2;
use window::input::{Input, InputState, MouseButton};

use crate::{material::UIMaterial, node::UIComputedNode};

/// The UI entity currently under the cursor, if any.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct HoveredNode(Option<Entity>);

/// Opts a node into hit testing and click events.
///
/// Add this to any UI entity that should receive [`UIClick`] events or
/// contribute to [`HoveredNode`].  Deliberately separate from
/// [`UIInteractionStyle`] so that interactability and visual feedback are
/// independent: a node can be clickable without changing colour, and a node
/// can show hover colours without being a click target.
#[derive(Component)]
pub struct Interactable;

/// Marks a node as non-interactive.  When present, `apply_interaction_styles`
/// uses `UIInteractionStyle::disabled` regardless of cursor position.
#[derive(Component)]
pub struct UIDisabled;

/// Per-node colour palette for the four interaction states.
///
/// Attach this alongside [`UIMaterial`] to get automatic hover/press colour
/// changes.  The system `apply_interaction_styles` writes the correct colour
/// into `UIMaterial::color` each frame based on the current cursor position and
/// left-button state.
///
/// # Example
/// ```rust,ignore
/// (
///     UINode::default(),
///     UIMaterial::flat([0.2, 0.2, 0.2, 1.0]),
///     UIInteractionStyle {
///         normal:   [0.20, 0.20, 0.20, 1.0],
///         hovered:  [0.28, 0.28, 0.28, 1.0],
///         pressed:  [0.14, 0.14, 0.14, 1.0],
///         disabled: [0.10, 0.10, 0.10, 0.5],
///     },
/// )
/// ```
#[derive(Component, Clone)]
pub struct UIInteractionStyle {
    pub normal: [f32; 4],
    pub hovered: [f32; 4],
    pub pressed: [f32; 4],
    pub disabled: [f32; 4],
}

/// Fired the frame a UI node is clicked with the left mouse button.
#[derive(Event)]
pub struct UIClick {
    pub entity: Entity,
    pub position: Vec2,
}

/// Walks all [`UIComputedNode`]s each frame, determines which one (if any) is
/// under the cursor, updates [`HoveredNode`], and fires [`UIClick`] events on
/// left-button press.
///
/// Runs in `LateUpdate`, after `compute_ui_nodes` has populated
/// [`UIComputedNode`] for the current frame.
pub(crate) fn update_ui_interaction(
    computed_nodes: Query<(Entity, &UIComputedNode), With<Interactable>>,
    input: Res<Input>,
    mut hovered: ResMut<HoveredNode>,
    mut click_writer: EventWriter<UIClick>,
) {
    let cursor = input.mouse_position();

    // Pick the node highest in the Z-order that contains the cursor.
    let mut best: Option<(Entity, i32)> = None;
    for (entity, node) in computed_nodes.iter() {
        let loc = node.location;
        let size = node.size;
        if cursor.x >= loc.x
            && cursor.x <= loc.x + size.x
            && cursor.y >= loc.y
            && cursor.y <= loc.y + size.y
        {
            if best.map_or(true, |(_, z)| node.z_index > z) {
                best = Some((entity, node.z_index));
            }
        }
    }

    **hovered = best.map(|(e, _)| e);

    if input.get_mouse_button_state(MouseButton::Left) == InputState::Pressed {
        if let Some((entity, _)) = best {
            click_writer.write(UIClick {
                entity,
                position: cursor,
            });
        }
    }
}

/// Drives [`UIMaterial::color`] from [`UIInteractionStyle`] each frame.
///
/// For each entity that has both components:
/// - If it has [`UIDisabled`], use `style.disabled`.
/// - Else if it is the currently hovered node and the left button is held,
///   use `style.pressed`.
/// - Else if it is hovered, use `style.hovered`.
/// - Otherwise use `style.normal`.
pub(crate) fn apply_interaction_styles(
    hovered: Res<HoveredNode>,
    input: Res<Input>,
    disabled_nodes: Query<(Entity, &UIDisabled)>,
    styled: Query<(Entity, &UIInteractionStyle, &mut UIMaterial)>,
) {
    let left_held = matches!(
        input.get_mouse_button_state(MouseButton::Left),
        InputState::Pressed | InputState::Down
    );

    for (entity, style, mut material) in styled.iter() {
        let color = if disabled_nodes.get_entity(entity).is_some() {
            style.disabled
        } else if **hovered == Some(entity) {
            if left_held {
                style.pressed
            } else {
                style.hovered
            }
        } else {
            style.normal
        };
        material.color = color;
    }
}
