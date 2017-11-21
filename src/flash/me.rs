use uefi::status::{Error, Result};

use exec::shell;
use flash::Component;
use fs::{find, load};

pub struct MeComponent {
    clean: bool
}

impl MeComponent {
    pub fn new(clean: bool) -> MeComponent {
        MeComponent {
            clean: clean
        }
    }
}

impl Component for MeComponent {
    fn name(&self) -> &str {
        if self.clean {
            "MECLEAN"
        } else {
            "ME"
        }
    }

    fn path(&self) -> &str {
        if self.clean {
            "\\system76-firmware-update\\firmware\\me_clean.rom"
        } else {
            "\\system76-firmware-update\\firmware\\me.rom"
        }
    }

    fn model(&self) -> &str {
        ""
    }

    fn version(&self) -> &str {
        ""
    }

    fn validate(&self) -> Result<bool> {
        //TODO: Better validation
        let data = load(self.path())?;
        if data.len() == 2048 * 1024 {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn flash(&self) -> Result<()> {
        find("\\system76-firmware-update\\res\\firmware.nsh")?;

        let status = if self.clean {
            shell("\\system76-firmware-update\\res\\firmware.nsh me_clean flash")?
        } else {
            shell("\\system76-firmware-update\\res\\firmware.nsh me flash")?
        };
        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        Ok(())
    }
}
