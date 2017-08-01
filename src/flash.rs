use ecflash::{Ec, EcFile, EcFlash};
use orbclient::{Color, Renderer};
use uefi::status::{Error, Result};

use display::{Display, Output};
use exec::shell;
use fs::{find, load};
use image::{self, Image};
use io::wait_key;
use proto::Protocol;
use text::TextDisplay;
use vars::{get_boot_item, set_boot_item};

fn bios() -> Result<()> {
    find("\\system76-firmware-update\\res\\firmware.nsh")?;

    let status = shell("\\system76-firmware-update\\res\\firmware.nsh bios verify")?;
    if status != 0 {
        println!("BIOS Verify Error: {}", status);
        return Err(Error::DeviceError);
    }

    let status = shell("\\system76-firmware-update\\res\\firmware.nsh bios flash")?;
    if status != 0 {
        println!("BIOS Flash Error: {}", status);
        return Err(Error::DeviceError);
    }

    Ok(())
}

fn ec(master: bool) -> Result<()> {
    find("\\system76-firmware-update\\res\\firmware.nsh")?;

    let (name, path, cmd) = if master {
        (
            "EC",
            "\\system76-firmware-update\\firmware\\ec.rom",
            "\\system76-firmware-update\\res\\firmware.nsh ec flash"
        )
    } else {
        (
            "EC2",
            "\\system76-firmware-update\\firmware\\ec2.rom",
            "\\system76-firmware-update\\res\\firmware.nsh ec2 flash"
        )
    };


    let (e_p, _e_v, e_s) = match EcFlash::new(master) {
        Ok(mut ec) => {
            (ec.project(), ec.version(), ec.size())
        },
        Err(err) => {
            println!("{} Open Error: {}", name, err);
            return Err(Error::NotFound);
        }
    };

    let (f_p, _f_v, f_s) = {
        let mut file = EcFile::new(load(path)?);
        (file.project(), file.version(), file.size())
    };

    if e_p != f_p {
        println!("{} Project Mismatch", name);
        return Err(Error::DeviceError);
    }

    if e_s != f_s {
        println!("{} Size Mismatch", name);
        return Err(Error::DeviceError);
    }

    // We could check e_v vs f_v to verify version, and not flash if up to date
    // Instead, we rely on the Linux side to determine when it is appropriate to flash
    let status = shell(cmd)?;
    if status != 0 {
        println!("{} Flash Error: {}", name, status);
        return Err(Error::DeviceError);
    }

    Ok(())
}

fn inner() -> Result<()> {
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum ValidateKind {
        Found,
        Mismatch,
        NotFound,
        Error(Error)
    }

    let validate = |name: &str, path: &str, ec_master: bool| -> ValidateKind {
        let loading = "Loading";

        print!("{}: {}", name, loading);

        let res = load(path);

        for _c in loading.chars() {
            print!("\x08");
        }

        let ret = match res {
            Ok(data) => {
                match EcFlash::new(ec_master).map(|mut ec| ec.project()) {
                    Ok(sys_project) => {
                        if EcFile::new(data).project() == sys_project {
                            ValidateKind::Found
                        } else {
                            ValidateKind::Mismatch
                        }
                    },
                    Err(_err) => {
                        ValidateKind::Mismatch
                    }
                }
            },
            Err(err) => if err == Error::NotFound {
                ValidateKind::NotFound
            } else {
                ValidateKind::Error(err)
            }
        };

        if ret == ValidateKind::NotFound {
            print!("\x08\x08");
            for _c in name.chars() {
                print!("\x08");
            }
        } else {
            println!("{:?}", ret);
        }

        ret
    };

    let has_bios = validate("BIOS Update", "\\system76-firmware-update\\firmware\\bios.rom", true);
    let has_ec = validate("EC Update", "\\system76-firmware-update\\firmware\\ec.rom", true);
    let has_ec2 = validate("EC2 Update", "\\system76-firmware-update\\firmware\\ec2.rom", false);

    if has_bios == ValidateKind::Found || has_ec == ValidateKind::Found || has_ec2 == ValidateKind::Found {
        println!("Press enter to commence flashing...");
        let c = wait_key()?;
        if c == '\n' || c == '\r' {
            let mut success = true;

            if has_bios == ValidateKind::Found {
                match bios() {
                    Ok(()) => {
                        println!("BIOS Update: Success");
                    },
                    Err(err) => {
                        success = false;
                        println!("BIOS Update: Failure: {:?}", err);
                    }
                }
            }

            if has_ec == ValidateKind::Found {
                match ec(true) {
                    Ok(()) => {
                        println!("EC Update: Success");
                    },
                    Err(err) => {
                        success = false;
                        println!("EC Update: Failure: {:?}", err);
                    }
                }
            }

            if has_ec2 == ValidateKind::Found {
                match ec(false) {
                    Ok(()) => {
                        println!("EC2 Update: Success");
                    },
                    Err(err) => {
                        success = false;
                        println!("EC2 Update: Failure: {:?}", err);
                    }
                }
            }

            if success {
                let option = 0x1776;

                if get_boot_item(option).is_ok() {
                    println!("Found boot option {:>04X}", option);

                    set_boot_item(option, &[])?;
                    println!("Removed boot option {:>04X}", option);
                } else {
                    println!("Already removed boot option {:>04X}", option);
                }

                println!("* All updates applied successfully *");
            } else {
                println!("! Failed to apply updates !");
            }
        }
    } else {
        println!("* No updates found *");
    }

    println!("Press any key to restart...");
    wait_key()?;

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
            let prompt = "Firmware Updater";
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
                display.char(x, y, c, Color::rgb(0xff, 0, 0));
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
