use core::fmt::{Arguments, Write};

pub fn _print(args: Arguments) {
    let uefi = unsafe { &mut *::UEFI };
    let _ = uefi.ConsoleOut.write_fmt(args);
}
