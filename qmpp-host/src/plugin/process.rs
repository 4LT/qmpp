use std::convert::TryFrom;
use std::sync::Arc;
use std::sync::Mutex;

use wasmer::{
    imports, Function, Instance, LazyInit, Memory, Module, WasmerEnv,
};

use quake_util::qmap::QuakeMap;

use crate::stub_import;
use super::common::{
    log_error, log_info, recv_c_string, send_bytes, PluginEnv,
};

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

            "QMPP_entity_count" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                entity_count
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
                process_env,
                keyvalue_read
            ),

            /*
            "QMPP_brush_count" => Function::new_native_with_env(
                &store,
                process_env,
                brush_count
            )
            */
        }
    };

    let instance = Instance::new(module, &import_object).unwrap();

    let process = instance.exports.get_function("QMPP_Hook_process").unwrap();
    process.call(&[]).unwrap();
}

fn entity_count(env: &ProcessEnv) -> u32 {
    env.map.entities.len() as u32
}

fn keyvalue_init_read(
    env: &ProcessEnv,
    ehandle: u32,
    key_ptr: u32,
    size_ptr: u32,
) -> u32 {
    let success = 0u32;
    let error_ehandle = 1u32;
    let error_key_transfer = 2u32;
    let error_key_lookup = 3u32;
    let error_size_transfer = 4u32;
    let error_bad_init = 6u32;
    let mem = env.memory.get_ref().unwrap();
    let mut kvrt = env.keyvalue_read_transaction.lock().unwrap();

    let entity = match env.map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => {
            return error_ehandle;
        }
    };

    let key = match recv_c_string(mem, key_ptr) {
        Ok(key) => key,
        Err(_) => {
            return error_key_transfer;
        }
    };

    let value = &match entity.edict().get(&key) {
        Some(v) => v,
        None => {
            return error_key_lookup;
        }
    };

    let value_bytes = value.to_bytes_with_nul();
    let size_bytes = &match u32::try_from(value_bytes.len()) {
        Ok(size) => size.to_le_bytes(),
        Err(_) => {
            return error_size_transfer;
        }
    };

    match send_bytes(mem, size_ptr, size_bytes) {
        Ok(_) => match kvrt.open(value.to_bytes_with_nul()) {
            Ok(_) => success,
            Err(_) => error_bad_init,
        },
        Err(_) => error_size_transfer,
    }
}

fn keyvalue_read(env: &ProcessEnv, val_ptr: u32) -> u32 {
    let success = 0u32;
    let error_value_transfer = 5u32;
    let error_bad_read = 7u32;

    let mem = env.memory.get_ref().unwrap();
    let mut kvrt = env.keyvalue_read_transaction.lock().unwrap();

    let payload = match kvrt.close() {
        Ok(value_vec) => value_vec,
        Err(_) => {
            return error_bad_read;
        }
    };

    match send_bytes(mem, val_ptr, &payload[..]) {
        Ok(_) => success,
        Err(_) => error_value_transfer,
    }
}

/*
fn brush_count(env: &ProcessEnv, ehandle: u32) -> u32 {
    let success = 0u32;
    let error_ehandle = 1u32;

    let entity = match env.map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => {
            return error_ehandle;
        }
    };

    match entity {
        Entity::Brush(_, brushes) => brushes.len() as u32,
        Entity::Point(
}*/
