#![no_std]
#![feature(default_alloc_error_handler)]

extern crate alloc;
extern crate wee_alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use core::panic::PanicInfo;

use qmpp_shared::LowApiCode;

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

    let status = unsafe {
        QMPP_keyvalue_init_read(0u32, key.as_ptr(), value_size.as_mut_ptr())
    };

    if status == LowApiCode::Success {
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
            LowApiCode::EntityLookupError => "Entity handle not found",
            LowApiCode::KeyLookupError => "Key not found in entity",
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

    if status == LowApiCode::Success {
        let keys_size = unsafe { keys_size.assume_init() };
        key_buffer.reserve(keys_size);

        unsafe { QMPP_keys_read(key_buffer.as_mut_ptr()) };

        unsafe { key_buffer.set_len(keys_size - 1) };

        let keys = key_buffer[..].split(|&ch| ch == 0u8);

        keys.for_each(|key| {
            let key_c_str =
                key.iter().chain(b"\0".iter()).copied().collect::<Vec<u8>>();

            let mut value_size = MaybeUninit::<usize>::uninit();
            let mut value_buffer = Vec::<u8>::new();

            let status = unsafe {
                QMPP_keyvalue_init_read(
                    0u32,
                    key_c_str.as_ptr(),
                    value_size.as_mut_ptr(),
                )
            };

            if status == LowApiCode::Success {
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
            LowApiCode::EntityLookupError => "Entity handle not found",
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

            if status == LowApiCode::Success {
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

                        if status == LowApiCode::Success {
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

    let mesg = "Button textures:";

    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }

    let classname_key = b"classname\0".to_vec();

    (0..entity_ct)
        .filter(|&ehandle| {
            let mut classname_size = MaybeUninit::<usize>::uninit();
            let mut classname = Vec::<u8>::new();

            let status = unsafe {
                QMPP_keyvalue_init_read(
                    ehandle,
                    classname_key.as_ptr(),
                    classname_size.as_mut_ptr(),
                )
            };

            if status == LowApiCode::Success {
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
        .filter_map(|ehandle| {
            let mut bhandle_ct = MaybeUninit::<u32>::uninit();

            let status =
                unsafe { QMPP_bhandle_count(ehandle, bhandle_ct.as_mut_ptr()) };

            if status == LowApiCode::Success {
                Some((ehandle, unsafe { bhandle_ct.assume_init() }))
            } else {
                None
            }
        })
        .flat_map(|(ehandle, bhandle_ct)| {
            (0..bhandle_ct).map(move |b_idx| (ehandle, b_idx))
        })
        .filter_map(|(ehandle, b_idx)| {
            let mut shandle_ct = MaybeUninit::<u32>::uninit();

            let status = unsafe {
                QMPP_shandle_count(ehandle, b_idx, shandle_ct.as_mut_ptr())
            };

            if status == LowApiCode::Success {
                Some((ehandle, b_idx, unsafe { shandle_ct.assume_init() }))
            } else {
                None
            }
        })
        .flat_map(|(ehandle, b_idx, shandle_ct)| {
            (0..shandle_ct).map(move |s_idx| (ehandle, b_idx, s_idx))
        })
        .filter_map(|(ehandle, b_idx, s_idx)| {
            let mut tex_size = MaybeUninit::<usize>::uninit();
            let mut texture = Vec::<u8>::new();
            let mut half_space = MaybeUninit::<HalfSpace>::uninit();
            let mut alignment = MaybeUninit::<Alignment>::uninit();
            let mut axes = MaybeUninit::<[Vec3; 2]>::uninit();

            let status = unsafe {
                QMPP_texture_init_read(
                    ehandle,
                    b_idx,
                    s_idx,
                    tex_size.as_mut_ptr(),
                )
            };

            if status != LowApiCode::Success {
                return None;
            }

            let tex_size = unsafe { tex_size.assume_init() };

            texture.reserve(tex_size);

            unsafe {
                QMPP_texture_read(texture.as_mut_ptr());
                texture.set_len(tex_size);
            }

            let status = unsafe {
                QMPP_half_space_read(
                    ehandle,
                    b_idx,
                    s_idx,
                    half_space.as_mut_ptr(),
                )
            };

            if status != LowApiCode::Success {
                return None;
            }

            let half_space = unsafe { half_space.assume_init() };

            let status = unsafe {
                QMPP_texture_alignment_read(
                    ehandle,
                    b_idx,
                    s_idx,
                    alignment.as_mut_ptr(),
                )
            };

            if status != LowApiCode::Success {
                return None;
            }

            let alignment = unsafe { alignment.assume_init() };

            let status = unsafe {
                QMPP_texture_axes_read(ehandle, b_idx, s_idx, axes.as_mut_ptr())
            };

            let axes = match status {
                LowApiCode::Success => Some(unsafe { axes.assume_init() }),
                LowApiCode::NoAxesError => None,
                _ => {
                    return None;
                }
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
    pub fn QMPP_ehandle_count() -> u32;
    pub fn QMPP_log_info(mesg_len: usize, mesg_ptr: *const u8);
    pub fn QMPP_log_error(mesg_len: usize, mesg_ptr: *const u8);

    pub fn QMPP_keyvalue_init_read(
        ehandle: u32,
        key_ptr: *const u8,
        size_ptr: *mut usize,
    ) -> LowApiCode;
    pub fn QMPP_keyvalue_read(val_ptr: *mut u8);

    pub fn QMPP_keys_init_read(
        ehandle: u32,
        size_ptr: *mut usize,
    ) -> LowApiCode;
    pub fn QMPP_keys_read(keys_ptr: *mut u8);

    pub fn QMPP_bhandle_count(
        ehandle: u32,
        brush_ct_ptr: *mut u32,
    ) -> LowApiCode;

    pub fn QMPP_shandle_count(
        ehandle: u32,
        brush_idx: u32,
        surface_ct_ptr: *mut u32,
    ) -> LowApiCode;

    pub fn QMPP_texture_init_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        size_ptr: *mut usize,
    ) -> LowApiCode;
    pub fn QMPP_texture_read(texture_ptr: *mut u8);

    pub fn QMPP_half_space_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut HalfSpace,
    ) -> LowApiCode;

    pub fn QMPP_texture_alignment_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut Alignment,
    ) -> LowApiCode;

    pub fn QMPP_texture_axes_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut [Vec3; 2],
    ) -> LowApiCode;
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
