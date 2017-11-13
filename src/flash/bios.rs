use uefi::status::{Error, Result};

use exec::shell;
use flash::{Component, EcComponent};
use fs::{find, load};

pub struct BiosComponent {
    ec: EcComponent
}

impl BiosComponent {
    pub fn new() -> BiosComponent {
        BiosComponent {
            ec: EcComponent::new(true)
        }
    }
}

impl Component for BiosComponent {
    fn name(&self) -> &str {
        "BIOS"
    }
    
    fn path(&self) -> &str {
        "\\system76-firmware-update\\firmware\\bios.rom"
    }
    
    fn validate(&self) -> Result<bool> {
        let data = load(self.path())?;
        Ok(self.ec.validate_data(data))
    }
    
    fn flash(&self) -> Result<()> {
        find("\\system76-firmware-update\\res\\firmware.nsh")?;

        let status = shell("\\system76-firmware-update\\res\\firmware.nsh bios verify")?;
        if status != 0 {
            println!("{} Verify Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        let status = shell("\\system76-firmware-update\\res\\firmware.nsh bios flash")?;
        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        Ok(())
    }
}