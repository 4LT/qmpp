use std::ffi::CString;

use wasmtime::{Caller, Extern, Memory};

macro_rules! stub_err {
    (  $ctx:expr, $fun:expr ) => {
        Err(anyhow::anyhow!(
            "\"{}\" not implemented for context \"{}\"",
            $fun,
            $ctx
        ))
    };
}

macro_rules! stub_func {
    (
        $linker:expr,
        $module:expr,
        $ctx:expr,
        $fun:expr,
        ( $( $arg:ty ),* ),
        $ret:ty $(,)?
    ) => {
        $linker.func_wrap(
            $module,
            $fun,
            |$(_:$arg),*| -> anyhow::Result<$ret> {
                stub_err!($ctx, $fun)
            }
        )
    };
    (
        $linker:expr,
        $module:expr,
        $ctx:expr,
        $fun:expr,
        $arg:ty,
        $ret:ty $(,)?
    ) => {
        $linker.func_wrap(
            $module,
            $fun,
            |_:$arg| -> anyhow::Result<$ret> {
                stub_err!($ctx, $fun)
            }
        )
    };
}

/*
macro_rules! stub_import {
    (
        $fun:expr,
        $ctx:expr,
        ( $( $arg:ty ),* ),
        $ret:ty $(,)?
    ) => {
        {
            |$(_:$arg),*| -> core::result::Result<
                $ret,
                $crate::plugin::common::ImportError
            > {
                stub_err!($fun, $ctx)
            }
        }
    };
    (
        $fun:expr,
        $ctx:expr,
        $arg:ty,
        $ret:ty $(,)?
    ) => {
        {
            |_:$arg| -> core::result::Result<
                $ret,
                $crate::plugin::common::ImportError
            > {
                stub_err!($fun, $ctx)
            }
        }
    };
}
*/

pub trait PluginEnv: Clone {
    fn plugin_name(&self) -> &str;
}

#[derive(Copy, Clone)]
enum LogLevel {
    Info,
    Error,
}

pub fn memory_from_caller(
    caller: &mut Caller<'_, impl PluginEnv>,
) -> anyhow::Result<Memory> {
    if let Some(Extern::Memory(memory)) = caller.get_export("memory") {
        anyhow::Ok(memory)
    } else {
        Err(anyhow::anyhow!("Could not obtain extern \"memory\""))
    }
}

pub fn recv_c_string(
    caller: &mut Caller<'_, impl PluginEnv>,
    ptr: i32,
) -> anyhow::Result<CString> {
    let memory = memory_from_caller(caller)?;
    let mut ptr = wasm_to_native_size(ptr);
    let mut bytes = Vec::<u8>::new();
    let mut byte_buf = [0u8];

    loop {
        memory
            .read(&caller, ptr, &mut byte_buf[..])
            .map_err(anyhow::Error::new)?;
        ptr += 1;
        bytes.push(byte_buf[0]);
        if byte_buf[0] == 0u8 {
            break;
        }
    }

    Ok(CString::from_vec_with_nul(bytes).unwrap())
}

pub fn recv_bytes(
    caller: &mut Caller<'_, impl PluginEnv>,
    len: i32,
    ptr: i32,
) -> anyhow::Result<Vec<u8>> {
    let memory = memory_from_caller(caller)?;
    let len = wasm_to_native_size(len);
    let start = wasm_to_native_size(ptr);
    let end = start + len;
    Ok(memory.data(caller)[start..end].into())
}

pub fn send_bytes(
    caller: &mut Caller<'_, impl PluginEnv>,
    ptr: i32,
    payload: &[u8],
) -> anyhow::Result<()> {
    let memory = memory_from_caller(caller)?;
    let ptr = wasm_to_native_size(ptr);
    memory
        .write(caller, ptr, payload)
        .map_err(anyhow::Error::new)?;
    Ok(())
}

fn log(
    mut caller: Caller<'_, impl PluginEnv>,
    mesg_len: i32,
    mesg_ptr: i32,
    level: LogLevel,
) {
    let env = caller.data().clone();

    match recv_bytes(&mut caller, mesg_len, mesg_ptr) {
        Result::Ok(bytes) => match String::from_utf8(bytes) {
            Result::Ok(mesg) => match level {
                LogLevel::Info => {
                    println!("{}\tINFO\t{}", env.plugin_name(), mesg)
                }
                LogLevel::Error => {
                    eprintln!("{}\tERROR\t{}", env.plugin_name(), mesg)
                }
            },
            Result::Err(_) => eprintln!("Invalid UTF-8 in message"),
        },
        Result::Err(_) => eprintln!("Error while receiving bytes"),
    }
}

pub fn log_info(
    caller: Caller<'_, impl PluginEnv>,
    mesg_len: i32,
    mesg_ptr: i32,
) {
    log(caller, mesg_len, mesg_ptr, LogLevel::Info)
}

pub fn log_error(
    caller: Caller<'_, impl PluginEnv>,
    mesg_len: i32,
    mesg_ptr: i32,
) {
    log(caller, mesg_len, mesg_ptr, LogLevel::Error)
}

pub fn wasm_to_native_size(wasm: i32) -> usize {
    usize::try_from(wasm as u32).unwrap()
}

pub fn native_to_wasm_size(native: usize) -> anyhow::Result<i32> {
    u32::try_from(native).map(|int| int as i32).map_err(|_| {
        anyhow::anyhow!(
            "{} is too large to convert from native size to wasm",
            native
        )
    })
}
