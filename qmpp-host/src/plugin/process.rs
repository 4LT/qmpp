use std::convert::TryFrom;
use std::convert::TryInto;
use std::sync::Arc;
use std::sync::Mutex;

use quake_util::qmap::{Brush, QuakeMap, Surface};

use wasmtime::{Caller, Engine, Linker, Module, Store};

use super::common::{
    log_error, log_info, native_to_wasm_size, recv_c_string, send_bytes,
    wasm_to_native_size, PluginEnv,
};

#[derive(Clone)]
struct ProcessEnv {
    plugin_name: String,
    map: Arc<QuakeMap>,
    keyvalue_read_transaction: Arc<Mutex<Transaction<Vec<u8>>>>,
    keys_read_transaction: Arc<Mutex<Transaction<Vec<u8>>>>,
    texture_read_transaction: Arc<Mutex<Transaction<Vec<u8>>>>,
}

impl PluginEnv for ProcessEnv {
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

pub fn process(engine: &Engine, module: &Module, map: Arc<QuakeMap>) {
    let process_env = ProcessEnv {
        plugin_name: String::from("hello"),
        map,
        keyvalue_read_transaction: Arc::new(Mutex::new(Transaction::new())),
        keys_read_transaction: Arc::new(Mutex::new(Transaction::new())),
        texture_read_transaction: Arc::new(Mutex::new(Transaction::new())),
    };

    let mut store = Store::new(engine, process_env);
    let mut linker = Linker::new(engine);

    stub_func!(linker, "env", "process", "QMPP_register", (i32, i32), (),)
        .unwrap();

    linker.func_wrap("env", "QMPP_log_info", log_info).unwrap();

    linker
        .func_wrap("env", "QMPP_log_error", log_error)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_ehandle_count", ehandle_count)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_bhandle_count", bhandle_count)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_shandle_count", shandle_count)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_entity_exists", entity_exists)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_brush_exists", brush_exists)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_surface_exists", surface_exists)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_keyvalue_init_read", keyvalue_init_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_keyvalue_read", keyvalue_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_keys_init_read", keys_init_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_keys_read", keys_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_texture_init_read", texture_init_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_texture_read", texture_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_half_space_read", half_space_read)
        .unwrap();

    linker
        .func_wrap("env", "QMPP_texture_alignment_read", texture_alignment_read)
        .unwrap();

    linker
        .func_wrap(
            "env",
            "QMPP_texture_alignment_is_valve",
            texture_alignment_is_valve,
        )
        .unwrap();

    linker
        .func_wrap("env", "QMPP_texture_axes_read", texture_axes_read)
        .unwrap();

    let instance = linker.instantiate(&mut store, module).unwrap();

    let process_func =
        instance.get_func(&mut store, "QMPP_Hook_process").unwrap();
    process_func.call(&mut store, &[], &mut []).unwrap();
}

fn ehandle_count(caller: Caller<'_, ProcessEnv>) -> anyhow::Result<i32> {
    let env = caller.data();
    native_to_wasm_size(env.map.entities.len())
}

fn keyvalue_init_read(
    mut caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    key_ptr: i32,
    size_ptr: i32,
) -> anyhow::Result<i32> {
    let env = caller.data().clone();
    let mut kvrt = env.keyvalue_read_transaction.lock().unwrap();

    let idx = usize::try_from(ehandle as u32).unwrap();
    let entity = match env.map.entities.get(idx) {
        Some(ent) => ent,
        None => {
            return Err(anyhow::anyhow!("Bad entity index {}", idx));
        }
    };

    let key = match recv_c_string(&mut caller, key_ptr) {
        Ok(key) => key,
        Err(_) => {
            return Err(anyhow::anyhow!("Key pointer out of bounds"));
        }
    };

    let value = &match entity.edict.get(&key) {
        Some(v) => v,
        None => {
            return Ok(0i32);
        }
    };

    let value_bytes = value.to_bytes_with_nul().to_vec();
    let size_bytes = match u32::try_from(value_bytes.len()) {
        Ok(size) => size.to_le_bytes(),
        Err(_) => {
            return Err(anyhow::anyhow!(
                "Attempted to send too many bytes to plugin",
            ));
        }
    };

    match send_bytes(&mut caller, size_ptr, &size_bytes) {
        Ok(_) => match kvrt.open(value_bytes) {
            Ok(_) => Ok(1i32),
            Err(_) => {
                Err(anyhow::anyhow!("Key-value read transaction already open"))
            }
        },
        Err(_) => Err(anyhow::anyhow!("Failed to send size to plugin")),
    }
}

