use std::{
    any::TypeId,
    collections::HashSet,
    hash::{Hash, Hasher},
};

use crate::entity::EntityType;

pub fn generate_type_id(type_ids: &Vec<TypeId>) -> EntityType {
    let sorted_type_ids: HashSet<TypeId> = HashSet::from_iter(type_ids.iter().cloned());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for type_id in sorted_type_ids {
        type_id.hash(&mut hasher);
    }
    EntityType(hasher.finish())
}
