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

const HALF_SPACE_POINTS: usize = 3;
const VECTOR_3D_COORDS: usize = 3;
const OFFSET_COMPONENTS: usize = 2;
const ROTATION_COMPONENTS: usize = 1;
const SCALE_COMPONENTS: usize = 2;

type HalfSpace = [[f64; VECTOR_3D_COORDS]; HALF_SPACE_POINTS];
type Alignment =
    [f64; OFFSET_COMPONENTS + ROTATION_COMPONENTS + SCALE_COMPONENTS];
type Vec3 = [f64; VECTOR_3D_COORDS];

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

    let success = unsafe {
        QMPP_keyvalue_init_read(0u32, key.as_ptr(), value_size.as_mut_ptr())
    };

    if success != 0u32 {
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
        let mesg = String::from("Key not found in entity");

        unsafe {
            QMPP_log_error(mesg.len(), mesg.as_ptr());
        }
    }

    let mesg = String::from("Worldspawn keys & values:");

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }

    let keys_size = unsafe { QMPP_keys_init_read(0u32) };
    let mut key_buffer = Vec::<u8>::new();
    key_buffer.reserve(keys_size);

    unsafe { QMPP_keys_read(key_buffer.as_mut_ptr()) };

    unsafe { key_buffer.set_len(keys_size - 1) };

    let keys = key_buffer[..].split(|&ch| ch == 0u8);

    keys.for_each(|key| {
        let key_c_str =
            key.iter().chain(b"\0".iter()).copied().collect::<Vec<u8>>();

        let mut value_size = MaybeUninit::<usize>::uninit();
        let mut value_buffer = Vec::<u8>::new();

        let success = unsafe {
            QMPP_keyvalue_init_read(
                0u32,
                key_c_str.as_ptr(),
                value_size.as_mut_ptr(),
            )
        };

        if success != 0u32 {
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

    let entity_ct = unsafe { QMPP_ehandle_count() };

    let (brush_ct, surface_ct) = (0..entity_ct)
        .map(|ehandle| {
            let ent_brush_ct = unsafe { QMPP_bhandle_count(ehandle) };

            let ent_surface_ct: u32 = (0..ent_brush_ct)
                .map(|b_idx| unsafe { QMPP_shandle_count(ehandle, b_idx) })
                .sum();

            (ent_brush_ct, ent_surface_ct)
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

    let mesg = "Button textures:";

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }

    let classname_key = b"classname\0".to_vec();

    (0..entity_ct)
        .filter(|&ehandle| {
            let mut classname_size = MaybeUninit::<usize>::uninit();
            let mut classname = Vec::<u8>::new();

            let success = unsafe {
                QMPP_keyvalue_init_read(
                    ehandle,
                    classname_key.as_ptr(),
                    classname_size.as_mut_ptr(),
                )
            };

            if success != 0u32 {
                let classname_size = unsafe { classname_size.assume_init() };

                classname.reserve(classname_size);

                unsafe {
                    QMPP_keyvalue_read(classname.as_mut_ptr());
                    classname.set_len(classname_size);
                }

                &classname[..] == b"func_button\0"
            } else {
                false
            }
        })
        .map(|ehandle| {
            let bhandle_ct = unsafe { QMPP_bhandle_count(ehandle) };
            (0..bhandle_ct).map(move |b_idx| (ehandle, b_idx))
        })
        .flatten()
        .filter_map(|(ehandle, b_idx)| {
            let shandle_ct = unsafe { QMPP_shandle_count(ehandle, b_idx) };
            Some((ehandle, b_idx, shandle_ct))
        })
        .flat_map(|(ehandle, b_idx, shandle_ct)| {
            (0..shandle_ct).map(move |s_idx| (ehandle, b_idx, s_idx))
        })
        .filter_map(|(ehandle, b_idx, s_idx)| {
            let mut texture = Vec::<u8>::new();
            let mut half_space = MaybeUninit::<HalfSpace>::uninit();
            let mut alignment = MaybeUninit::<Alignment>::uninit();
            let mut axes = MaybeUninit::<[Vec3; 2]>::uninit();

            let tex_size =
                unsafe { QMPP_texture_init_read(ehandle, b_idx, s_idx) };

            texture.reserve(tex_size);

            unsafe {
                QMPP_texture_read(texture.as_mut_ptr());
                texture.set_len(tex_size);
            }

            unsafe {
                QMPP_half_space_read(
                    ehandle,
                    b_idx,
                    s_idx,
                    half_space.as_mut_ptr(),
                )
            };

            let half_space = unsafe { half_space.assume_init() };

            unsafe {
                QMPP_texture_alignment_read(
                    ehandle,
                    b_idx,
                    s_idx,
                    alignment.as_mut_ptr(),
                )
            };

            let alignment = unsafe { alignment.assume_init() };

            let is_valve = unsafe {
                QMPP_texture_alignment_is_valve(ehandle, b_idx, s_idx)
            };

            let axes = if is_valve != 0u32 {
                unsafe {
                    QMPP_texture_axes_read(
                        ehandle,
                        b_idx,
                        s_idx,
                        axes.as_mut_ptr(),
                    )
                };

                Some(unsafe { axes.assume_init() })
            } else {
                None
            };

            Some((
                half_space,
                String::from_utf8(
                    texture.into_iter().take_while(|&ch| ch != 0u8).collect(),
                )
                .unwrap(),
                alignment,
                axes,
            ))
        })
        .for_each(|(half_space, texture, alignment, axes)| {
            let mut points = half_space
                .into_iter()
                .map(|[x, y, z]| format!("{:5} {:5} {:5}", x, y, z));

            let mesg = format!(
                "({}) ({}) ({}):",
                points.next().unwrap(),
                points.next().unwrap(),
                points.next().unwrap(),
            );
            unsafe {
                QMPP_log_info(mesg.len(), mesg.as_ptr());
            }

            if let Some(axes) = axes {
                let mesg = format!(
                    "  U: <{:2.3} {:2.3} {:2.3}> V: <{:2.3} {:2.3} {:2.3}>",
                    axes[0][0],
                    axes[0][1],
                    axes[0][2],
                    axes[1][0],
                    axes[1][1],
                    axes[1][2]
                );
                unsafe {
                    QMPP_log_info(mesg.len(), mesg.as_ptr());
                }
            }

            let mesg = format!(
                "  texture: {} offset: ({:3.1} {:3.1}) \
                rotation: {:4.3} scale: ({:2.2} {:2.2})",
                texture,
                alignment[0],
                alignment[1],
                alignment[2],
                alignment[3],
                alignment[4]
            );
            unsafe {
                QMPP_log_info(mesg.len(), mesg.as_ptr());
            }
        });
}

#[allow(non_snake_case, improper_ctypes)]
extern "C" {
    pub fn QMPP_register(name_len: usize, name_ptr: *const u8);

    pub fn QMPP_log_info(mesg_len: usize, mesg_ptr: *const u8);
    pub fn QMPP_log_error(mesg_len: usize, mesg_ptr: *const u8);

    pub fn QMPP_ehandle_count() -> u32;
    pub fn QMPP_bhandle_count(ehandle: u32) -> u32;
    pub fn QMPP_shandle_count(ehandle: u32, brush_idx: u32) -> u32;

    pub fn QMPP_entity_exists(ehandle: u32) -> u32;
    pub fn QMPP_brush_exists(ehandle: u32, brush_idx: u32) -> u32;
    pub fn QMPP_surface_exists(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
    ) -> u32;

    pub fn QMPP_keyvalue_init_read(
        ehandle: u32,
        key_ptr: *const u8,
        size_ptr: *mut usize,
    ) -> u32;
    pub fn QMPP_keyvalue_read(val_ptr: *mut u8);

    pub fn QMPP_keys_init_read(ehandle: u32) -> usize;
    pub fn QMPP_keys_read(keys_ptr: *mut u8);

    pub fn QMPP_texture_init_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
    ) -> usize;
    pub fn QMPP_texture_read(texture_ptr: *mut u8);

    pub fn QMPP_half_space_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut HalfSpace,
    );

    pub fn QMPP_texture_alignment_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut Alignment,
    );

    pub fn QMPP_texture_alignment_is_valve(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
    ) -> u32;

    pub fn QMPP_texture_axes_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut [Vec3; 2],
    );
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
