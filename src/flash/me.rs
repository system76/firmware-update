use uefi::status::{Error, Result};

use exec::shell;
use flash::Component;
use fs::find;

pub struct MeComponent;

impl MeComponent {
    pub fn new() -> MeComponent {
        MeComponent
    }
}

impl Component for MeComponent {
    fn name(&self) -> &str {
        "ME"
    }
    
    fn path(&self) -> &str {
        "\\system76-firmware-update\\firmware\\me.rom"
    }
    
    fn validate(&self) -> Result<bool> {
        //TODO
        Ok(true)
    }
    
    fn flash(&self) -> Result<()> {
        find("\\system76-firmware-update\\res\\firmware.nsh")?;

        let status = shell("\\system76-firmware-update\\res\\firmware.nsh me flash")?;
        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        Ok(())
    }
}