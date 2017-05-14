use core::fmt::{Arguments, Write};

pub static mut STDOUT: Option<*mut Write> = None;

pub fn _print(args: Arguments) {
    let stdout = unsafe { &mut *STDOUT.unwrap() };
    let _ = stdout.write_fmt(args);
}
