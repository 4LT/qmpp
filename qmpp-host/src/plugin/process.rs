use std::convert::TryFrom;
use std::convert::TryInto;
use std::sync::Arc;
use std::sync::Mutex;

use wasmer::{
    imports, Function, Instance, LazyInit, Memory, Module, WasmerEnv,
};

use quake_util::qmap::{Alignment, Brush, Entity, QuakeMap, Surface};

use super::common::{
    log_error, log_info, recv_c_string, send_bytes, PluginEnv,
};

#[derive(WasmerEnv, Clone)]
struct ProcessEnv {
    plugin_name: String,

    #[wasmer(export)]
    memory: LazyInit<Memory>,

    map: Arc<QuakeMap>,
    keyvalue_read_transaction: Arc<Mutex<Transaction<Vec<u8>>>>,
    keys_read_transaction: Arc<Mutex<Transaction<Vec<u8>>>>,
    texture_read_transaction: Arc<Mutex<Transaction<Vec<u8>>>>,
}

impl PluginEnv for ProcessEnv {
    fn memory(&self) -> &Memory {
        self.memory.get_ref().unwrap()
    }

    fn plugin_name(&self) -> &str {
        &self.plugin_name
    }
}

enum TransactionState<T> {
    Closed,
    Open(T),
}

struct Transaction<T> {
    state: TransactionState<T>,
}

impl<T> Transaction<T> {
    pub fn new() -> Self {
        Self {
            state: TransactionState::Closed,
        }
    }

    pub fn open(&mut self, payload: T) -> Result<(), ()> {
        match self.state {
            TransactionState::Closed => {
                self.state = TransactionState::Open(payload);
                Ok(())
            }
            TransactionState::Open(_) => Err(()),
        }
    }

    pub fn close(&mut self) -> Result<T, ()> {
        match std::mem::replace(&mut self.state, TransactionState::Closed) {
            TransactionState::Closed => Err(()),
            TransactionState::Open(payload) => {
                self.state = TransactionState::Closed;
                Ok(payload)
            }
        }
    }
}

