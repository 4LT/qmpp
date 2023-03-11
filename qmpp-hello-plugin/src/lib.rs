#![no_std]

#[cfg(not(test))]
use core::panic::PanicInfo;

mod implementation;

pub use implementation::*;

#[cfg(test)]
mod tests;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
