use ecs::component::Component;

use crate::transform::UIValue;

#[derive(Component)]
pub struct UINode {
    width: UIValue,
    height: UIValue,
}

#[derive(Component)]
pub(crate) struct UIComputedNode;
