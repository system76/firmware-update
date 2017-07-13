use ecflash::{Ec, EcFile, EcFlash};
use orbclient::{Color, Renderer};
use uefi::status::{Error, Result};

use display::{Display, Output};
use fs::load;
use image::{self, Image};
use io::wait_key;
use proto::Protocol;
use text::TextDisplay;

fn inner() -> Result<()> {
    match EcFlash::new(true).map(|mut ec| ec.project()) {
        Ok(sys_project) => {
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            enum ValidateKind {
                Found,
                Mismatch,
                NotFound,
                Error(Error)
            }

            let validate = |name: &str, path: &str| -> ValidateKind {
                let loading = "Loading";

                print!("{}: {}", name, loading);

                let res = load(path);

                for _c in loading.chars() {
                    print!("\x08");
                }

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

                println!("{:?}", ret);

                ret
            };

            let has_bios = validate("BIOS Update", "\\system76-fu\\firmware\\bios.rom");
            let has_ec = validate("EC Update", "\\system76-fu\\firmware\\ec.rom");

            if has_bios == ValidateKind::Found || has_ec == ValidateKind::Found {
                println!("Press enter to commence flashing");
                let c = wait_key()?;
                if c == '\n' || c == '\r' {
                    if has_bios == ValidateKind::Found {
                        match super::bios::main() {
                            Ok(()) => {
                                println!("Flashing BIOS: Success");
                            },
                            Err(err) => {
                                println!("Flashing BIOS: Failure: {:?}", err);
                            }
                        }
                    }

                    if has_ec == ValidateKind::Found {
                        match super::ec::main() {
                            Ok(()) => {
                                println!("Flashing EC: Success");
                            },
                            Err(err) => {
                                println!("Flashing EC: Failure: {:?}", err);
                            }
                        }
                    }
                }
            } else {
                println!("No updates found.");
            }
        },
        Err(err) => {
            println!("System EC: Error: {}", err);
        }
    };

    println!("Press any key to exit");
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
        if let Ok(data) = load("\\system76-fu\\res\\splash.bmp") {
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
            let y = 16; //(display.height() as i32 - splash.height() as i32)/2;
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
            let prompt = "Dot not disconnect your power adapter";
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
        let rows = 30;
        let off_x = (display.width() as i32 - cols as i32 * 8)/2;
        let off_y = 16 + splash.height() as i32 + 16;
        display.rect(off_x, off_y, cols as u32 * 8, rows as u32 * 16, Color::rgb(0, 0, 0));
        display.sync();

        let mut text = TextDisplay::new(&mut display);
        text.off_x = off_x;
        text.off_y = off_y;
        text.cols = cols;
        text.rows = rows;
        text.pipe(inner)?;
    }

    display.set(Color::rgb(0, 0, 0));
    display.sync();

    Ok(())
}
