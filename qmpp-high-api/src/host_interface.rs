use qmpp_shared::LowApiCode;

use alloc::vec::Vec;
use core::mem::MaybeUninit;
use cstr_core::{CStr, CString};
use quake_util::qmap::{Alignment, BaseAlignment, HalfSpace};

const HALF_SPACE_POINTS: usize = 3;
const VECTOR_3D_COORDS: usize = 3;
const OFFSET_COMPONENTS: usize = 2;
const ROTATION_COMPONENTS: usize = 1;
const SCALE_COMPONENTS: usize = 2;

type RawVec3 = [f64; VECTOR_3D_COORDS];
type RawHalfSpace = [RawVec3; HALF_SPACE_POINTS];
type RawAlignment =
    [f64; OFFSET_COMPONENTS + ROTATION_COMPONENTS + SCALE_COMPONENTS];
type RawAxes = [RawVec3; 2];

#[allow(non_snake_case, improper_ctypes)]
extern "C" {
    fn QMPP_register(name_len: usize, name_ptr: *const u8);
    fn QMPP_ehandle_count() -> u32;
    fn QMPP_log_info(mesg_len: usize, mesg_ptr: *const u8);
    fn QMPP_log_error(mesg_len: usize, mesg_ptr: *const u8);

    fn QMPP_keyvalue_init_read(
        ehandle: u32,
        key_ptr: *const u8,
        size_ptr: *mut usize,
    ) -> LowApiCode;
    fn QMPP_keyvalue_read(val_ptr: *mut u8);

    fn QMPP_keys_init_read(ehandle: u32, size_ptr: *mut usize) -> LowApiCode;
    fn QMPP_keys_read(keys_ptr: *mut u8);

    fn QMPP_bhandle_count(ehandle: u32, brush_ct_ptr: *mut u32) -> LowApiCode;

    fn QMPP_shandle_count(
        ehandle: u32,
        brush_idx: u32,
        surface_ct_ptr: *mut u32,
    ) -> LowApiCode;

    fn QMPP_texture_init_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        size_ptr: *mut usize,
    ) -> LowApiCode;
    fn QMPP_texture_read(texture_ptr: *mut u8);

    fn QMPP_half_space_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut RawHalfSpace,
    ) -> LowApiCode;

    fn QMPP_texture_alignment_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut RawAlignment,
    ) -> LowApiCode;

    fn QMPP_texture_axes_read(
        ehandle: u32,
        brush_idx: u32,
        surface_idx: u32,
        ptr: *mut RawAxes,
    ) -> LowApiCode;
}

pub fn register(mesg: &str) {
    unsafe {
        QMPP_register(mesg.len(), mesg.as_ptr());
    }
}

pub fn ehandle_count() -> u32 {
    unsafe { QMPP_ehandle_count() }
}

pub fn log_info(mesg: &str) {
    unsafe {
        QMPP_log_info(mesg.len(), mesg.as_ptr());
    }
}

pub fn log_error(mesg: &str) {
    unsafe {
        QMPP_log_error(mesg.len(), mesg.as_ptr());
    }
}

pub fn read_keyvalue(ehandle: u32, key: &CStr) -> Result<CString, LowApiCode> {
    let key_bytes = key.to_bytes_with_nul();
    let mut value_size = MaybeUninit::<usize>::uninit();
    let mut value_buffer = Vec::<u8>::new();

    let status = unsafe {
        QMPP_keyvalue_init_read(
            ehandle,
            key_bytes.as_ptr(),
            value_size.as_mut_ptr(),
        )
    };

    if status == LowApiCode::Success {
        let value_size = unsafe { value_size.assume_init() };
        value_buffer.reserve(value_size);

        unsafe { QMPP_keyvalue_read(value_buffer.as_mut_ptr()) };

        unsafe {
            // exclude null terminator
            value_buffer.set_len(value_size - 1);
        }

        let value = unsafe { CString::from_vec_unchecked(value_buffer) };
        Ok(value)
    } else {
        Err(status)
    }
}

