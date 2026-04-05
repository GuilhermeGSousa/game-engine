use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{Query, query_filter::With},
    resource::Res,
};
use window::input::{Input, InputState, MouseButton};

use crate::node::UIComputedNode;

/// Describes the current interaction state of a UI node that has been marked
/// as interactive.
///
/// Attach this component to any [`UINode`](crate::node::UINode) entity that
/// should respond to pointer input.  The engine will update it every frame via
/// the [`update_interactions`] system.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Interaction {
    /// The cursor is not over this node and no button is held on it.
    #[default]
    None,
    /// The cursor is positioned over this node but no button is held.
    Hovered,
    /// The primary mouse button is held while the cursor is over this node.
    Pressed,
}

/// System that evaluates hit-testing for all UI nodes that carry an
/// [`Interaction`] component.
///
/// Registered by [`UIPlugin`](crate::plugin::UIPlugin) in
/// [`UpdateGroup::LateUpdate`](app::update_group::UpdateGroup::LateUpdate) so
/// that it runs *after* `compute_ui_nodes` has produced fresh
/// [`UIComputedNode`](crate::node::UIComputedNode) data.
pub(crate) fn update_interactions(
    nodes: Query<(Entity, &UIComputedNode), With<Interaction>>,
    input: Res<Input>,
    mut cmd: CommandQueue,
) {
    let cursor = input.mouse_position();
    let left_pressed = matches!(
        input.get_mouse_button_state(MouseButton::Left),
        InputState::Pressed | InputState::Down
    );

    for (entity, computed) in nodes.iter() {
        let x = computed.location.x;
        let y = computed.location.y;
        let w = computed.size.x;
        let h = computed.size.y;

        let hovered =
            cursor.x >= x && cursor.x <= x + w && cursor.y >= y && cursor.y <= y + h;

        let new_state = if hovered && left_pressed {
            Interaction::Pressed
        } else if hovered {
            Interaction::Hovered
        } else {
            Interaction::None
        };

        cmd.insert(new_state, entity);
    }
}
