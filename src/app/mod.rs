// SPDX-License-Identifier: GPL-3.0-only

use core::{char, ptr};
use orbclient::{Color, Renderer};
use std::exec::exec_path;
use std::fs::{find, load};
use std::prelude::*;
use std::proto::Protocol;
use std::uefi::reset::ResetType;
use std::vars::{
    get_boot_current, get_boot_item, get_boot_next, get_boot_order, set_boot_item, set_boot_next,
    set_boot_order,
};

use crate::display::{Display, Output, ScaledDisplay};
use crate::image::{self, Image};
use crate::key::raw_key;
use crate::text::TextDisplay;

pub use self::bios::BiosComponent;
pub use self::component::Component;

mod bios;
mod component;

static FIRMWAREDIR: &str = concat!("\\", env!("BASEDIR"), "\\firmware");
static FIRMWARENSH: &str = concat!("\\", env!("BASEDIR"), "\\res\\firmware.nsh");
static FIRMWARECAP: &str = concat!("\\", env!("BASEDIR"), "\\firmware\\firmware.cap");
static FIRMWAREROM: &str = concat!("\\", env!("BASEDIR"), "\\firmware\\firmware.rom");
static IPXEEFI: &str = concat!("\\", env!("BASEDIR"), "\\firmware\\ipxe.efi");
static SHELLEFI: &str = concat!("\\", env!("BASEDIR"), "\\res\\shell.efi");
static SPLASHBMP: &str = concat!("\\", env!("BASEDIR"), "\\res\\splash.bmp");

