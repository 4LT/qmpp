use std::convert::TryFrom;
use std::convert::TryInto;
use std::sync::Arc;
use std::sync::Mutex;

use wasmer::{
    imports, Function, Instance, LazyInit, Memory, Module, WasmerEnv,
};

use quake_util::qmap::{Entity, QuakeMap};

use super::common::{
    log_error, log_info, recv_c_string, send_bytes, PluginEnv,
};
use crate::{abort_plugin, stub_import};

#[derive(WasmerEnv, Clone)]
struct ProcessEnv {
    plugin_name: String,

    #[wasmer(export)]
    memory: LazyInit<Memory>,

    map: Arc<QuakeMap>,
    keyvalue_read_transaction: Arc<Mutex<KeyvalueReadTransaction>>,
}

impl PluginEnv for ProcessEnv {
    fn memory(&self) -> &Memory {
        self.memory.get_ref().unwrap()
    }

    fn plugin_name(&self) -> &str {
        &self.plugin_name
    }
}

enum KeyvalueReadState {
    Closed,
    Open(Vec<u8>),
}

struct KeyvalueReadTransaction {
    state: KeyvalueReadState,
}

impl KeyvalueReadTransaction {
    pub fn new() -> Self {
        Self {
            state: KeyvalueReadState::Closed,
        }
    }

    pub fn open(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let byte_vec = bytes.to_vec();

        match self.state {
            KeyvalueReadState::Closed => {
                self.state = KeyvalueReadState::Open(byte_vec);
                Ok(())
            }
            KeyvalueReadState::Open(_) => Err(()),
        }
    }

    pub fn close(&mut self) -> Result<Vec<u8>, ()> {
        match std::mem::replace(&mut self.state, KeyvalueReadState::Closed) {
            KeyvalueReadState::Closed => Err(()),
            KeyvalueReadState::Open(byte_vec) => {
                self.state = KeyvalueReadState::Closed;
                Ok(byte_vec)
            }
        }
    }
}

pub fn process(module: &Module, map: Arc<QuakeMap>) {
    let process_env = ProcessEnv {
        plugin_name: String::from("hello"),
        memory: LazyInit::new(),
        map,
        keyvalue_read_transaction: Arc::new(Mutex::new(
            KeyvalueReadTransaction::new(),
        )),
    };

    let import_object = imports! {
        "env" => {
            "QMPP_register" => Function::new_native(
                module.store(),
                stub_import!(
                    "QMPP_register",
                    "process",
                    (u32, u32)
                )
            ),

            "QMPP_ehandle_count" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                ehandle_count
            ),
            "QMPP_log_info" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                log_info
            ),
            "QMPP_log_error" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                log_error
            ),

            "QMPP_keyvalue_init_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                keyvalue_init_read
            ),
            "QMPP_keyvalue_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                keyvalue_read
            ),

            "QMPP_bhandle_count" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                bhandle_count
            ),

            "QMPP_shandle_count" => Function::new_native_with_env(
                module.store(),
                process_env,
                shandle_count
            )
        }
    };

    let instance = Instance::new(module, &import_object).unwrap();

    let process = instance.exports.get_function("QMPP_Hook_process").unwrap();
    process.call(&[]).unwrap();
}

fn ehandle_count(env: &ProcessEnv) -> u32 {
    if let Ok(ct) = env.map.entities.len().try_into() {
        ct
    } else {
        abort_plugin!("Too many entities (> ~4B)");
    }
}

fn keyvalue_init_read(
    env: &ProcessEnv,
    ehandle: u32,
    key_ptr: u32,
    size_ptr: u32,
) -> u32 {
    let mem = env.memory.get_ref().unwrap();
    let mut kvrt = env.keyvalue_read_transaction.lock().unwrap();

    let entity = match env.map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => return qmpp_shared::ERROR_ENTITY_LOOKUP,
    };

    let key = match recv_c_string(mem, key_ptr) {
        Ok(key) => key,
        Err(_) => {
            abort_plugin!("Key pointer out of bounds");
        }
    };

    let value = &match entity.edict().get(&key) {
        Some(v) => v,
        None => {
            return qmpp_shared::ERROR_KEY_LOOKUP;
        }
    };

    let value_bytes = value.to_bytes_with_nul();
    let size_bytes = match u32::try_from(value_bytes.len()) {
        Ok(size) => size.to_le_bytes(),
        Err(_) => {
            abort_plugin!("Attempt to send too many bytes to plugin");
        }
    };

    match send_bytes(mem, size_ptr, &size_bytes) {
        Ok(_) => match kvrt.open(value.to_bytes_with_nul()) {
            Ok(_) => qmpp_shared::SUCCESS,
            Err(_) => abort_plugin!("Key-value read transaction already open"),
        },
        Err(_) => abort_plugin!("Failed to send size to plugin"),
    }
}

fn keyvalue_read(env: &ProcessEnv, val_ptr: u32) {
    let mem = env.memory.get_ref().unwrap();
    let mut kvrt = env.keyvalue_read_transaction.lock().unwrap();

    let payload = match kvrt.close() {
        Ok(value_vec) => value_vec,
        Err(_) => {
            abort_plugin!("Key-value read transaction is closed");
        }
    };

    if send_bytes(mem, val_ptr, &payload[..]).is_err() {
        abort_plugin!(
            "Failed to send value with {} bytes to plugin",
            payload.len()
        )
    }
}

fn bhandle_count(env: &ProcessEnv, ehandle: u32, brush_ct_ptr: u32) -> u32 {
    let mem = env.memory.get_ref().unwrap();

    let entity = match env.map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => return qmpp_shared::ERROR_ENTITY_LOOKUP,
    };

    let brush_ct = match entity {
        Entity::Brush(_, brushes) => brushes.len().try_into().unwrap(),
        Entity::Point(_) => 0u32,
    };

    let brush_ct_bytes = brush_ct.to_le_bytes();

    match send_bytes(mem, brush_ct_ptr, &brush_ct_bytes) {
        Ok(_) => qmpp_shared::SUCCESS,
        Err(_) => abort_plugin!("Failed to send brush count to plugin"),
    }
}

fn shandle_count(
    env: &ProcessEnv,
    ehandle: u32,
    brush_idx: u32,
    surface_ct_ptr: u32,
) -> u32 {
    let mem = env.memory.get_ref().unwrap();

    let entity = match env.map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => return qmpp_shared::ERROR_ENTITY_LOOKUP,
    };

    let brush = match entity {
        Entity::Point(_) => {
            return qmpp_shared::ERROR_ENTITY_TYPE;
        },
        Entity::Brush(_, brushes) => {
            match brushes.get(usize::try_from(brush_idx).unwrap()) {
                Some(b) => b,
                None => {
                    return qmpp_shared::ERROR_BRUSH_LOOKUP;
                }
            }
        }
    };

    let surf_ct: u32 = brush.len().try_into().unwrap();
    let surf_ct_bytes = surf_ct.to_le_bytes();

    match send_bytes(mem, surface_ct_ptr, &surf_ct_bytes) {
        Ok(_) => qmpp_shared::SUCCESS,
        Err(_) => abort_plugin!("Failed to send surface count to plugin"),
    }
}
