#![no_std]
#![feature(alloc)]
#![feature(asm)]
#![feature(compiler_builtins_lib)]
#![feature(const_fn)]
#![feature(lang_items)]
#![feature(try_trait)]

#[macro_use]
extern crate alloc;
extern crate compiler_builtins;
extern crate dmi;
extern crate ecflash;
extern crate orbclient;
extern crate plain;
extern crate uefi;
extern crate uefi_alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::{char, mem, ptr, slice};
use core::fmt::Write;
use core::ops::Try;
use ecflash::{Ec, EcFile, EcFlash};
use orbclient::{Color, Renderer};
use uefi::guid::{GuidKind, NULL_GUID, GLOBAL_VARIABLE_GUID};
use uefi::status::{Error, Result, Status};

use console::Console;
use display::{Display, Output};
use fs::File;
use loaded_image::LoadedImage;
use proto::Protocol;

pub static mut HANDLE: uefi::Handle = uefi::Handle(0);
pub static mut UEFI: *mut uefi::system::SystemTable = 0 as *mut uefi::system::SystemTable;

#[macro_use]
mod macros;

pub mod console;
pub mod display;
pub mod externs;
pub mod fs;
pub mod image;
pub mod io;
pub mod loaded_image;
pub mod panic;
pub mod pointer;
pub mod proto;
pub mod rt;
pub mod shell;

fn wstr(string: &str) -> Vec<u16> {
    let mut wstring = vec![];

    for c in string.chars() {
        wstring.push(c as u16);
    }
    wstring.push(0);

    wstring
}

fn nstr(wstring: *const u16) -> String {
    let mut string = String::new();

    let mut i = 0;
    loop {
        let w = unsafe { *wstring.offset(i) };
        i += 1;
        if w == 0 {
            break;
        }
        let c = unsafe { char::from_u32_unchecked(w as u32) };
        string.push(c);
    }

    string
}

fn find(path: &str) -> Result<(usize, File)> {
    let wpath = wstr(path);

    for (i, mut fs) in fs::FileSystem::all().iter_mut().enumerate() {
        let mut root = fs.root()?;
        match root.open(&wpath) {
            Ok(file) => {
                return Ok((i, file));
            },
            Err(err) => if err != Error::NotFound {
                return Err(err);
            }
        }
    }

    Err(Error::NotFound)
}

fn load(path: &str) -> Result<Vec<u8>> {
    let (_i, mut file) = find(path)?;

    let mut data = vec![];
    let _count = file.read_to_end(&mut data)?;

    Ok(data)
}

extern "win64" fn fake_clear(a: &uefi::text::TextOutput) -> Status {
    Status(0)
}

fn shell(cmd: &str) -> Result<usize> {
    let handle = unsafe { ::HANDLE };
    let uefi = unsafe { &mut *::UEFI };

    let args = [
        "res\\shell.efi",
        "-nointerrupt",
        "-nomap",
        "-nostartup",
        "-noversion",
        cmd
    ];

    let mut cmdline = format!("\"{}\"", args[0]);
    for arg in args.iter().skip(1) {
        cmdline.push_str(" \"");
        cmdline.push_str(arg);
        cmdline.push_str("\"");
    }

    let wcmdline = wstr(&cmdline);

    let data = load(args[0])?;

    let mut shell_handle = uefi::Handle(0);
    (uefi.BootServices.LoadImage)(false, handle, 0, data.as_ptr(), data.len(), &mut shell_handle)?;

    if let Ok(loaded_image) = LoadedImage::handle_protocol(shell_handle) {
        //loaded_image.0.SystemTable.ConsoleOut.ClearScreen = fake_clear;
        loaded_image.0.LoadOptionsSize = (wcmdline.len() as u32) * 2;
        loaded_image.0.LoadOptions = wcmdline.as_ptr();
    }

    let mut exit_size = 0;
    let mut exit_ptr = ::core::ptr::null_mut();
    let ret = (uefi.BootServices.StartImage)(shell_handle, &mut exit_size, &mut exit_ptr)?;

    Ok(ret)
}

fn wait_key() -> Result<char> {
    let uefi = unsafe { &mut *::UEFI };

    let mut index = 0;
    (uefi.BootServices.WaitForEvent)(1, &uefi.ConsoleIn.WaitForKey, &mut index)?;

    let mut input = uefi::text::TextInputKey {
        ScanCode: 0,
        UnicodeChar: 0
    };

    let _ = (uefi.ConsoleIn.ReadKeyStroke)(uefi.ConsoleIn, &mut input)?;

    Ok(unsafe {
        char::from_u32_unchecked(input.UnicodeChar as u32)
    })
}

