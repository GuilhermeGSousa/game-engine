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

impl IntoIterator for Children {
    type Item = Entity;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = <&'a Vec<Entity> as IntoIterator>::Item;
    type IntoIter = <&'a Vec<Entity> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.children).into_iter()
    }
}

#[allow(dead_code)]
#[derive(Component)]
pub struct ChildOf {
    parent: Entity,
}

impl ChildOf {
    pub fn new(parent: Entity) -> Self {
        Self { parent }
    }
}
