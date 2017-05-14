use alloc_uefi;
use collections::boxed::Box;
use core::fmt::Write;
use uefi;

use console::Console;
use display::Display;
use io;
use main;

#[no_mangle]
pub extern "win64" fn _start(_image_handle: *const (), uefi: &'static mut uefi::system::SystemTable) -> isize {
    unsafe {
        ::UEFI = uefi;
        io::STDOUT = Some(uefi.ConsoleOut as *mut Write);
        alloc_uefi::init(&mut *::UEFI);
    }

    for display in Display::all() {
        let console = Console::new(display);
        unsafe {
            io::STDOUT = Some(Box::into_raw(Box::new(console)) as *mut Write);
        }
        break;
    }

    main();

    loop {
        unsafe { asm!("hlt" : : : : "intel", "volatile"); }
    }
}