pub fn read_keys(ehandle: u32) -> Result<Vec<CString>, LowApiCode> {
    let mut keys_size = MaybeUninit::<usize>::uninit();
    let mut key_buffer = Vec::<u8>::new();

    let status =
        unsafe { QMPP_keys_init_read(ehandle, keys_size.as_mut_ptr()) };

    if status == LowApiCode::Success {
        let keys_size = unsafe { keys_size.assume_init() };
        key_buffer.reserve(keys_size);

        unsafe { QMPP_keys_read(key_buffer.as_mut_ptr()) };

        unsafe {
            // exclude last null terminator
            key_buffer.set_len(keys_size - 1);
        }

        let keys = key_buffer[..]
            .split(|&ch| ch == 0u8)
            .map(|key_bytes| unsafe {
                CString::from_vec_unchecked(key_bytes.into())
            })
            .collect();

        Ok(keys)
    } else {
        Err(status)
    }
}

pub fn bhandle_count(ehandle: u32) -> Result<u32, LowApiCode> {
    let mut brush_idx_ct = MaybeUninit::<u32>::uninit();

    let status =
        unsafe { QMPP_bhandle_count(ehandle, brush_idx_ct.as_mut_ptr()) };

    if status == LowApiCode::Success {
        Ok(unsafe { brush_idx_ct.assume_init() })
    } else {
        Err(status)
    }
}

pub fn shandle_count(ehandle: u32, brush_idx: u32) -> Result<u32, LowApiCode> {
    let mut surface_idx_ct = MaybeUninit::<u32>::uninit();

    let status = unsafe {
        QMPP_shandle_count(ehandle, brush_idx, surface_idx_ct.as_mut_ptr())
    };

    if status == LowApiCode::Success {
        Ok(unsafe { surface_idx_ct.assume_init() })
    } else {
        Err(status)
    }
}

pub fn read_texture(
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
) -> Result<CString, LowApiCode> {
    let mut texture_name_size = MaybeUninit::<usize>::uninit();
    let mut texture_name_buffer = Vec::<u8>::new();

    let status = unsafe {
        QMPP_texture_init_read(
            ehandle,
            brush_idx,
            surface_idx,
            texture_name_size.as_mut_ptr(),
        )
    };

    if status == LowApiCode::Success {
        let texture_name_size = unsafe { texture_name_size.assume_init() };
        texture_name_buffer.reserve(texture_name_size);

        unsafe { QMPP_texture_read(texture_name_buffer.as_mut_ptr()) };

        unsafe {
            // exclude null terminator
            texture_name_buffer.set_len(texture_name_size - 1);
        }

        let texture =
            unsafe { CString::from_vec_unchecked(texture_name_buffer) };

        Ok(texture)
    } else {
        Err(status)
    }
}

pub fn read_alignment(
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
) -> Result<Alignment, LowApiCode> {
    let mut texture_alignment = MaybeUninit::<RawAlignment>::uninit();
    let mut axes = MaybeUninit::<RawAxes>::uninit();

    let status = unsafe {
        QMPP_texture_alignment_read(
            ehandle,
            brush_idx,
            surface_idx,
            texture_alignment.as_mut_ptr(),
        )
    };

    let alignment = if status == LowApiCode::Success {
        unsafe { texture_alignment.assume_init() }
    } else {
        return Err(status);
    };

    let alignment = BaseAlignment {
        offset: [alignment[0], alignment[1]],
        rotation: alignment[2],
        scale: [alignment[3], alignment[4]],
    };

    let status = unsafe {
        QMPP_texture_axes_read(
            ehandle,
            brush_idx,
            surface_idx,
            axes.as_mut_ptr(),
        )
    };

    if status == LowApiCode::Success {
        let axes = unsafe { axes.assume_init() };
        Ok(Alignment::Valve220(alignment, axes))
    } else if status == LowApiCode::NoAxesError {
        Ok(Alignment::Standard(alignment))
    } else {
        Err(status)
    }
}

pub fn read_half_space(
    ehandle: u32,
    brush_idx: u32,
    surface_idx: u32,
) -> Result<HalfSpace, LowApiCode> {
    let mut half_space = MaybeUninit::<RawHalfSpace>::uninit();

    let status = unsafe {
        QMPP_half_space_read(
            ehandle,
            brush_idx,
            surface_idx,
            half_space.as_mut_ptr(),
        )
    };

    if status == LowApiCode::Success {
        Ok(unsafe { half_space.assume_init() })
    } else {
        Err(status)
    }
}
