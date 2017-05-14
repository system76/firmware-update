#![no_std]
#![feature(alloc)]
#![feature(asm)]
#![feature(collections)]
#![feature(compiler_builtins_lib)]
#![feature(const_fn)]
#![feature(lang_items)]

extern crate alloc;
extern crate alloc_uefi;
#[macro_use]
extern crate collections;
extern crate compiler_builtins;
extern crate orbclient;
extern crate uefi;

pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod console;
pub mod display;
pub mod externs;
pub mod io;
pub mod panic;
pub mod rt;

fn main() {
    println!("This is a test");
}
