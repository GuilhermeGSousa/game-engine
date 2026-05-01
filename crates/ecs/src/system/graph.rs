use petgraph::graph::DiGraph;

use crate::system::{access::SystemAccess, BoxedSystem};

pub(crate) type SystemDependencyGraph = DiGraph<SystemNode, ()>;

pub struct SystemNode {
    pub(crate) system: BoxedSystem,
    cached_access: SystemAccess,
}

impl SystemNode {
    pub(crate) fn new(system: BoxedSystem) -> Self {
        let cached_access = system.access();
        Self {
            system,
            cached_access,
        }
    }

    pub(crate) fn access(&self) -> &SystemAccess {
        &self.cached_access
    }
}
