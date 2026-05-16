use ecs::component::Component;

/// Marker component — attach to a Camera entity to have its output mirrored to the terminal.
#[derive(Component)]
pub struct TerminalCamera;
