#![no_std]
#![feature(default_alloc_error_handler)]

extern crate alloc;
extern crate wee_alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn QMPP_Hook_init() {
    let name = String::from("hello");
    unsafe {
        QMPP_register(name.len(), name.as_ptr());
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn QMPP_Hook_process() {
    let entity_ct = unsafe { QMPP_entity_count() };
    let mesg = format!("Found {} entities", entity_ct);
    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }

    const SUCCESS: u32 = 0;
    const ERROR_EHANDLE: u32 = 1;
    const ERROR_KEY_TRANSFER: u32 = 2;
    const ERROR_KEY_LOOKUP: u32 = 3;
    const ERROR_SIZE_TRANSFER: u32 = 4;
    const ERROR_VALUE_TRANSFER: u32 = 5;
    const ERROR_BAD_INIT: u32 = 6;
    const ERROR_BAD_READ: u32 = 7;

    let key = b"message\0".to_vec();
    let mut value_buffer = Vec::<u8>::new();
    let mut value_size = MaybeUninit::<usize>::uninit();
    let mut status = unsafe {
        QMPP_keyvalue_init_read(0usize, key.as_ptr(), value_size.as_mut_ptr())
    };

    if status == SUCCESS {
        let value_size = unsafe { value_size.assume_init() };
        value_buffer.reserve(value_size);
        status = unsafe {
            QMPP_keyvalue_read(value_buffer.as_mut_ptr())
        };
    }

    if status == SUCCESS {
        let value_size = unsafe { value_size.assume_init() };

        unsafe {
            value_buffer.set_len(value_size);
        }

        let value = String::from_utf8(
            value_buffer
                .iter()
                .copied()
                .take_while(|&ch| ch != 0u8)
                .collect::<Vec<u8>>(),
        ).unwrap();

        let mesg = format!("Map name: {}", value);

        unsafe {
            QMPP_log_info(mesg.len(), mesg.as_ptr());
        }
    } else {
        let mesg = String::from(match status {
            ERROR_EHANDLE => "Entity handle out of bounds",
            ERROR_KEY_TRANSFER => "Failed to receive key from plugin",
            ERROR_KEY_LOOKUP => "Key not found in entity",
            ERROR_SIZE_TRANSFER => "Failed to send size to plugin",
            ERROR_VALUE_TRANSFER => "Failed to send value to plugin",
            ERROR_BAD_INIT => "Illegal state for init operation",
            ERROR_BAD_READ => "Illegal state for read operation",
            _ => "Unknown status"
        });

        unsafe {
            QMPP_log_error(mesg.len(), mesg.as_ptr());
        }
    }

    unsafe {
        let name = "this/will/fail";
        QMPP_register(name.len(), name.as_ptr());
    }
}

#[allow(non_snake_case)]
extern "C" {
    pub fn QMPP_register(name_len: usize, name_ptr: *const u8);
    pub fn QMPP_entity_count() -> u32;
    pub fn QMPP_log_info(mesg_len: usize, mesg_ptr: *const u8);
    pub fn QMPP_log_error(mesg_len: usize, mesg_ptr: *const u8);

    pub fn QMPP_keyvalue_init_read(
        ehandle: usize,
        key_ptr: *const u8,
        size_ptr: *mut usize,
    ) -> u32;
    pub fn QMPP_keyvalue_read(
        val_ptr: *mut u8,
    ) -> u32;

    pub fn QMPP_brush_count(
        ehandle: usize,
        size_ptr: *mut usize,
    ) -> u32;
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
