#![no_std]
#![feature(compiler_builtins_lib)]
#![feature(lang_items)]

extern crate compiler_builtins;

use core::fmt::Write;

#[allow(dead_code)]
#[allow(non_snake_case)]
pub mod uefi;

pub fn main(uefi: &mut uefi::system::SystemTable) {
    let mode = uefi.ConsoleOut.Mode.clone();
    let _ = writeln!(uefi.ConsoleOut, "Modes: {}", mode.MaxMode);
    for i in 0..mode.MaxMode as usize {
        let mut x = 0;
        let mut y = 0;
        (uefi.ConsoleOut.QueryMode)(uefi.ConsoleOut, i, &mut x, &mut y);
        let _ = writeln!(uefi.ConsoleOut, "  {}: {}, {}", i, x, y);
    }

    let tables = uefi.config_tables();
    let _ = writeln!(uefi.ConsoleOut, "Config tables: {}", tables.len());
    for (i, table) in tables.iter().enumerate() {
        let _ = writeln!(uefi.ConsoleOut, "  {}: {}: {:?}", i, table.VendorGuid, table.kind());
    }

    loop {}
}
