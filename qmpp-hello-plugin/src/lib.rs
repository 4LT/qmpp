#![no_std]
#![feature(default_alloc_error_handler)]

extern crate alloc;
extern crate qmpp_shared;
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

    let key = b"message\0".to_vec();
    let mut value_buffer = Vec::<u8>::new();
    let mut value_size = MaybeUninit::<usize>::uninit();

    let status = unsafe {
        QMPP_keyvalue_init_read(0u32, key.as_ptr(), value_size.as_mut_ptr())
    };

    if status == qmpp_shared::SUCCESS {
        let value_size = unsafe { value_size.assume_init() };
        value_buffer.reserve(value_size);

        unsafe { QMPP_keyvalue_read(value_buffer.as_mut_ptr()) };

        unsafe {
            value_buffer.set_len(value_size);
        }

        let value = String::from_utf8(
            value_buffer
                .iter()
                .copied()
                .take_while(|&ch| ch != 0u8)
                .collect::<Vec<u8>>(),
        )
        .unwrap();

        let mesg = format!("Map name: {}", value);

        unsafe {
            QMPP_log_info(mesg.len(), mesg.as_ptr());
        }
    } else {
        let mesg = String::from(match status {
            qmpp_shared::ERROR_KEY_LOOKUP => "Key not found in entity",
            _ => "Unknown status",
        });

        unsafe {
            QMPP_log_error(mesg.len(), mesg.as_ptr());
        }
    }

    let brush_ct = unsafe { QMPP_brush_count(0u32) };

    let mesg = format!("Worldspawn has {} brushes", brush_ct);

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    };

    let mesg = String::from("Targetnames:");

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    };

    let targetname_key = b"targetname\0".to_vec();

    (0..entity_ct).for_each(|ehandle| {
        let status = unsafe {
            QMPP_keyvalue_init_read(
                ehandle,
                targetname_key.as_ptr(),
                value_size.as_mut_ptr(),
            )
        };

        if status == qmpp_shared::SUCCESS {
            let value_size = unsafe { value_size.assume_init() };
            let mut value_buffer = Vec::<u8>::new();
            value_buffer.reserve(value_size);
            unsafe { QMPP_keyvalue_read(value_buffer.as_mut_ptr()) };
            unsafe { value_buffer.set_len(value_size) };

            let value = String::from_utf8(
                value_buffer
                    .iter()
                    .copied()
                    .take_while(|&ch| ch != 0u8)
                    .collect::<Vec<u8>>(),
            )
            .unwrap();

            unsafe {
                QMPP_log_info(value.len(), value.as_ptr());
            }
        }
    });
}

#[allow(non_snake_case)]
extern "C" {
    pub fn QMPP_register(name_len: usize, name_ptr: *const u8);
    pub fn QMPP_entity_count() -> u32;
    pub fn QMPP_log_info(mesg_len: usize, mesg_ptr: *const u8);
    pub fn QMPP_log_error(mesg_len: usize, mesg_ptr: *const u8);

    pub fn QMPP_keyvalue_init_read(
        ehandle: u32,
        key_ptr: *const u8,
        size_ptr: *mut usize,
    ) -> u32;
    pub fn QMPP_keyvalue_read(val_ptr: *mut u8);

    pub fn QMPP_brush_count(ehandle: u32) -> u32;
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