fn bios() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let (fs, _) = find("\\res\\firmware.nsh")?;

    (uefi.ConsoleOut.ClearScreen)(uefi.ConsoleOut)?;

    let status = shell(&format!("fs{}:\\res\\firmware.nsh bios verify", fs))?;
    if status != 0 {
        println!("Failed to verify BIOS: {}", status);
        return Err(Error::DeviceError);
    }

    println!("Press enter key to flash BIOS, any other to cancel");
    let c = wait_key()?;

    if c == '\r' || c == '\n' {
        let status = shell(&format!("fs{}:\\res\\firmware.nsh bios flash", fs))?;
        if status != 0 {
            println!("Failed to flash BIOS: {}", status);
            return Err(Error::DeviceError);
        }

        println!("Flashed BIOS successfully");
    } else {
        println!("Cancelled BIOS flashing");
    }

    Ok(())
}

fn boot() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let boot_current = {
        let name = wstr("BootCurrent");
        let mut data = [0; 2];
        let mut data_size = data.len();
        (uefi.RuntimeServices.GetVariable)(name.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;
        if data_size != 2 {
            return Err(Error::LoadError);
        }
        (data[0] as u16) | ((data[1] as u16) << 8)
    };

    println!("BootCurrent: {:>04X}", boot_current);

    let boot_order = {
        let name = wstr("BootOrder");
        let mut data = [0; 4096];
        let mut data_size = data.len();
        (uefi.RuntimeServices.GetVariable)(name.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;

        let mut order = vec![];
        for chunk in data[..data_size].chunks(2) {
            if chunk.len() == 2 {
                order.push((chunk[0] as u16) | (chunk[1] as u16) << 8);
            }
        }
        order
    };

    print!("BootOrder: ");
    for i in 0..boot_order.len() {
        if i > 0 {
            print!(",");
        }
        print!("{:>04X}", boot_order[i]);
    }
    println!("");

    for &num in boot_order.iter() {
        let name = format!("Boot{:>04X}", num);

        let (attributes, description) = {
            let name = wstr(&name);
            let mut data = [0; 4096];
            let mut data_size = data.len();
            (uefi.RuntimeServices.GetVariable)(name.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;
            if data_size < 6 {
                return Err(Error::LoadError);
            }

            let attributes =
                (data[0] as u32) |
                (data[1] as u32) << 8 |
                (data[2] as u32) << 16 |
                (data[3] as u32) << 24;

            let description = nstr(data[6..].as_ptr() as *const u16);

            (attributes, description)
        };

        println!("{}: {:>08X}: {}", name, attributes, description);
    }

    Ok(())
}

fn config() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    for table in uefi.config_tables().iter() {
        println!("{}: {:?}", table.VendorGuid, table.VendorGuid.kind());
    }

    Ok(())
}

fn dmi() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    for table in uefi.config_tables().iter() {
        if table.VendorGuid.kind() == GuidKind::Smbios {
            let smbios = plain::from_bytes::<dmi::Smbios>(unsafe {
                slice::from_raw_parts(table.VendorTable as *const u8, mem::size_of::<dmi::Smbios>())
            }).unwrap();

            //TODO: Check anchors, checksums

            let tables = dmi::tables(unsafe {
                slice::from_raw_parts(smbios.table_address as *const u8, smbios.table_length as usize)
            });
            for table in tables {
                match table.header.kind {
                    0 => if let Ok(info) = plain::from_bytes::<dmi::BiosInfo>(&table.data){
                        println!("{:?}", info);

                        if let Some(string) = table.get_str(info.vendor) {
                            println!("  Vendor: {}", string);
                        }

                        if let Some(string) = table.get_str(info.version) {
                            println!("  Version: {}", string);
                        }

                        if let Some(string) = table.get_str(info.date) {
                            println!("  Date: {}", string);
                        }
                    },
                    1 => if let Ok(info) = plain::from_bytes::<dmi::SystemInfo>(&table.data) {
                        println!("{:?}", info);

                        if let Some(string) = table.get_str(info.manufacturer) {
                            println!("  Manufacturer: {}", string);
                        }

                        if let Some(string) = table.get_str(info.name) {
                            println!("  Name: {}", string);
                        }

                        if let Some(string) = table.get_str(info.version) {
                            println!("  Version: {}", string);
                        }
                    },
                    _ => ()
                }
            }
        }
    }

    Ok(())
}

