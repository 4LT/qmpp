use std::ffi::CString;
use std::fmt::Display;

use wasmer::WasmerEnv;

use wasmer::{Memory, MemoryView};

macro_rules! stub_err {
    ( $fun:expr, $ctx:expr ) => {
        $crate::plugin::common::error_with_message(format!(
            "\"{}\" not implemented for context \"{}\"",
            $fun, $ctx
        ))
    };
}

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

/*
pub fn stub_import<'a, A, R, S: Display>(
    fun: S,
    ctx: S,
) -> impl Fn(A) -> R {
    move |_: A| -> R {
        panic!(
            "\"{}\" not implemented for context \"{}\"",
            fun,
            ctx
        )
    }
}
*/

pub fn error_with_message<T>(msg: impl Display) -> Result<T, ImportError> {
    Err(ImportError::new(msg))
}

pub trait PluginEnv: WasmerEnv + Clone {
    fn memory(&self) -> &Memory;
    fn plugin_name(&self) -> &str;
}

pub enum TransferError {
    Overflow,
}

#[derive(Debug)]
pub struct ImportError {
    msg: String,
}

impl ImportError {
    pub fn new(msg: impl Display) -> Self {
        Self {
            msg: format!("{}", msg),
        }
    }
}

impl Display for ImportError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.msg)?;
        Ok(())
    }
}

impl std::error::Error for ImportError {}

#[derive(Copy, Clone)]
enum LogLevel {
    Info,
    Error,
}

pub fn recv_c_string(mem: &Memory, ptr: u32) -> Result<CString, TransferError> {
    let index = ptr as usize;
    let view: MemoryView<u8> = mem.view();
    let view_slice = &view[index..];

    let bytes: Vec<u8> = view_slice
        .iter()
        .map(|cell| cell.get())
        .take_while(|&ch| ch != 0u8)
        .collect();

    if view_slice.len() <= bytes.len() {
        return Err(TransferError::Overflow);
    }

    Ok(CString::new(bytes).unwrap())
}

pub fn recv_bytes(
    mem: &Memory,
    len: u32,
    ptr: u32,
) -> Result<Vec<u8>, TransferError> {
    let start = ptr as usize;
    let end = start + len as usize;
    let view: MemoryView<u8> = mem.view();

    Ok((&view[start..end]).iter().map(|cell| cell.get()).collect())
}

pub fn send_bytes(
    mem: &Memory,
    ptr: u32,
    payload: &[u8],
) -> Result<(), TransferError> {
    let index = ptr as usize;
    let view: MemoryView<u8> = mem.view();
    let view_slice = &view[index..];

    if view_slice.len() < payload.len() {
        return Err(TransferError::Overflow);
    }

    view_slice
        .iter()
        .zip(payload.iter())
        .for_each(|(cell, &byte)| cell.set(byte));

    Ok(())
}

fn log<E: PluginEnv>(env: &E, mesg_len: u32, mesg_ptr: u32, level: LogLevel) {
    match recv_bytes(env.memory(), mesg_len, mesg_ptr) {
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

pub fn log_info<E: PluginEnv>(env: &E, mesg_len: u32, mesg_ptr: u32) {
    log(env, mesg_len, mesg_ptr, LogLevel::Info)
}

pub fn log_error<E: PluginEnv>(env: &E, mesg_len: u32, mesg_ptr: u32) {
    log(env, mesg_len, mesg_ptr, LogLevel::Error)
}