fn keyvalue_read(
    mut caller: Caller<'_, ProcessEnv>,
    val_ptr: i32,
) -> anyhow::Result<()> {
    let env = caller.data().clone();
    let mut kvrt = env.keyvalue_read_transaction.lock().unwrap();

    let payload = kvrt
        .close()
        .map_err(|_| anyhow::anyhow!("Key-value read transaction is closed"))?;

    if send_bytes(&mut caller, val_ptr, &payload[..]).is_err() {
        Err(anyhow::anyhow!(
            "Failed to send value with {} bytes to plugin",
            payload.len()
        ))
    } else {
        Ok(())
    }
}

fn keys_init_read(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
) -> anyhow::Result<i32> {
    let env = caller.data();
    let mut krt = env.keys_read_transaction.lock().unwrap();

    let entity = match env.map.entities.get(wasm_to_native_size(ehandle)) {
        Some(ent) => ent,
        None => {
            return Err(anyhow::anyhow!("Failed to look up entity"));
        }
    };

    let keys = entity
        .edict
        .keys()
        .flat_map(|key| key.as_bytes_with_nul().iter())
        .copied()
        .collect::<Vec<u8>>();

    let key_count = keys.len().try_into().unwrap();

    match krt.open(keys) {
        Ok(_) => Ok(key_count),
        Err(_) => Err(anyhow::anyhow!("Keys transaction already open")),
    }
}

fn keys_read(
    mut caller: Caller<'_, ProcessEnv>,
    keys_ptr: i32,
) -> anyhow::Result<()> {
    let env = caller.data().clone();
    let mut krt = env.keys_read_transaction.lock().unwrap();

    let payload = krt
        .close()
        .map_err(|_| anyhow::anyhow!("Keys read transaction is closed"))?;

    if send_bytes(&mut caller, keys_ptr, &payload[..]).is_err() {
        Err(anyhow::anyhow!(
            "Failed to send keys in {} bytes to plugin",
            payload.len()
        ))
    } else {
        Ok(())
    }
}

fn bhandle_count(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
) -> anyhow::Result<i32> {
    let entity_idx = usize::try_from(ehandle as u32).unwrap();
    let env = caller.data();

    let entity = match env.map.entities.get(entity_idx) {
        Some(ent) => ent,
        None => {
            return Err(anyhow::anyhow!("Bad entity index {}", entity_idx));
        }
    };

    native_to_wasm_size(entity.brushes.len())
}

fn shandle_count(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
) -> anyhow::Result<i32> {
    let env = caller.data();
    let brush = get_brush(env.map.as_ref(), ehandle, brush_idx)?;
    native_to_wasm_size(brush.len())
}

fn entity_exists(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
) -> anyhow::Result<i32> {
    Ok(if ehandle < ehandle_count(caller)? {
        1i32
    } else {
        0i32
    })
}

fn brush_exists(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
) -> anyhow::Result<i32> {
    let brush_idx = brush_idx as u32;

    Ok(if brush_idx < bhandle_count(caller, ehandle)? as u32 {
        1i32
    } else {
        0i32
    })
}

fn surface_exists(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
) -> anyhow::Result<i32> {
    let surface_idx = surface_idx as u32;

    Ok(
        if surface_idx < shandle_count(caller, ehandle, brush_idx)? as u32 {
            1i32
        } else {
            0i32
        },
    )
}

fn texture_init_read(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
) -> anyhow::Result<i32> {
    let env = caller.data();
    let mut trt = env.texture_read_transaction.lock().unwrap();

    let surface =
        get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx)?;

    let texture = surface.texture.as_bytes_with_nul().to_vec();

    let texture_length = texture.len().try_into().unwrap();

    match trt.open(texture) {
        Ok(_) => Ok(texture_length),
        Err(_) => Err(anyhow::anyhow!("Texture transaction already open")),
    }
}

