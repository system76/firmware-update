#![no_std]
#![feature(compiler_builtins_lib)]
#![feature(lang_items)]

extern crate compiler_builtins;
extern crate uefi;

pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod externs;
pub mod io;
pub mod panic;

#[no_mangle]
pub extern "win64" fn _start(_image_handle: *const (), uefi: &mut uefi::system::SystemTable) -> isize {
    unsafe {
        UEFI = uefi;
    }

    let mode = uefi.ConsoleOut.Mode.clone();
    println!("Modes: {}", mode.MaxMode);
    for i in 0..mode.MaxMode {
        let mut x = 0;
        let mut y = 0;
        (uefi.ConsoleOut.QueryMode)(uefi.ConsoleOut, i as usize, &mut x, &mut y);
        println!(" {}{}: {}, {}", if i == mode.Mode { "*" } else { " " }, i, x, y);
    }

    let tables = uefi.config_tables();
    println!("Config tables: {}", tables.len());
    for (i, table) in tables.iter().enumerate() {
        println!("  {}: {}: {:?}", i, table.VendorGuid, table.kind());
    }

    println!("Loop");
    loop {}

    //0
}
