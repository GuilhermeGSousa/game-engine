use crate::system::access::SystemAccess;

#[derive(Default)]
pub(crate) struct ScheduledBatch {
    access: SystemAccess,
    system_indices: Vec<usize>,
}

impl ScheduledBatch {
    pub(crate) fn is_disjoint_from(&self, other_access: &SystemAccess) -> bool {
        SystemAccess::are_disjoint(&self.access, other_access)
    }

    pub(crate) fn push(&mut self, system_index: usize, access: SystemAccess) {
        self.access.combine(access);
        self.system_indices.push(system_index);
    }
}
