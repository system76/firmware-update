use uefi::status::Result;

use io::wait_key;

pub mod bios;
pub mod boot;
pub mod config;
pub mod dmi;
pub mod ec;
pub mod flash;
pub mod mouse;
pub mod vars;

pub fn menu() -> Result<()> {
    loop {
        print!("1 => flash");
        print!(", 2 => bios");
        print!(", 3 => boot");
        print!(", 4 => config");
        print!(", 5 => dmi");
        print!(", 6 => ec");
        print!(", 7 => mouse");
        print!(", 8 => vars");
        println!(", 0 => exit");


        let c = wait_key().unwrap_or('?');

        println!("{}", c);

        let res = match c {
            '1' => self::flash::main(),
            '2' => self::bios::main(),
            '3' => self::boot::main(),
            '4' => self::config::main(),
            '5' => self::dmi::main(),
            '6' => self::ec::main(),
            '7' => self::mouse::main(),
            '8' => self::vars::main(),
            '0' => return Ok(()),
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
