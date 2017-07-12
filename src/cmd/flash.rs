use core::fmt::Write;
use ecflash::{Ec, EcFile, EcFlash};
use orbclient::{Color, Renderer};
use uefi::status::{Error, Result};

use console::Console;
use display::{Display, Output};
use fs::load;
use image::{self, Image};
use io::wait_key;
use proto::Protocol;

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
        let y = display.height() as i32 - 64;
        for c in prompt.chars() {
            display.char(x, y, c, Color::rgb(0xff, 0xff, 0xff));
            x += 8;
        }
    }

    {
        let prompt = "Dot not disconnect your power adapter";
        let mut x = (display.width() as i32 - prompt.len() as i32 * 8)/2;
        let y = display.height() as i32 - 32;
        for c in prompt.chars() {
            display.char(x, y, c, Color::rgb(0xff, 0, 0));
            x += 8;
        }
    }

    display.sync();

    let mut console = Console::new(display);
    console.bg = bg;

    console
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
        let mut console = console(&mut display, &splash);

        let _ = write!(console, "Loading Splash...");
        if let Ok(data) = load("\\system76-fu\\res\\splash.bmp") {
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

        match EcFlash::new(true).map(|mut ec| ec.project()) {
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
                        Ok(data) => if EcFile::new(data).project() == sys_project {
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

                let has_bios = validate(&mut console, "BIOS Update", "\\system76-fu\\res\\firmware\\bios.rom");
                let has_ec = validate(&mut console, "EC Update", "\\system76-fu\\res\\firmware\\ec.rom");

                if has_bios == ValidateKind::Found || has_ec == ValidateKind::Found {
                    let _ = writeln!(console, "Press enter to commence flashing");
                    let c = wait_key()?;
                    if c == '\n' || c == '\r' {
                        if has_bios == ValidateKind::Found {
                            let _ = write!(console, "Flashing BIOS");
                            let res = super::bios::main();
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
                            let res = super::ec::main();
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

    display.set(Color::rgb(0, 0, 0));
    display.sync();

    Ok(())
}
