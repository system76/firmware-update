#![no_std]
#![feature(compiler_builtins_lib)]
#![feature(lang_items)]

extern crate compiler_builtins;

use core::fmt::Write;

#[allow(dead_code)]
#[allow(non_snake_case)]
pub mod uefi;

pub fn main(uefi: &mut uefi::system::SystemTable) {
    let _ = write!(uefi.ConsoleOut, "Hello, World write 2!!\n\r");
    let _ = write!(uefi.ConsoleOut, "Test new Xargo!!\n\r");

    loop {}
}
