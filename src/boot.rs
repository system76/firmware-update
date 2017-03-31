#![no_std]
#![feature(compiler_builtins_lib)]
#![feature(lang_items)]

extern crate compiler_builtins;

use core::fmt::Write;

#[allow(dead_code)]
#[allow(non_snake_case)]
pub mod uefi;

pub fn main(uefi: &mut uefi::system::SystemTable) {
    let _ = writeln!(uefi.ConsoleOut, "Text Mode!!");

    let mode = uefi.ConsoleOut.Mode.clone();
    let _ = writeln!(uefi.ConsoleOut, "{:#?}", mode);

    for (i, table) in uefi.config_tables().iter().enumerate() {
        let _ = writeln!(uefi.ConsoleOut, "{}: {}: {:?}", i, table.VendorGuid, table.kind());
    }

    loop {}
}
