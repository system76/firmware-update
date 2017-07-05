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

use core::fmt::Write;
use orbclient::{Color, Renderer};

use console::Console;
use display::{Display, Output};
use proto::Protocol;

pub static mut HANDLE: uefi::Handle = uefi::Handle(0);
pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod console;
pub mod display;
pub mod ec;
pub mod externs;
pub mod fs;
pub mod image;
pub mod io;
pub mod panic;
pub mod proto;
pub mod rt;

fn main() {
    {
        let uefi = unsafe { &mut *::UEFI };

        println!("Wait");
        (uefi.BootServices.Stall)(1000000);

        println!("Start shell");

        let parent_handle = unsafe { ::HANDLE };
        let shell = include_bytes!("../res/shell.efi");
        let mut shell_handle = uefi::Handle(0);
        let res = (uefi.BootServices.LoadImage)(false, parent_handle, 0, shell.as_ptr(), shell.len(), &mut shell_handle);
        println!("Load image: {:X}", res);

        let arg = [
            b'T' as u16,
            b'E' as u16,
            b'S' as u16,
            b'T' as u16,
            0u16
        ];
        println!("Arg {:X}", arg.as_ptr() as usize);

        let args = [
            arg.as_ptr()
        ];
        println!("Args {:X}", args.as_ptr() as usize);

        let parameters = uefi::shell::ShellParameters {
            Argv: args.as_ptr(),
            Argc: args.len(),
            StdIn: uefi.ConsoleInHandle,
            StdOut: uefi.ConsoleOutHandle,
            StdErr: uefi.ConsoleErrorHandle,
        };
        println!("StdIn: {:X}", parameters.StdIn.0);
        println!("StdOut: {:X}", parameters.StdOut.0);
        println!("StdErr: {:X}", parameters.StdErr.0);
        println!("Parameters: {:X}", &parameters as *const _ as usize);

        // println!("Wait");
        // (uefi.BootServices.Stall)(1000000);

        // let res = (uefi.BootServices.InstallProtocolInterface)(&mut shell_handle, &uefi::guid::EFI_SHELL_PARAMETERS_GUID, uefi::boot::InterfaceType::NativeInterface, &parameters as *const _ as usize);
        // println!("Install parameters: {:X}", res);

        println!("Wait");
        (uefi.BootServices.Stall)(1000000);

        let mut exit_size = 0;
        let mut exit_ptr = ::core::ptr::null_mut();
        let res = (uefi.BootServices.StartImage)(shell_handle, &mut exit_size, &mut exit_ptr);
        println!("Start image: {:X}, {}", res, exit_size);

        println!("Wait");
        (uefi.BootServices.Stall)(1000000);

        return;
    }

    if let Ok(mut output) = Output::one() {
        let mut max_i = 0;
        let mut max_w = 0;
        let mut max_h = 0;

        for i in 0..output.0.Mode.MaxMode {
            let mut mode_ptr = ::core::ptr::null_mut();
            let mut mode_size = 0;
            (output.0.QueryMode)(output.0, i, &mut mode_size, &mut mode_ptr);

            let mode = unsafe { &mut *mode_ptr };
            let w = mode.HorizontalResolution;
            let h = mode.VerticalResolution;
            if w >= max_w && h >= max_h {
                max_i = i;
                max_w = w;
                max_h = h;
            }
        }

        (output.0.SetMode)(output.0, max_i);

        let mut display = Display::new(output);

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

        display.sync();

        /*
        {
            let mut console = Console::new(&mut display);

            console.bg = Color::rgb(0x41, 0x3e, 0x3c);

            match ec::EcFlash::new(1) {
                Some(mut ec) => {
                    let _ = writeln!(console, "EC FOUND");
                    let _ = writeln!(console, "Project: {}", ec.project());
                    let _ = writeln!(console, "Version: {}", ec.version());
                    /*
                    writeln!(console, "Size: {} KB", ec.size()/1024);
                    */
                },
                None => {
                    let _ = writeln!(console, "EC NOT FOUND");
                }
            }
        }
        */
    }
}
