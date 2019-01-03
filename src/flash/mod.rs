use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ops::Try;
use core::ptr;
use ecflash::{Ec, EcFlash};
use orbclient::{Color, Renderer};
use uefi::guid;
use uefi::reset::ResetType;
use uefi::status::{Error, Result, Status};

use display::{Display, Output};
use exec::exec_path;
use fs::{find, load};
use hw::EcMem;
use image::{self, Image};
use io::wait_key;
use proto::Protocol;
use string::{nstr, wstr};
use text::TextDisplay;
use vars::{
    get_boot_current,
    get_boot_next, set_boot_next,
    get_boot_item, set_boot_item,
    get_os_indications, set_os_indications,
    get_os_indications_supported};

pub use self::bios::BiosComponent;
pub use self::component::Component;
pub use self::ec::EcComponent;

mod bios;
mod component;
mod ec;

static ECROM: &'static str = concat!("\\", env!("BASEDIR"), "\\firmware\\ec.rom");
static EC2ROM: &'static str = concat!("\\", env!("BASEDIR"), "\\firmware\\ec2.rom");
static FIRMWAREDIR: &'static str = concat!("\\", env!("BASEDIR"), "\\firmware");
static FIRMWARENSH: &'static str = concat!("\\", env!("BASEDIR"), "\\res\\firmware.nsh");
static FIRMWAREROM: &'static str = concat!("\\", env!("BASEDIR"), "\\firmware\\firmware.rom");
static MESETTAG: &'static str = concat!("\\", env!("BASEDIR"), "\\firmware\\meset.tag");
static SHELLEFI: &'static str = concat!("\\", env!("BASEDIR"), "\\res\\shell.efi");
static SPLASHBMP: &'static str = concat!("\\", env!("BASEDIR"), "\\res\\splash.bmp");

fn shell(cmd: &str) -> Result<usize> {
    exec_path(
        SHELLEFI,
        &[
            "-nointerrupt",
            "-nomap",
            "-nostartup",
            "-noversion",
            cmd
        ]
    )
}

fn ac_connected() -> bool {
    if let Ok(mut ec) = EcFlash::new(true) {
        // Insyde models use a different address, derived from inspecting the ACPI tables
        let address = match ec.project().as_str() {
            "N130ZU" | "N150ZU" => 0xFF500100,
            _ => 0xFF700100,
        };
        unsafe { EcMem::new(address).adp() }
    } else {
        true
    }
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


fn reset_dmi() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let mut vars = vec![];

    let mut name = [0; 1024];
    let mut guid = guid::NULL_GUID;
    loop {
        let mut size = 1024;
        let status = (uefi.RuntimeServices.GetNextVariableName)(&mut size, name.as_mut_ptr(), &mut guid);
        if let Err(err) = status.into_result() {
            match err {
                Error::NotFound => break,
                _ => return Err(err),
            }
        }
        let name_str = nstr(name.as_mut_ptr());
        if name_str.starts_with("DmiVar") {
            vars.push((name_str, guid.clone()));
        }
    }

    for (name, guid) in vars {
        println!("{}: Deleting", name);

        let wname = wstr(&name);
        let mut attributes = 0;
        let mut data = [0; 65536];
        let mut data_size = data.len();
        (uefi.RuntimeServices.GetVariable)(wname.as_ptr(), &guid, &mut attributes, &mut data_size, data.as_mut_ptr())?;

        let empty = [];
        (uefi.RuntimeServices.SetVariable)(wname.as_ptr(), &guid, attributes, 0, empty.as_ptr())?;
    }

    Ok(())
}

fn inner() -> Result<()> {
    let mut shutdown = false;

    let option = get_boot_current()?;
    println!("Booting from item {:>04X}", option);

    set_boot_next(Some(option))?;
    println!("Set boot override to {:>04X}", option);

    if ! ac_connected() {
        println!("Connect AC adapter and press any key to reboot...");
        wait_key()?;
        return Ok(());
    }

    let (components, validations) = components_validations();

    if validations.iter().any(|v| *v != ValidateKind::Found && *v != ValidateKind::NotFound) {
        println!("! Errors were found !");
    } else if ! validations.iter().any(|v| *v == ValidateKind::Found) {
        println!("* No updates were found *");
    } else {
        // Skip enter if in manufacturing mode
        let c = if find(MESETTAG).is_ok() {
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
                if let Err(err) = reset_dmi() {
                    println!("Failed to reset DMI: {:?}", err);
                }

                let supported = get_os_indications_supported().unwrap_or(0);
                if supported & 1 == 1 {
                    println!("Booting into BIOS setup on next boot");
                    let mut indications = get_os_indications().unwrap_or(0);
                    indications |= 1;
                    set_os_indications(Some(indications))?;
                } else {
                    println!("Cannot boot into BIOS setup automatically");
                }

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
        if let Ok(data) = load(SPLASHBMP) {
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