fn texture_read(
    mut caller: Caller<'_, ProcessEnv>,
    texture_ptr: i32,
) -> anyhow::Result<()> {
    let env = caller.data().clone();
    let mut trt = env.texture_read_transaction.lock().unwrap();

    let payload = match trt.close() {
        Ok(texture) => texture,
        Err(_) => {
            return Err(anyhow::anyhow!("Texture read transaction is closed"));
        }
    };

    if send_bytes(&mut caller, texture_ptr, &payload[..]).is_err() {
        Err(anyhow::anyhow!(
            "Failed to send texture in {} bytes to plugin",
            payload.len()
        ))
    } else {
        Ok(())
    }
}

fn half_space_read(
    mut caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
    ptr: i32,
) -> anyhow::Result<()> {
    let env = caller.data();

    let surface =
        get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx)?;

    let payload = surface
        .half_space
        .into_iter()
        .flat_map(|point| point.into_iter())
        .flat_map(|num| num.to_le_bytes().into_iter())
        .collect::<Vec<u8>>();

    if send_bytes(&mut caller, ptr, &payload[..]).is_err() {
        Err(anyhow::anyhow!(
            "Failed to send half-space in {} bytes to plugin",
            payload.len()
        ))
    } else {
        Ok(())
    }
}

fn texture_alignment_read(
    mut caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
    ptr: i32,
) -> anyhow::Result<()> {
    let env = caller.data();

    let surface =
        get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx)?;

    let alignment = &surface.alignment;

    let payload = alignment
        .offset
        .into_iter()
        .chain([alignment.rotation].into_iter())
        .chain(alignment.scale.into_iter())
        .flat_map(|num| num.to_le_bytes().into_iter())
        .collect::<Vec<u8>>();

    if send_bytes(&mut caller, ptr, &payload[..]).is_err() {
        Err(anyhow::anyhow!(
            "Failed to send alignment in {} bytes to plugin",
            payload.len()
        ))
    } else {
        Ok(())
    }
}

fn texture_alignment_is_valve(
    caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
) -> anyhow::Result<i32> {
    let env = caller.data();

    let surface =
        get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx)?;

    Ok(match &surface.alignment.axes {
        None => 0i32,
        _ => 1i32,
    })
}

fn texture_axes_read(
    mut caller: Caller<'_, ProcessEnv>,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
    ptr: i32,
) -> anyhow::Result<()> {
    let env = caller.data();

    let surface =
        get_surface(env.map.as_ref(), ehandle, brush_idx, surface_idx)?;

    let axes = match &surface.alignment.axes {
        None => {
            return Err(anyhow::anyhow!("No axes on Standard-style surface"));
        }
        Some(axes) => axes,
    };

    let payload = axes
        .iter()
        .flat_map(|axis| axis.iter())
        .flat_map(|num| num.to_le_bytes().into_iter())
        .collect::<Vec<u8>>();

    if send_bytes(&mut caller, ptr, &payload[..]).is_err() {
        Err(anyhow::anyhow!(
            "Failed to send axes in {} bytes to plugin",
            payload.len()
        ))
    } else {
        Ok(())
    }
}

fn get_brush(
    map: &QuakeMap,
    ehandle: i32,
    brush_idx: i32,
) -> anyhow::Result<&Brush> {
    let entity = match map.entities.get(wasm_to_native_size(ehandle)) {
        Some(ent) => ent,
        None => {
            return Err(anyhow::anyhow!("Bad entity index {}", ehandle as u32));
        }
    };

    let brushes = &entity.brushes;

    match brushes.get(wasm_to_native_size(brush_idx)) {
        Some(b) => Ok(b),
        None => Err(anyhow::anyhow!("Bad brush index {}", brush_idx as u32)),
    }
}

fn get_surface(
    map: &QuakeMap,
    ehandle: i32,
    brush_idx: i32,
    surface_idx: i32,
) -> anyhow::Result<&Surface> {
    match get_brush(map, ehandle, brush_idx) {
        Ok(brush) => match brush.get(wasm_to_native_size(surface_idx)) {
            Some(s) => Ok(s),
            None => Err(anyhow::anyhow!(
                "Bad surface index {}",
                surface_idx as u32,
            )),
        },
        Err(failure) => Err(failure),
    }
}
