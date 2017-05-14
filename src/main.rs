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

use core::{char, mem, slice};
use orbclient::Renderer;

use display::Display;
use fs::FileSystem;
use proto::Protocol;

pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod console;
pub mod display;
pub mod externs;
pub mod fs;
pub mod io;
pub mod panic;
pub mod proto;
pub mod rt;

fn main() {
    for (i, display) in Display::all().iter().enumerate() {
        println!("Display {}: {}x{}", i, display.width(), display.height());
    }

    for (i, mut fs) in FileSystem::all().iter_mut().enumerate() {
        println!("FileSystem {}", i);
        match fs.root() {
            Ok(mut root) => {
                println!("  Opened root");
                loop {
                    let mut info = uefi::fs::FileInfo::default();
                    let buf = unsafe {
                        slice::from_raw_parts_mut(
                            &mut info as *mut _ as *mut u8,
                            mem::size_of_val(&info)
                        )
                    };
                    match root.read(buf) {
                        Ok(0) => break,
                        Ok(_len) => {
                            print!("    File '");
                            for &w in info.FileName.iter() {
                                if w == 0 {
                                    break;
                                }
                                print!("{}", char::from_u32(w as u32).unwrap_or('?'));
                            }
                            println!("'");
                        },
                        Err(err) => {
                            println!("    Failed to read: {}", err);
                            break;
                        }
                    }
                }
            },
            Err(err) => {
                println!("  Failed to open root: {}", err);
            }
        }
    }
}
