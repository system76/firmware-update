use uefi::status::Result;

use io::wait_key;

pub mod bios;
pub mod boot;
pub mod config;
pub mod dmi;
pub mod ec;
pub mod mouse;
pub mod splash;
pub mod vars;

pub fn menu() -> Result<()> {
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
            '1' => self::bios::main(),
            '2' => self::boot::main(),
            '3' => self::config::main(),
            '4' => self::dmi::main(),
            '5' => self::ec::main(),
            '6' => self::mouse::main(),
            '7' => self::splash::main(),
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