pub fn process(module: &Module, map: Arc<QuakeMap>) {
    let process_env = ProcessEnv {
        plugin_name: String::from("hello"),
        memory: LazyInit::new(),
        map,
        keyvalue_read_transaction: Arc::new(Mutex::new(Transaction::new())),
        keys_read_transaction: Arc::new(Mutex::new(Transaction::new())),
        texture_read_transaction: Arc::new(Mutex::new(Transaction::new())),
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

            "QMPP_keys_init_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                keys_init_read,
            ),
            "QMPP_keys_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                keys_read,
            ),

            "QMPP_bhandle_count" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                bhandle_count
            ),

            "QMPP_shandle_count" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                shandle_count
            ),

            "QMPP_texture_init_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                texture_init_read,
            ),
            "QMPP_texture_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                texture_read,
            ),

            "QMPP_half_space_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                half_space_read
            ),

            "QMPP_texture_alignment_read" => Function::new_native_with_env(
                module.store(),
                process_env.clone(),
                texture_alignment_read
            ),

            "QMPP_texture_axes_read" => Function::new_native_with_env(
                module.store(),
                process_env,
                texture_axes_read
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

    let value_bytes = value.to_bytes_with_nul().to_vec();
    let size_bytes = match u32::try_from(value_bytes.len()) {
        Ok(size) => size.to_le_bytes(),
        Err(_) => {
            abort_plugin!("Attempt to send too many bytes to plugin");
        }
    };

    match send_bytes(mem, size_ptr, &size_bytes) {
        Ok(_) => match kvrt.open(value_bytes) {
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

fn keys_init_read(env: &ProcessEnv, ehandle: u32, size_ptr: u32) -> u32 {
    let mem = env.memory.get_ref().unwrap();
    let mut krt = env.keys_read_transaction.lock().unwrap();

    let entity = match env.map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => return qmpp_shared::ERROR_ENTITY_LOOKUP,
    };

    let keys = entity
        .edict()
        .keys()
        .flat_map(|key| key.as_bytes_with_nul().iter())
        .copied()
        .collect::<Vec<u8>>();

    let size_bytes = match u32::try_from(keys.len()) {
        Ok(size) => size.to_le_bytes(),
        Err(_) => {
            abort_plugin!("Attempt to send too many bytes to plugin");
        }
    };

    match send_bytes(mem, size_ptr, &size_bytes) {
        Ok(_) => match krt.open(keys) {
            Ok(_) => qmpp_shared::SUCCESS,
            Err(_) => abort_plugin!("Keys transaction already open"),
        },
        Err(_) => abort_plugin!("Failed to send size to plugin"),
    }
}

fn keys_read(env: &ProcessEnv, keys_ptr: u32) {
    let mem = env.memory.get_ref().unwrap();
    let mut krt = env.keys_read_transaction.lock().unwrap();

    let payload = match krt.close() {
        Ok(keys) => keys,
        Err(_) => {
            abort_plugin!("Keys read transaction is closed")
        }
    };

    if send_bytes(mem, keys_ptr, &payload[..]).is_err() {
        abort_plugin!(
            "Failed to send keys in {} bytes to plugin",
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

    let brush = match get_brush(env.map.as_ref(), ehandle, brush_idx) {
        Ok(b) => b,
        Err(code) => {
            return code;
        }
    };

    let surf_ct: u32 = brush.len().try_into().unwrap();
    let surf_ct_bytes = surf_ct.to_le_bytes();

    match send_bytes(mem, surface_ct_ptr, &surf_ct_bytes) {
        Ok(_) => qmpp_shared::SUCCESS,
        Err(_) => abort_plugin!("Failed to send surface count to plugin"),
    }
}

fn texture_init_read(
    env: &ProcessEnv,
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
    size_ptr: u32,
) -> u32 {
    let mem = env.memory.get_ref().unwrap();
    let mut trt = env.texture_read_transaction.lock().unwrap();

    let surface =
        match get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx) {
            Ok(s) => s,
            Err(code) => {
                return code;
            }
        };

    let texture = surface.texture.as_bytes_with_nul().to_vec();

    let size_bytes = match u32::try_from(texture.len()) {
        Ok(size) => size.to_le_bytes(),
        Err(_) => {
            abort_plugin!("Attempt to send too many bytes to plugin");
        }
    };

    match send_bytes(mem, size_ptr, &size_bytes) {
        Ok(_) => match trt.open(texture) {
            Ok(_) => qmpp_shared::SUCCESS,
            Err(_) => abort_plugin!("Texture transaction already open"),
        },
        Err(_) => abort_plugin!("Failed to send size to plugin"),
    }
}

fn texture_read(env: &ProcessEnv, texture_ptr: u32) {
    let mem = env.memory.get_ref().unwrap();
    let mut trt = env.texture_read_transaction.lock().unwrap();

    let payload = match trt.close() {
        Ok(texture) => texture,
        Err(_) => {
            abort_plugin!("Texture read transaction is closed")
        }
    };

    if send_bytes(mem, texture_ptr, &payload[..]).is_err() {
        abort_plugin!(
            "Failed to send texture in {} bytes to plugin",
            payload.len()
        )
    }
}

fn half_space_read(
    env: &ProcessEnv,
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
    ptr: u32,
) -> u32 {
    let mem = env.memory.get_ref().unwrap();

    let surface =
        match get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx) {
            Ok(s) => s,
            Err(code) => {
                return code;
            }
        };

    let payload = surface
        .half_space
        .into_iter()
        .flat_map(|point| point.into_iter())
        .flat_map(|num| num.to_le_bytes().into_iter())
        .collect::<Vec<u8>>();

    if send_bytes(mem, ptr, &payload[..]).is_err() {
        abort_plugin!(
            "Failed to send half-space in {} bytes to plugin",
            payload.len()
        )
    }

    qmpp_shared::SUCCESS
}

fn texture_alignment_read(
    env: &ProcessEnv,
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
    ptr: u32,
) -> u32 {
    let mem = env.memory.get_ref().unwrap();

    let surface =
        match get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx) {
            Ok(s) => s,
            Err(code) => {
                return code;
            }
        };

    let alignment = match &surface.alignment {
        Alignment::Standard(align) => align,
        Alignment::Valve220 { base, axes: _ } => base,
    };

    let payload = alignment
        .offset
        .into_iter()
        .chain([alignment.rotation].into_iter())
        .chain(alignment.scale.into_iter())
        .flat_map(|num| num.to_le_bytes().into_iter())
        .collect::<Vec<u8>>();

    if send_bytes(mem, ptr, &payload[..]).is_err() {
        abort_plugin!(
            "Failed to send alignment in {} bytes to plugin",
            payload.len()
        )
    }

    qmpp_shared::SUCCESS
}

fn texture_axes_read(
    env: &ProcessEnv,
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
    ptr: u32,
) -> u32 {
    let mem = env.memory.get_ref().unwrap();

    let surface =
        match get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx) {
            Ok(s) => s,
            Err(code) => {
                return code;
            }
        };

    let axes = match &surface.alignment {
        Alignment::Standard(_) => {
            return qmpp_shared::ERROR_NO_AXES;
        }
        Alignment::Valve220 { base: _, axes } => axes,
    };

    let payload = axes
        .iter()
        .flat_map(|axis| axis.iter())
        .flat_map(|num| num.to_le_bytes().into_iter())
        .collect::<Vec<u8>>();

    if send_bytes(mem, ptr, &payload[..]).is_err() {
        abort_plugin!(
            "Failed to send axes in {} bytes to plugin",
            payload.len()
        )
    }

    qmpp_shared::SUCCESS
}

fn get_brush(
    map: &QuakeMap,
    ehandle: u32,
    brush_idx: u32,
) -> Result<&Brush, u32> {
    let entity = match map.entities.get(usize::try_from(ehandle).unwrap()) {
        Some(ent) => ent,
        None => {
            return Err(qmpp_shared::ERROR_ENTITY_LOOKUP);
        }
    };

    let brushes = match entity {
        Entity::Brush(_, brushes) => brushes,
        Entity::Point(_) => {
            return Err(qmpp_shared::ERROR_ENTITY_TYPE);
        }
    };

    match brushes.get(usize::try_from(brush_idx).unwrap()) {
        Some(b) => Ok(b),
        None => Err(qmpp_shared::ERROR_BRUSH_LOOKUP),
    }
}

fn get_surface(
    map: &QuakeMap,
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
) -> Result<&Surface, u32> {
    match get_brush(map, ehandle, brush_idx) {
        Ok(brush) => match brush.get(usize::try_from(surface_idx).unwrap()) {
            Some(s) => Ok(s),
            None => Err(qmpp_shared::ERROR_SURFACE_LOOKUP),
        },
        Err(code) => Err(code),
    }
}
