use alloc_uefi;
use core::fmt::Write;
use uefi;

use io;
use main;

#[no_mangle]
pub extern "win64" fn _start(_image_handle: *const (), uefi: &'static mut uefi::system::SystemTable) -> isize {
    unsafe {
        ::UEFI = uefi;

        let mut max_i = 0;
        let mut max_w = 0;
        let mut max_h = 0;

        for i in 0..uefi.ConsoleOut.Mode.MaxMode {
            let mut w = 0;
            let mut h = 0;
            (uefi.ConsoleOut.QueryMode)(uefi.ConsoleOut, i as usize, &mut w, &mut h);

            if w >= max_w && h >= max_h {
                max_i = i;
                max_w = w;
                max_h = h;
            }
        }

        (uefi.ConsoleOut.SetMode)(uefi.ConsoleOut, max_i as usize);

        io::STDOUT = Some(uefi.ConsoleOut as *mut Write);
        alloc_uefi::init(&mut *::UEFI);
    }

    main();

    loop {
        unsafe { asm!("hlt" : : : : "intel", "volatile"); }
    }
}
