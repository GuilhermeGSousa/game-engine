use std::ops::{Deref, DerefMut};

use crate::component::Component;


pub struct Mut<'w, T: Component>
{
    data: &'w mut T 
}

impl <'w, T> Mut<'w, T>
where T: Component {
    pub fn new(data: &'w mut T) -> Self
    {
        Self { data }
    }
}

impl <'w, T> Deref for Mut<'w, T>
where T: Component {
    type Target = &'w mut T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl <'w, T> DerefMut for Mut<'w, T>
where T: Component
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}