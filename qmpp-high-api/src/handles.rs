use crate::host_interface::*;
use core::iter::Iterator;

pub fn entity_handles() -> impl Iterator {
    (0..ehandle_count()).map(EntityHandle::new)
}

pub struct EntityHandle {
    entity_idx: u32,
}

impl EntityHandle {
    pub(crate) fn new(entity_idx: u32) -> EntityHandle {
        EntityHandle { entity_idx }
    }
}
