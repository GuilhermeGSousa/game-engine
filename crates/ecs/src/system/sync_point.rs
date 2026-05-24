use crate::{system::access::SystemAccess, System};

pub(crate) struct SyncPoint;

impl System for SyncPoint {
    fn name(&self) -> &'static str {
        "SyncPoint"
    }

    fn access(&self) -> SystemAccess {
        let mut access = SystemAccess::default();
        access.write_world();
        access
    }

    unsafe fn run_unsafe(&mut self, _world: crate::world::UnsafeWorldCell) {}

    fn apply(&mut self, _world: &mut crate::World) {}
}
