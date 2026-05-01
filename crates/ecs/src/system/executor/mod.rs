use crate::{
    system::{graph::SystemDependencyGraph, schedule::CompiledScheduleData},
    World,
};

pub mod multi_thread;
pub mod single_thread;

pub trait SystemExecutor: Send + Sync {
    fn init() -> Self
    where
        Self: Sized;

    fn run(
        &self,
        graph: &mut SystemDependencyGraph,
        compiled_data: &CompiledScheduleData,
        world: &mut World,
    );
}
