use alloc_uefi;
use uefi;

#[no_mangle]
pub extern "win64" fn _start(_image_handle: *const (), uefi: &'static mut uefi::system::SystemTable) -> isize {
    unsafe {
        ::UEFI = uefi;
        alloc_uefi::init(uefi);
    }

    ::main();

    0
}
