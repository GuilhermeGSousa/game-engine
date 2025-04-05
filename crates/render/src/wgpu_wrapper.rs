use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct WgpuWrapper<T>(T);

impl<T> WgpuWrapper<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

unsafe impl<T> Send for WgpuWrapper<T> {}
unsafe impl<T> Sync for WgpuWrapper<T> {}

impl<T> Deref for WgpuWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for WgpuWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
