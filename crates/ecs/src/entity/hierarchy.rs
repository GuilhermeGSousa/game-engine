use crate::{component::Component, entity::Entity};

#[derive(Component)]
pub struct Children {
    children: Vec<Entity>,
}

impl Children {
    pub(crate) fn from_children(children: Vec<Entity>) -> Self {
        Self { children }
    }

    pub(crate) fn add_child(&mut self, child: Entity) {
        self.children.push(child);
    }
}

#[derive(Component)]
pub struct ChildOf {
    parent: Entity,
}

impl ChildOf {
    pub fn new(parent: Entity) -> Self {
        Self { parent }
    }
}
