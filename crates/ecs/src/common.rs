use std::{
    any::TypeId,
    hash::{Hash, Hasher},
};

use crate::entity::EntityType;

pub fn generate_type_id(type_ids: &Vec<TypeId>) -> EntityType {
    let mut sorted_type_ids: Vec<TypeId> = type_ids.clone();
    sorted_type_ids.sort();
    sorted_type_ids.dedup();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for type_id in sorted_type_ids {
        type_id.hash(&mut hasher);
    }
    EntityType(hasher.finish())
}
