use std::marker::PhantomData;

use crate::{component::Component, system_input::SystemInput, world::UnsafeWorldCell};
use typle::typle;

pub struct Query<'world, T: QueryData> {
    world: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

pub trait QueryData {
    //type Item<'a>;
}

enum ArchetypeIteratorState {
    Pending(usize),
    Iterating(usize),
}

impl<'world, T: QueryData> Query<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'world, T> {
        QueryIter {
            world: self.world,
            archetype_iteration_state: ArchetypeIteratorState::Pending(0),
            current_entity_index: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'world, T> {
    world: UnsafeWorldCell<'world>,
    archetype_iteration_state: ArchetypeIteratorState,
    current_entity_index: usize,
    _marker: PhantomData<T>,
}

impl<'world, T> Iterator for QueryIter<'world, T>
where
    T: QueryData,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
        // let archetypes = self.world.get_world_mut().get_archetypes_mut();

        // if let ArchetypeIteratorState::Pending(pending_index) = self.archetype_iteration_state {
        //     // Loop until we find a good one
        //     let mut current_index = pending_index;
        //     while current_index < archetypes.len() {
        //         if archetypes[current_index].has_data_for::<T>() {
        //             self.archetype_iteration_state =
        //                 ArchetypeIteratorState::Iterating(current_index);
        //             self.current_entity_index = 0;
        //             break;
        //         }

        //         current_index += 1;
        //     }
        // }

        // match self.archetype_iteration_state {
        //     ArchetypeIteratorState::Pending(_) => None,
        //     ArchetypeIteratorState::Iterating(index) => {
        //         let current_archetype = &archetypes[index];

        //         match current_archetype.get_components::<T>(self.current_entity_index) {
        //             Some(value) => {
        //                 self.current_entity_index += 1;
        //                 if self.current_entity_index >= current_archetype.len() {
        //                     self.archetype_iteration_state =
        //                         ArchetypeIteratorState::Pending(index + 1);
        //                 }

        //                 return Some(value);
        //             }
        //             None => {
        //                 panic!("This should not be possible!")
        //             }
        //         };
        //     }
        // }
    }
}

unsafe impl<T> SystemInput for Query<'_, T>
where
    T: QueryData,
{
    type Data<'world> = Query<'world, T>;

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        Query::new(world)
    }
}

impl<T> QueryData for &T where T: Component {}

#[typle(Tuple for 0..=12)]
impl<T> QueryData for T
where
    T: Tuple,
    T<_>: QueryData,
{
}
