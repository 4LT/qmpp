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
                .into_iter()
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
            qmpp_shared::ERROR_ENTITY_LOOKUP => "Entity handle not found",
            qmpp_shared::ERROR_KEY_LOOKUP => "Key not found in entity",
            _ => "Unknown status",
        });

        unsafe {
            QMPP_log_error(mesg.len(), mesg.as_ptr());
        }
    }

    let mesg = String::from("Worldspawn keys & values:");

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }

    let mut keys_size = MaybeUninit::<usize>::uninit();
    let mut key_buffer = Vec::<u8>::new();

    let status = unsafe { QMPP_keys_init_read(0u32, keys_size.as_mut_ptr()) };

    if status == qmpp_shared::SUCCESS {
        let keys_size = unsafe { keys_size.assume_init() };
        key_buffer.reserve(keys_size);

        unsafe { QMPP_keys_read(key_buffer.as_mut_ptr()) };

        unsafe { key_buffer.set_len(keys_size) };

        let keys = key_buffer[..]
            .split(|&ch| ch == 0u8)
            .filter(|slice| slice.len() > 0);

        keys.for_each(|key| {
            let key_c_str = key
                .into_iter()
                .chain(b"\0".into_iter())
                .copied()
                .collect::<Vec<u8>>();

            let mut value_size = MaybeUninit::<usize>::uninit();
            let mut value_buffer = Vec::<u8>::new();

            let status = unsafe {
                QMPP_keyvalue_init_read(
                    0u32,
                    key_c_str.as_ptr(),
                    value_size.as_mut_ptr(),
                )
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
                        .into_iter()
                        .take_while(|&ch| ch != 0)
                        .collect::<Vec<u8>>(),
                )
                .unwrap();

                let key = String::from_utf8(key.to_vec()).unwrap();

                let mesg = format!("{}: {}", key, value);

                unsafe {
                    QMPP_log_info(mesg.len(), mesg.as_ptr());
                }
            }
        });
    } else {
        let mesg = String::from(match status {
            qmpp_shared::ERROR_ENTITY_LOOKUP => "Entity handle not found",
            _ => "Unknown status",
        });

        unsafe {
            QMPP_log_error(mesg.len(), mesg.as_ptr());
        }
    }

    let entity_ct = unsafe { QMPP_ehandle_count() };

    let (brush_ct, surface_ct) = (0..entity_ct)
        .map(|ehandle| {
            let mut ent_brush_ct = MaybeUninit::<u32>::uninit();

            let status = unsafe {
                QMPP_bhandle_count(ehandle, ent_brush_ct.as_mut_ptr())
            };

            if status == qmpp_shared::SUCCESS {
                let ent_brush_ct = unsafe { ent_brush_ct.assume_init() };

                let ent_surface_ct = (0..ent_brush_ct)
                    .map(|b_idx| {
                        let mut brush_surface_ct = MaybeUninit::<u32>::uninit();

                        let status = unsafe {
                            QMPP_shandle_count(
                                ehandle,
                                b_idx,
                                brush_surface_ct.as_mut_ptr(),
                            )
                        };

                        if status == qmpp_shared::SUCCESS {
                            unsafe { brush_surface_ct.assume_init() }
                        } else {
                            0
                        }
                    })
                    .sum();

                (ent_brush_ct, ent_surface_ct)
            } else {
                (0, 0)
            }
        })
        .fold((0, 0), |(b_accum, s_accum), (b_ct, s_ct)| {
            (b_accum + b_ct, s_accum + s_ct)
        });

    let mesg = format!(
        "Found {} surfaces in {} brushes in {} entities",
        surface_ct, brush_ct, entity_ct,
    );

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }
}

#[allow(non_snake_case)]
extern "C" {
    pub fn QMPP_register(name_len: usize, name_ptr: *const u8);
    pub fn QMPP_ehandle_count() -> u32;
    pub fn QMPP_log_info(mesg_len: usize, mesg_ptr: *const u8);
    pub fn QMPP_log_error(mesg_len: usize, mesg_ptr: *const u8);

    pub fn QMPP_keyvalue_init_read(
        ehandle: u32,
        key_ptr: *const u8,
        size_ptr: *mut usize,
    ) -> u32;
    pub fn QMPP_keyvalue_read(val_ptr: *mut u8);

    pub fn QMPP_keys_init_read(ehandle: u32, size_ptr: *mut usize) -> u32;
    pub fn QMPP_keys_read(keys_ptr: *mut u8);

    pub fn QMPP_bhandle_count(ehandle: u32, brush_ct_ptr: *mut u32) -> u32;

    pub fn QMPP_shandle_count(
        ehandle: u32,
        brush_idx: u32,
        surface_ct_ptr: *mut u32,
    ) -> u32;
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
