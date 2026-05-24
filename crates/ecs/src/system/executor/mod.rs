use crate::{system::schedule::CompiledScheduleData, World};

#[cfg(all(feature = "multithreaded", not(target_arch = "wasm32")))]
pub mod multi_thread;
pub mod single_thread;

pub trait SystemExecutor: Send + Sync {
    fn init(compiled_data: &CompiledScheduleData) -> Self
    where
        Self: Sized;

    fn run(&mut self, compiled_data: &mut CompiledScheduleData, world: &mut World);
}
