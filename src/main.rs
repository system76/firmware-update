#![no_std]
#![feature(alloc)]
#![feature(asm)]
#![feature(compiler_builtins_lib)]
#![feature(const_fn)]
#![feature(lang_items)]

#[macro_use]
extern crate alloc;
extern crate alloc_uefi;
extern crate compiler_builtins;
extern crate orbclient;
extern crate uefi;

use alloc::String;
use core::char;
use orbclient::{Color, Renderer};

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
pub mod image;
pub mod io;
pub mod panic;
pub mod proto;
pub mod rt;

fn dump(path: String, mut dir: fs::Dir) {
    loop {
        match dir.read() {
            Ok(None) => {
                break;
            },
            Ok(Some(info)) => {
                let is_dir = info.Attribute & uefi::fs::FILE_DIRECTORY == uefi::fs::FILE_DIRECTORY;
                let mut hidden = info.Attribute & uefi::fs::FILE_HIDDEN == uefi::fs::FILE_HIDDEN;

                let mut file_name = path.clone();
                file_name.push('/');
                for (i, &w) in info.FileName.iter().enumerate() {
                    if w == 0 {
                        break;
                    }
                    if i == 0 && w == '.' as u16 {
                        hidden = true;
                    }
                    if let Some(c) = char::from_u32(w as u32) {
                        file_name.push(c);
                    }
                }

                if ! hidden {
                    if is_dir {
                        println!("  {}/", file_name);
                        match dir.open_dir(&info.FileName) {
                            Ok(new_dir) => {
                                dump(file_name, new_dir);
                            },
                            Err(err) => {
                                println!("  Failed to open dir: {}", err);
                            }
                        }
                    } else {
                        println!("  {}", file_name);
                    }
                }
            },
            Err(err) => {
                println!("  Failed to read: {}", err);
                break;
            }
        }
    }
}

fn main() {
    for (i, mut display) in Display::all().iter_mut().enumerate() {
        println!("Display {}: {}x{}", i, display.width(), display.height());
        for j in 0..display.0.Mode.MaxMode {
            let mut mode_ptr = ::core::ptr::null_mut();
            let mut mode_size = 0;
            (display.0.QueryMode)(display.0, j, &mut mode_size, &mut mode_ptr);
            let mode = unsafe { &mut *mode_ptr };
            println!("  {}: {:X} {}: {}, {}, {}", j, mode_ptr as usize, mode_size, mode.HorizontalResolution, mode.VerticalResolution, mode.PixelFormat as usize);
        }
    }

    /*
    for (i, mut fs) in FileSystem::all().iter_mut().enumerate() {
        println!("FileSystem {}", i);
        match fs.root() {
            Ok(root) => {
                dump(String::new(), root)
            },
            Err(err) => {
                println!("  Failed to open root: {}", err);
            }
        }
    }
    */

    /*
    if let Ok(mut display) = Display::one() {
        display.set(Color::rgb(0x41, 0x3e, 0x3c));

        if let Ok(splash) = image::bmp::parse(include_bytes!("../res/splash.bmp")) {
            let x = (display.width() as i32 - splash.width() as i32)/2;
            let y = (display.height() as i32 - splash.height() as i32)/2;
            splash.draw(&mut display, x, y);
        }

        {
            let prompt = "Firmware Updater";
            let mut x = (display.width() as i32 - prompt.len() as i32 * 8)/2;
            let y = display.height() as i32 - 32;
            for c in prompt.chars() {
                display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
                x += 8;
            }
        }
    }
    */
}
