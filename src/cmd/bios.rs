use uefi::status::{Error, Result};

use fs::find;
use io::wait_key;
use shell::shell;

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let (fs, _) = find("\\res\\firmware.nsh")?;

    (uefi.ConsoleOut.ClearScreen)(uefi.ConsoleOut)?;

    let status = shell(&format!("fs{}:\\res\\firmware.nsh bios verify", fs))?;
    if status != 0 {
        println!("Failed to verify BIOS: {}", status);
        return Err(Error::DeviceError);
    }

    println!("Press enter key to flash BIOS, any other to cancel");
    let c = wait_key()?;

    if c == '\r' || c == '\n' {
        let status = shell(&format!("fs{}:\\res\\firmware.nsh bios flash", fs))?;
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
