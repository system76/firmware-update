#![no_std]
#![feature(allocator)]
#![feature(compiler_builtins_lib)]
#![feature(lang_items)]

extern crate compiler_builtins;
extern crate uefi;

use uefi::boot::MemoryType;
use uefi::status::Status;

pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

#[allocator]
mod alloc;
pub mod externs;
pub mod io;
pub mod panic;

fn main() {
    let uefi = unsafe { &mut *::UEFI };

    let pool = uefi::boot::MemoryType::EfiConventionalMemory;
    let mut ptr = 0;
    let res = (uefi.BootServices.AllocatePool)(MemoryType::EfiConventionalMemory, 4096, &mut ptr);
    println!("{}: {:X}: {:X}", pool as usize, res, ptr);

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
}

#[no_mangle]
pub extern "win64" fn _start(_image_handle: *const (), uefi: &mut uefi::system::SystemTable) -> isize {
    unsafe {
        UEFI = uefi;
    }

    main();

    0
}
