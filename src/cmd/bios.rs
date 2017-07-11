use uefi::status::{Error, Result};

use exec::shell;
use io::wait_key;

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    (uefi.ConsoleOut.ClearScreen)(uefi.ConsoleOut)?;

    let status = shell("\\res\\firmware.nsh bios verify")?;
    if status != 0 {
        println!("Failed to verify BIOS: {}", status);
        return Err(Error::DeviceError);
    }

    println!("Press enter key to flash BIOS, any other to cancel");
    let c = wait_key()?;

    if c == '\r' || c == '\n' {
        let status = shell("\\res\\firmware.nsh bios flash")?;
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
