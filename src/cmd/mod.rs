use uefi::status::Result;

use io::wait_key;

pub mod boot;
pub mod config;
pub mod dmi;
pub mod flash;
pub mod mouse;
pub mod vars;

pub fn menu() -> Result<()> {
    loop {
        print!("1 => flash");
        print!(", 2 => boot");
        print!(", 3 => config");
        print!(", 4 => dmi");
        print!(", 5 => mouse");
        print!(", 6 => vars");
        println!(", 0 => exit");


        let c = wait_key().unwrap_or('?');

        println!("{}", c);

        let res = match c {
            '1' => self::flash::main(),
            '2' => self::boot::main(),
            '3' => self::config::main(),
            '4' => self::dmi::main(),
            '5' => self::mouse::main(),
            '6' => self::vars::main(),
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
