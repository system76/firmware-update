#![no_std]
#![feature(asm)]
#![feature(collections)]
#![feature(compiler_builtins_lib)]
#![feature(const_fn)]
#![feature(lang_items)]

extern crate alloc_uefi;
#[macro_use]
extern crate collections;
extern crate compiler_builtins;
extern crate orbclient;
extern crate uefi;

use orbclient::{Color, Renderer};

use display::Display;

pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod externs;

pub mod display;
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

    let mut displays = Display::all();
    println!("Displays: {}", displays.len());
    for display in displays.iter_mut() {
        display.set(Color::rgb(0x5F, 0xAF, 0xFF));

        let mut x = 0;
        let mut y = 0;
        for c in "This is a test\nof drawing".chars() {
            if c == '\n' {
                y += 16;
                x = 0;
            } else {
                display.char(x, y, c, Color::rgb(255, 255, 255));
                x += 8;
            }
        }
    }

    loop {}
}
