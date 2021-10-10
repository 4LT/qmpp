use quake_util::qmap::{
    QuakeMap,
    Edict
};

pub enum Patch<T> {
    Leave,
    Delete,
    Modify(T),
}

impl<T> Default for Patch<T> {
    fn default() -> Self {
        Self::Leave
    }
}

pub struct EdictPatch {
    map: HashMap<CString, Patch<CString>>
}

impl EdictPatch {
    fn get(&self, key: &'a CStr) -> Patch<&'a CStr> {
        &self.map.get(key.into()).unwrap_or_default()
    }

    fn set(&mut self, key: &'a CStr, value: CString) {
        self.map.set(key.into(), value);
    }
}

pub struct QuakeMapPatcher {
    entity_patches: Vec<EntityPatcher>,
    qmap: QuakeMap,
}

impl QuakeMapPatcher {
    pub fn new(qmap: QuakeMap) -> Self {
        Self {
            entity_patches: qmap.entities
                .iter()
                .map(|ent| Patch::<EntityPatcher>::Leave)
                .collect(),
            qmap,
        }
    }

    pub fn patch(self, qmap: &mut QuakeMap) -> PatchResult {
        self.entity_patches
    }
}

pub struct EntityPatcher {
    entity: Entity,
    edict_patch: EdictPatch,
}

