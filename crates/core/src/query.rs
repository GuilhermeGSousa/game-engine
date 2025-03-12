use std::marker::PhantomData;

use crate::{bundle::ComponentBorrowBundle, system_input::SystemInput, world::UnsafeWorldCell};

pub struct Query<'world, T: ComponentBorrowBundle> {
    world: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

impl<'world, T: ComponentBorrowBundle> Query<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'world, T> {
        QueryIter {
            world: self.world,
            current_archetype_index: 0,
            current_entity_index: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'world, T> {
    world: UnsafeWorldCell<'world>,
    current_archetype_index: usize,
    current_entity_index: usize,
    _marker: PhantomData<T>,
}

impl<'world, T> Iterator for QueryIter<'world, T>
where
    T: ComponentBorrowBundle,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let archetypes = self.world.get_world_mut().get_archetypes_mut();

        for i in self.current_archetype_index..archetypes.len() {
            let archetype = &archetypes[i];

            if self.current_entity_index >= archetype.len() {
                self.current_entity_index = 0;
                self.current_archetype_index += 1;
            }

            // if let Some(components) =
            // {
            //     self.current_archetype_index += 1;
            //     return Some(components);
            // } else {
            //     panic!("AAAAHHHH");
            // }
        }
        None
    }
}

unsafe impl<T> SystemInput for Query<'_, T>
where
    T: ComponentBorrowBundle,
{
    type Data<'world> = Query<'world, T>;

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        Query::new(world)
    }
}