fn ec() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let (fs, _) = find("\\res\\firmware.nsh")?;

    (uefi.ConsoleOut.ClearScreen)(uefi.ConsoleOut)?;

    println!("Verifying EC");

    let (e_p, e_v, e_s) = match EcFlash::new(1) {
        Ok(mut ec) => {
            (ec.project(), ec.version(), ec.size())
        },
        Err(err) => {
            println!("EC Error: {}", err);
            return Err(Error::NotFound);
        }
    };

    println!("Flash Project: {}", e_p);
    println!("Flash Version: {}", e_v);
    println!("Flash Size: {} KB", e_s/1024);

    let (f_p, f_v, f_s) = {
        let mut file = EcFile::new(load("res\\firmware\\ec.rom")?);
        (file.project(), file.version(), file.size())
    };

    println!("File Project: {}", f_p);
    println!("File Version: {}", f_v);
    println!("File Size: {} KB", f_s/1024);

    if e_p != f_p {
        println!("Project Mismatch");
        return Err(Error::DeviceError);
    }

    if e_s != f_s {
        println!("Size Mismatch");
        return Err(Error::DeviceError);
    }

    println!("Press enter key to flash EC, any other to cancel");
    let c = wait_key()?;

    if c == '\r' || c == '\n' {
        let status = shell(&format!("fs{}:\\res\\firmware.nsh ec flash", fs))?;
        if status != 0 {
            println!("Failed to flash EC: {}", status);
            return Err(Error::DeviceError);
        }

        println!("Flashed EC successfully");
    } else {
        println!("Cancelled EC flashing");
    }

    Ok(())
}

fn mouse() -> Result<()> {
    use uefi::pointer::SimplePointerState;
    use uefi::text::TextInputKey;

    let uefi = unsafe { &mut *::UEFI };

    let mut pointers = pointer::Pointer::all();

    let mut events = vec![];
    for (i, mut pointer) in pointers.iter_mut().enumerate() {
        (pointer.0.Reset)(pointer.0, false)?;

        println!("Pointer {}: {:X}, {:?}", i, pointer.0.WaitForInput.0, pointer.0.Mode);
        events.push(pointer.0.WaitForInput)
    }

    println!("Keyboard {:X}", uefi.ConsoleIn.WaitForKey.0);
    events.push(uefi.ConsoleIn.WaitForKey);

    loop {
        let mut index = 0;
        (uefi.BootServices.WaitForEvent)(events.len(), events.as_mut_ptr(), &mut index)?;

        println!("Event {:X}", index);

        if let Some(mut pointer) = pointers.get_mut(index) {
            let mut state = SimplePointerState::default();
            (pointer.0.GetState)(pointer.0, &mut state)?;

            println!("{}: {:?}", index, state);
        } else {
            let mut input = TextInputKey::default();

            let _ = (uefi.ConsoleIn.ReadKeyStroke)(uefi.ConsoleIn, &mut input);

            println!("{}", char::from_u32(input.UnicodeChar as u32).unwrap_or('?'));

            break;
        }
    }

    Ok(())
}

fn console<'a>(display: &'a mut Display, splash: &image::Image) -> Console<'a> {
    let bg = Color::rgb(0x41, 0x3e, 0x3c);

    display.set(bg);

    {
        let x = (display.width() as i32 - splash.width() as i32)/2;
        let y = (display.height() as i32 - splash.height() as i32)/2;
        splash.draw(display, x, y);
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

    let mut console = Console::new(display);
    console.bg = bg;

    console
}

