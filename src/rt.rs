use core::fmt::Write;
use core::ops::Try;
use uefi;
use uefi_alloc;

use uefi::status::Result;

use io;
use main;

fn set_max_mode(output: &mut uefi::text::TextOutput) -> Result<()> {
    let mut max_i = None;
    let mut max_w = 0;
    let mut max_h = 0;

    for i in 0..output.Mode.MaxMode as usize {
        let mut w = 0;
        let mut h = 0;
        if (output.QueryMode)(output, i, &mut w, &mut h).into_result().is_ok() {
            if w >= max_w && h >= max_h {
                max_i = Some(i);
                max_w = w;
                max_h = h;
            }
        }
    }

    if let Some(i) = max_i {
        (output.SetMode)(output, i)?;
    }

    Ok(())
}

#[no_mangle]
pub extern "win64" fn _start(handle: uefi::Handle, uefi: &'static mut uefi::system::SystemTable) -> isize {
    unsafe {
        ::HANDLE = handle;
        ::UEFI = uefi;

        io::STDOUT = Some(uefi.ConsoleOut as *mut Write);

        if let Err(err) = set_max_mode(uefi.ConsoleOut).into_result() {
            println!("Failed to set max mode: {:?}", err);
        }

        uefi_alloc::init(::core::mem::transmute(&mut *::UEFI));
    }

    main();

    0
}
