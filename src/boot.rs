#![no_std]

#[allow(dead_code)]
#[allow(non_snake_case)]
pub mod uefi;

pub fn efi_main(sys: uefi::Handle<uefi::SystemTable>) {
    (&*sys.ConOut).write("Hello, World!\n\r");
}