fn shell(cmd: &str) -> Result<usize> {
    exec_path(
        SHELLEFI,
        &["-nointerrupt", "-nomap", "-nostartup", "-noversion", cmd],
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ValidateKind {
    Found,
    Mismatch,
    NotFound,
    Error(Status),
}

fn components_validations() -> (Vec<Box<dyn Component>>, Vec<ValidateKind>) {
    let components: Vec<Box<dyn Component>> = vec![
        Box::new(BiosComponent::new()),
    ];

    let validations: Vec<ValidateKind> = components
        .iter()
        .map(|component| {
            let loading = "Loading";

            print!("{}: {}", component.name(), loading);

            let ret = match component.validate() {
                Ok(valid) => {
                    if valid {
                        ValidateKind::Found
                    } else {
                        ValidateKind::Mismatch
                    }
                }
                Err(err) => {
                    if err == Status::NOT_FOUND || err == Status::INVALID_PARAMETER {
                        ValidateKind::NotFound
                    } else {
                        ValidateKind::Error(err)
                    }
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
                if !current_version.is_empty() {
                    println!("{}: Currently {}", component.name(), current_version);
                }
            }

            ret
        })
        .collect();

    (components, validations)
}

fn set_override() -> Result<u16> {
    let option = get_boot_current()?;
    println!("Booting from item {:>04X}", option);

    set_boot_next(Some(option))?;
    println!("Set boot override to {:>04X}", option);

    Ok(option)
}

fn remove_override(option: u16) -> Result<()> {
    if let Ok(next) = get_boot_next() {
        println!("Found boot override {:>04X}", next);

        set_boot_next(None)?;
        println!("Removed boot override");
    } else {
        println!("Already removed boot override");
    }

    if let Ok(mut order) = get_boot_order() {
        println!("Found boot order {:>04X?}", order);
        order.retain(|&x| x != option);
        set_boot_order(&order)?;
        println!("Set boot order {:>04X?}", order);
    } else {
        println!("Failed to read boot order");
    }

    if get_boot_item(option).is_ok() {
        println!("Found boot option {:>04X}", option);

        set_boot_item(option, &[])?;
        println!("Removed boot option {:>04X}", option);
    } else {
        println!("Already removed boot option {:>04X}", option);
    }

    Ok(())
}

fn inner() -> Result<()> {
    let mut reboot = false;
    let mut success = false;

    let option = set_override()?;

    let (components, validations) = components_validations();

    let message = if validations
        .iter()
        .any(|v| *v != ValidateKind::Found && *v != ValidateKind::NotFound)
    {
        "! Errors were found !"
    } else if !validations.iter().any(|v| *v == ValidateKind::Found) {
        "* No updates were found *"
    } else {
        println!("Press enter to commence flashing, the system may reboot...");
        let k = raw_key()?;
        let c = unsafe { char::from_u32_unchecked(k.UnicodeChar as u32) };

        if c == '\n' || c == '\r' {
            success = true;

            for (component, validation) in components.iter().zip(validations.iter()) {
                if *validation == ValidateKind::Found {
                    // Only reboot if components are flashed
                    reboot = true;
                    match component.flash() {
                        Ok(()) => {
                            println!("{}: Success", component.name());
                        }
                        Err(err) => {
                            println!("{}: Failure: {:?}", component.name(), err);
                            success = false;
                            break;
                        }
                    }
                }
            }

            if success {
                reboot = true;
                "* All updates applied successfully *"
            } else {
                "! Failed to apply updates !"
            }
        } else {
            "! Not applying updates !"
        }
    };

    remove_override(option)?;

    println!("{}", message);

    if success && find(IPXEEFI).is_ok() {
        println!("Launching iPXE...");
        match exec_path(IPXEEFI, &[]) {
            Ok(status) => {
                println!("iPXE exited with status {}", status);
            }
            Err(err) => {
                println!("Failed to launch iPXE: {:?}", err);
            }
        }
    }

    if reboot {
        println!("System will reboot in 5 seconds");
        let _ = (std::system_table().BootServices.Stall)(5_000_000);
        (std::system_table().RuntimeServices.ResetSystem)(
            ResetType::Cold,
            Status(0),
            0,
            ptr::null(),
        );
    } else {
        println!("Press any key to restart...");
        raw_key()?;
    }

    Ok(())
}

pub fn main() -> Result<()> {
    let mut display = {
        let output = Output::one()?;

        let mut max_i = 0;
        let mut max_w = 0;
        let mut max_h = 0;

        for i in 0..output.0.Mode.MaxMode {
            let mut mode_ptr = ::core::ptr::null_mut();
            let mut mode_size = 0;
            Result::from((output.0.QueryMode)(
                output.0,
                i,
                &mut mode_size,
                &mut mode_ptr,
            ))?;

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

    let mut display = ScaledDisplay::new(&mut display);

    let mut splash = Image::new(0, 0);
    {
        println!("Loading Splash...");
        if let Ok(data) = load(SPLASHBMP) {
            if let Ok(image) = image::bmp::parse(&data) {
                splash = image;
            }
        }
        println!(" Done");
    }

    {
        let bg = Color::rgb(0x36, 0x32, 0x2f);

        display.set(bg);

        {
            let x = (display.width() as i32 - splash.width() as i32) / 2;
            let y = 16;
            splash.draw(&mut display, x, y);
        }

        {
            let prompt = concat!("Firmware Updater ", env!("CARGO_PKG_VERSION"));
            let mut x = (display.width() as i32 - prompt.len() as i32 * 8) / 2;
            let y = display.height() as i32 - 64;
            for c in prompt.chars() {
                display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
                x += 8;
            }
        }

        {
            let prompt = "Do not disconnect your power adapter";
            let mut x = (display.width() as i32 - prompt.len() as i32 * 8) / 2;
            let y = display.height() as i32 - 32;
            for c in prompt.chars() {
                display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
                x += 8;
            }
        }

        display.sync();
    }

    {
        let cols = 80;
        let off_x = (display.width() as i32 - cols as i32 * 8) / 2;
        let off_y = 16 + splash.height() as i32 + 16;
        let rows = (display.height() as i32 - 64 - off_y - 1) as usize / 16;
        display.rect(
            off_x,
            off_y,
            cols as u32 * 8,
            rows as u32 * 16,
            Color::rgb(0, 0, 0),
        );
        display.sync();

        let mut text = TextDisplay::new(display);
        text.off_x = off_x;
        text.off_y = off_y;
        text.cols = cols;
        text.rows = rows;
        text.pipe(inner)?;
    }

    Ok(())
}