fn splash() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let mut display = {
        let output = Output::one()?;

        /*
        let mut max_i = 0;
        let mut max_w = 0;
        let mut max_h = 0;

        for i in 0..output.0.Mode.MaxMode {
            let mut mode_ptr = ::core::ptr::null_mut();
            let mut mode_size = 0;
            (output.0.QueryMode)(output.0, i, &mut mode_size, &mut mode_ptr)?;

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
        */

        Display::new(output)
    };

    let mut splash = image::Image::new(0, 0);
    {
        let mut console = console(&mut display, &splash);

        let _ = write!(console, "Loading Splash...");
        if let Ok(data) = load("res\\splash.bmp") {
            if let Ok(image) = image::bmp::parse(&data) {
                splash = image;

                let x = (console.display.width() as i32 - splash.width() as i32)/2;
                let y = (console.display.height() as i32 - splash.height() as i32)/2;
                splash.draw(console.display, x, y);

                console.display.sync();
            }
        }
        let _ = writeln!(console, " Done");
    }

    {
        let mut console = console(&mut display, &splash);

        let res: ::core::result::Result<String, String> = Ok(format!("TEST")); // EcFlash::new(1).map(|mut ec| ec.project());
        match res {
            Ok(sys_project) => {
                #[derive(Clone, Copy, Debug, Eq, PartialEq)]
                enum ValidateKind {
                    Found,
                    Mismatch,
                    NotFound,
                    Error(Error)
                }

                let validate = |console: &mut Console, name: &str, path: &str| -> ValidateKind {
                    let _ = write!(console, "{}: Loading", name);

                    let res = load(path);

                    console.x = (name.len() as i32 + 2) * 8;
                    let _ = write!(console, "       ");
                    console.x = (name.len() as i32 + 2) * 8;

                    let ret = match res {
                        Ok(data) => if EcFile::new(data.clone()).project() == sys_project {
                            ValidateKind::Found
                        } else {
                            ValidateKind::Mismatch
                        },
                        Err(err) => if err == Error::NotFound {
                            ValidateKind::NotFound
                        } else {
                            ValidateKind::Error(err)
                        }
                    };

                    let _ = writeln!(console, "{:?}", ret);

                    ret
                };

                let has_bios = validate(&mut console, "BIOS Update", "res\\firmware\\bios.rom");
                let has_ec = validate(&mut console, "EC Update", "res\\firmware\\ec.rom");

                if has_bios == ValidateKind::Found || has_ec == ValidateKind::Found {
                    let _ = writeln!(console, "Press enter to commence flashing");
                    let c = wait_key()?;
                    if c == '\n' || c == '\r' {
                        if has_bios == ValidateKind::Found {
                            let _ = write!(console, "Flashing BIOS");
                            let res = bios();
                            console.display.sync();
                            match res {
                                Ok(()) => {
                                    let _ = writeln!(console, ": Success");
                                },
                                Err(err) => {
                                    let _ = writeln!(console, ": Failure: {:?}", err);
                                }
                            }
                        }

                        if has_ec == ValidateKind::Found {
                            let _ = write!(console, "Flashing EC");
                            let res = ec();
                            console.display.sync();
                            match res {
                                Ok(()) => {
                                    let _ = writeln!(console, ": Success");
                                },
                                Err(err) => {
                                    let _ = writeln!(console, ": Failure: {:?}", err);
                                }
                            }
                        }
                    }
                } else {
                    let _ = writeln!(console, "No updates found.");
                }
            },
            Err(err) => {
                let _ = writeln!(console, "System EC: Error: {}", err);
            }
        };

        let _ = writeln!(console, "Press any key to exit");
        wait_key()?;
    };

    (uefi.ConsoleOut.ClearScreen)(uefi.ConsoleOut)?;

    Ok(())
}

fn vars() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let mut name = [0; 4096];
    let mut guid = NULL_GUID;
    loop {
        let name_ptr = name.as_mut_ptr();
        let mut name_size = name.len();

        match (uefi.RuntimeServices.GetNextVariableName)(&mut name_size, name_ptr, &mut guid).into_result() {
            Ok(_) => {
                println!("{}: {}", guid, nstr(name_ptr));
            },
            Err(err) => match err {
                Error::NotFound => break,
                _ => return Err(err)
            }
        }
    }

    Ok(())
}

fn main() {
    let uefi = unsafe { &mut *::UEFI };

    let _ = (uefi.BootServices.SetWatchdogTimer)(0, 0, 0, ptr::null());

    let _ = (uefi.ConsoleOut.SetAttribute)(uefi.ConsoleOut, 0x0F);

    loop {
        print!("1 => bios");
        print!(", 2 => boot");
        print!(", 3 => config");
        print!(", 4 => dmi");
        print!(", 5 => ec");
        print!(", 6 => mouse");
        print!(", 7 => splash");
        print!(", 8 => vars");
        println!(", 0 => exit");


        let c = wait_key().unwrap_or('?');

        println!("{}", c);

        let res = match c {
            '1' => bios(),
            '2' => boot(),
            '3' => config(),
            '4' => dmi(),
            '5' => ec(),
            '6' => mouse(),
            '7' => splash(),
            '8' => vars(),
            '0' => return,
            _ => {
                println!("Invalid selection '{}'", c);
                Ok(())
            }
        };

        if let Err(err) = res {
            println!("Failed to run command: {:?}", err);
        }
    }
}
