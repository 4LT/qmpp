#![cfg_attr(target_family = "wasm", no_std)]

#[repr(u32)]
#[non_exhaustive]
#[derive(Eq, PartialEq, Clone, Copy)]
pub enum LowApiCode {
    Success = 0,
    KeyLookupError,
    EntityLookupError,
    BrushLookupError,
    SurfaceLookupError,
    EntityTypeError,
    NoAxesError,
}

#[cfg(not(target_family = "wasm"))]
unsafe impl wasmer::FromToNativeWasmType for LowApiCode {
    type Native = i32;

    fn from_native(native: Self::Native) -> Self {
        match native {
            x if x == Self::Success.to_native() => LowApiCode::Success,
            x if x == Self::KeyLookupError.to_native() => {
                LowApiCode::KeyLookupError
            }
            x if x == Self::EntityLookupError.to_native() => {
                LowApiCode::EntityLookupError
            }
            x if x == Self::BrushLookupError.to_native() => {
                LowApiCode::BrushLookupError
            }
            x if x == Self::SurfaceLookupError.to_native() => {
                LowApiCode::SurfaceLookupError
            }
            x if x == Self::EntityTypeError.to_native() => {
                LowApiCode::EntityTypeError
            }
            x if x == Self::NoAxesError.to_native() => LowApiCode::NoAxesError,
            _ => panic!("{} out of range for LowApiCode", native),
        }
    }

    fn to_native(self) -> Self::Native {
        self as Self::Native
    }
}
