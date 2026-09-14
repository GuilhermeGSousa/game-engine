use std::ops::Deref;

use crate::{component::Component, entity::Entity};

#[derive(Component)]
pub struct Children {
    children: Vec<Entity>,
}

impl Children {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Entity> + ExactSizeIterator {
        self.children.iter()
    }

    pub(crate) fn from_children(children: Vec<Entity>) -> Self {
        Self { children }
    }

    pub(crate) fn add_child(&mut self, child: Entity) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }

    pub(crate) fn remove_child(&mut self, child: Entity) {
        self.children.retain(|candidate| *candidate != child);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.children.is_empty()
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
        self.children.iter()
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

    pub fn parent(&self) -> Entity {
        self.parent
    }
}

impl Deref for ChildOf {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.parent
    }
}
