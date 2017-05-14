use alloc_uefi;
use collections::boxed::Box;
use core::fmt::Write;
use uefi;

use console::Console;
use display::Display;
use proto::Protocol;
use io;
use main;

#[no_mangle]
pub extern "win64" fn _start(_image_handle: *const (), uefi: &'static mut uefi::system::SystemTable) -> isize {
    unsafe {
        ::UEFI = uefi;
        io::STDOUT = Some(uefi.ConsoleOut as *mut Write);
        alloc_uefi::init(&mut *::UEFI);
    }

    if let Ok(display) = Display::one() {
        let console = Box::new(Console::new(display));
        unsafe {
            io::STDOUT = Some(Box::into_raw(console) as *mut Write);
        }
    }

    main();

    loop {
        unsafe { asm!("hlt" : : : : "intel", "volatile"); }
    }
}
