use derive_more::Deref;
use ecs::{
    events::event_reader::EventReader,
    resource::{Res, ResMut, Resource},
};
use window::input::{Input, InputState, MouseButton};

use crate::interaction::{HoveredNode, UIClick};

/// The UI entity that currently holds keyboard focus, if any.
///
/// Focus is set when the user clicks an [`Interactable`](crate::interaction::Interactable) node
/// and cleared when the user clicks on empty space.  Widgets that need keyboard
/// input (e.g. [`UITextInput`](crate::text_input::UITextInput)) read this resource to decide
/// whether to consume typed characters.
#[derive(Resource, Deref)]
pub struct FocusedWidget(pub Option<ecs::entity::Entity>);

/// Sets [`FocusedWidget`] based on left-button clicks.
///
/// - Click on an interactable node → focus that node.
/// - Click on empty space (no node hovered) → clear focus.
pub(crate) fn update_focus(
    mut focused: ResMut<FocusedWidget>,
    hovered: Res<HoveredNode>,
    input: Res<Input>,
    _clicks: EventReader<UIClick>,
) {
    if input.get_mouse_button_state(MouseButton::Left) == InputState::Pressed {
        focused.0 = hovered.0;
    }
}
