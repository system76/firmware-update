#![no_std]
#![feature(collections)]
#![feature(compiler_builtins_lib)]
#![feature(lang_items)]

extern crate alloc_uefi;
#[macro_use]
extern crate collections;
extern crate compiler_builtins;
extern crate uefi;

use uefi::boot::LocateSearchType;
use uefi::guid::EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;

pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod externs;
pub mod io;
pub mod panic;
pub mod rt;

fn main() {
    let uefi = unsafe { &mut *::UEFI };


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
    println!("Mode {}: {}x{}", max_i, max_w, max_h);

    let tables = uefi.config_tables();
    println!("Config tables: {}", tables.len());
    for (i, table) in tables.iter().enumerate() {
        println!("  {}: {}: {:?}", i, table.VendorGuid, table.VendorGuid.kind());
    }

    let guid = EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;
    let mut count = 32;
    let mut handles = [uefi::Handle(0); 32];
    (uefi.BootServices.LocateHandleBuffer)(LocateSearchType::ByProtocol, &guid, 0, &mut count, handles.as_mut_ptr());
    println!("Graphics Outputs: {}", count);
    for i in 0..count {
        if let Some(handle) = handles.get(i) {
            println!("  {}: {:?}", i, handle);
        } else {
            println!("  {}: out of buffer", i);
        }
    }

    println!("Loop");
    loop {}
}
