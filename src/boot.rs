#![no_std]
#![feature(intrinsics)]
#![feature(asm)]

use uefi::SimpleTextOutput;

#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(missing_copy_implementations)]
pub mod uefi;

// #[no_stack_check]
pub fn efi_main(sys : uefi::SystemTable) {
    sys.console().write("Hello, World!\n\r");

    loop {}
}

#[no_mangle]
pub fn abort() -> ! {
	loop {}
}

#[no_mangle]
pub fn breakpoint() -> ! {
	loop {}
}
