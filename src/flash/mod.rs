use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr;
use orbclient::{Color, Renderer};
use uefi::reset::ResetType;
use uefi::status::{Error, Result, Status};

use display::{Display, Output};
use fs::{find, load};
use hw::EcMem;
use image::{self, Image};
use io::wait_key;
use proto::Protocol;
use text::TextDisplay;
use vars::{get_boot_current, get_boot_next, set_boot_next, get_boot_item, set_boot_item};

pub use self::bios::BiosComponent;
pub use self::component::Component;
pub use self::ec::EcComponent;

mod bios;
mod component;
mod ec;

fn ac_connected() -> bool {
    unsafe { EcMem::new().adp() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValidateKind {
    Found,
    Mismatch,
    NotFound,
    Error(Error)
}

fn components_validations() -> (Vec<Box<Component>>, Vec<ValidateKind>) {
    let components: Vec<Box<Component>> = vec![
        Box::new(BiosComponent::new()),
        Box::new(EcComponent::new(true)),
        Box::new(EcComponent::new(false)),
    ];

    let validations: Vec<ValidateKind> = components.iter().map(|component| {
        let loading = "Loading";

        print!("{}: {}", component.name(), loading);

        let ret =  match component.validate() {
            Ok(valid) => if valid {
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

        for _c in loading.chars() {
            print!("\x08");
        }

        if ret == ValidateKind::NotFound {
            print!("\x08\x08");
            for _c in component.name().chars() {
                print!("\x08");
            }
        } else {
            println!("{:?}", ret);

            let current_version = component.version();
            if ! current_version.is_empty() {
                println!("{}: Currently {}", component.name(), current_version);
            }
        }

        ret
    }).collect();

    (components, validations)
}

fn inner() -> Result<()> {
    let mut shutdown = false;

    let option = get_boot_current()?;
    println!("Booting from item {:>04X}", option);

    set_boot_next(Some(option))?;
    println!("Set boot override to {:>04X}", option);

    let (components, validations) = components_validations();

    if validations.iter().any(|v| *v != ValidateKind::Found && *v != ValidateKind::NotFound) {
        println!("! Errors were found !");
    } else if ! validations.iter().any(|v| *v == ValidateKind::Found) {
        println!("* No updates were found *");
    } else {
        // Skip enter if in manufacturing mode
        let c = if find("\\system76-firmware-update\\firmware\\meset.tag").is_ok() {
            '\n'
        } else {
            println!("Press enter to commence flashing, the system may reboot...");
            wait_key()?
        };

        if c == '\n' || c == '\r' {
            shutdown = true;

            let mut success = true;

            for (component, validation) in components.iter().zip(validations.iter()) {
                if *validation == ValidateKind::Found {
                    match component.flash() {
                        Ok(()) => {
                            println!("{}: Success", component.name());
                        },
                        Err(err) => {
                            println!("{}: Failure: {:?}", component.name(), err);
                            success = false;
                            break;
                        }
                    }
                }
            }


            if success {
                println!("* All updates applied successfully *");
            } else {
                println!("! Failed to apply updates !");
            }
        } else {
            println!("! Not applying updates !");
        }
    }

    if let Ok(next) = get_boot_next() {
        println!("Found boot override {:>04X}", next);

        set_boot_next(None)?;
        println!("Removed boot override");
    } else {
        println!("Already removed boot override");
    }

    if get_boot_item(option).is_ok() {
        println!("Found boot option {:>04X}", option);

        set_boot_item(option, &[])?;
        println!("Removed boot option {:>04X}", option);
    } else {
        println!("Already removed boot option {:>04X}", option);
    }

    if shutdown {
        println!("Press any key to shutdown...");
        wait_key()?;

        unsafe {
            ((&mut *::UEFI).RuntimeServices.ResetSystem)(ResetType::Shutdown, Status(0), 0, ptr::null());
        }
    } else {
        println!("Press any key to restart...");
        wait_key()?;
    }

    Ok(())
}

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let mut display = {
        let output = Output::one()?;

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

        let _ = (output.0.SetMode)(output.0, max_i);

        Display::new(output)
    };

    let mut splash = Image::new(0, 0);
    {
        println!("Loading Splash...");
        if let Ok(data) = load("\\system76-firmware-update\\res\\splash.bmp") {
            if let Ok(image) = image::bmp::parse(&data) {
                splash = image;
            }
        }
        println!(" Done");
    }

    {
        let bg = Color::rgb(0x41, 0x3e, 0x3c);

        display.set(bg);

        {
            let x = (display.width() as i32 - splash.width() as i32)/2;
            let y = 16;
            splash.draw(&mut display, x, y);
        }

        {
            let prompt = concat!("Firmware Updater ", env!("CARGO_PKG_VERSION"));
            let mut x = (display.width() as i32 - prompt.len() as i32 * 8)/2;
            let y = display.height() as i32 - 64;
            for c in prompt.chars() {
                display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
                x += 8;
            }
        }

        {
            let prompt = "Do not disconnect your power adapter";
            let mut x = (display.width() as i32 - prompt.len() as i32 * 8)/2;
            let y = display.height() as i32 - 32;
            for c in prompt.chars() {
                display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
                x += 8;
            }
        }

        display.sync();
    }

    if ! ac_connected() {
        {
            let prompt = "Connect your power adapter!";
            let mut x = (display.width() as i32 - prompt.len() as i32 * 8)/2;
            let y = (display.height() as i32 - 16)/2;
            for c in prompt.chars() {
                display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
                x += 8;
            }
        }

        display.sync();

        while ! ac_connected() {
            let _ = (uefi.BootServices.Stall)(1000);
        }
    }

    {
        let cols = 80;
        let off_x = (display.width() as i32 - cols as i32 * 8)/2;
        let off_y = 16 + splash.height() as i32 + 16;
        let rows = (display.height() as i32 - 64 - off_y - 1) as usize/16;
        display.rect(off_x, off_y, cols as u32 * 8, rows as u32 * 16, Color::rgb(0, 0, 0));
        display.sync();

        let mut text = TextDisplay::new(&mut display);
        text.off_x = off_x;
        text.off_y = off_y;
        text.cols = cols;
        text.rows = rows;
        text.pipe(inner)?;
    }

    Ok(())
}
