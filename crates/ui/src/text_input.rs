use ecs::{
    component::Component,
    entity::Entity,
    events::{Event, event_writer::EventWriter},
    query::Query,
    resource::{Res, Resource},
};
use window::input::{Input, InputState, KeyCode, PhysicalKey};

use crate::{focus::FocusedWidget, material::UIMaterial, text::TextComponent};

/// A single-line text input widget.
///
/// Place this on an entity that also has [`UINode`](crate::node::UINode),
/// [`UIMaterial`], [`TextComponent`], and
/// [`Interactable`](crate::interaction::Interactable).
///
/// The `update_text_inputs` system reads typed characters from `Input` when
/// this entity has focus, updates `TextComponent::text` each frame (showing
/// a `|` cursor when focused, or the placeholder when empty and unfocused),
/// and fires [`UITextInputChanged`] whenever `value` changes.
#[derive(Component)]
pub struct UITextInput {
    /// Current string value of the field.
    pub value: String,
    /// Shown when the field is empty and unfocused.
    pub placeholder: String,
    /// Byte offset of the insert cursor within `value`.
    pub cursor: usize,
}

impl UITextInput {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            value: String::new(),
            placeholder: placeholder.into(),
            cursor: 0,
        }
    }
}

/// Fired whenever a [`UITextInput`]'s value changes.
#[derive(Event)]
pub struct UITextInputChanged {
    pub entity: Entity,
    pub value: String,
}

/// Dummy resource marker so the event can be registered.
#[derive(Resource)]
pub struct TextInputResource;

/// Processes keyboard input for focused [`UITextInput`] widgets.
///
/// - Typed printable characters are inserted at the cursor.
/// - Backspace (on press) removes the character before the cursor.
/// - Left/Right arrow keys move the cursor by one `char`.
/// - `TextComponent::text` is updated every frame to reflect the current
///   value + cursor indicator (focused) or placeholder (empty + unfocused).
pub(crate) fn update_text_inputs(
    focused: Res<FocusedWidget>,
    input: Res<Input>,
    text_inputs: Query<(
        Entity,
        &mut UITextInput,
        &mut TextComponent,
        &mut UIMaterial,
    )>,
    mut writer: EventWriter<UITextInputChanged>,
) {
    for (entity, mut ti, mut text, mut material) in text_inputs.iter() {
        let is_focused = **focused == Some(entity);

        if is_focused {
            let mut changed = false;

            // --- printable characters ---
            for &c in input.typed_chars() {
                let cursor = ti.cursor;
                ti.value.insert(cursor, c);
                ti.cursor += c.len_utf8();
                changed = true;
            }

            // --- backspace: delete character before cursor ---
            let backspace = input.get_key_state(PhysicalKey::Code(KeyCode::Backspace));
            if backspace == InputState::Pressed && ti.cursor > 0 {
                let mut pos = ti.cursor;
                loop {
                    pos -= 1;
                    if ti.value.is_char_boundary(pos) {
                        break;
                    }
                }
                ti.value.remove(pos);
                ti.cursor = pos;
                changed = true;
            }

            // --- arrow keys: move cursor ---
            let left = input.get_key_state(PhysicalKey::Code(KeyCode::ArrowLeft));
            if left == InputState::Pressed && ti.cursor > 0 {
                let mut pos = ti.cursor;
                loop {
                    pos -= 1;
                    if ti.value.is_char_boundary(pos) {
                        break;
                    }
                }
                ti.cursor = pos;
            }

            let right = input.get_key_state(PhysicalKey::Code(KeyCode::ArrowRight));
            if right == InputState::Pressed && ti.cursor < ti.value.len() {
                let mut pos = ti.cursor + 1;
                while pos <= ti.value.len() && !ti.value.is_char_boundary(pos) {
                    pos += 1;
                }
                ti.cursor = pos;
            }

            if changed {
                writer.write(UITextInputChanged {
                    entity,
                    value: ti.value.clone(),
                });
            }
        }

        // --- update displayed text and border colour ---
        let display = if is_focused {
            let mut s = ti.value.clone();
            s.insert(ti.cursor, '|');
            s
        } else if ti.value.is_empty() {
            ti.placeholder.clone()
        } else {
            ti.value.clone()
        };

        if text.text != display {
            text.text = display;
        }

        // Highlight border when focused.
        material.border_color = if is_focused {
            [0.40, 0.65, 1.00, 1.0]
        } else {
            [0.30, 0.30, 0.30, 1.0]
        };
    }
}
