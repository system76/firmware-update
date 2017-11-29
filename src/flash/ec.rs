use alloc::{String, Vec};
use ecflash::{Ec, EcFile, EcFlash};
use uefi::status::{Error, Result};

use exec::shell;
use flash::Component;
use fs::{find, load};

pub struct EcComponent {
    master: bool,
    model: String,
    version: String,
}

impl EcComponent {
    pub fn new(master: bool) -> EcComponent {
        let mut model = String::new();
        let mut version = String::new();

        if let Ok(mut ec) = EcFlash::new(master) {
            model = ec.project();
            version = ec.version();
        }


        EcComponent {
            master: master,
            model: model,
            version: version,
        }
    }

    pub fn validate_data(&self, data: Vec<u8>) -> bool {
        match EcFlash::new(self.master).map(|mut ec| ec.project()) {
            Ok(project) => {
                if EcFile::new(data).project() == project {
                    true
                } else {
                    false
                }
            },
            Err(_err) => {
                false
            }
        }
    }
}

impl Component for EcComponent {
    fn name(&self) -> &str {
        if self.master {
            "EC"
        } else {
            "EC2"
        }
    }

    fn path(&self) -> &str {
        if self.master {
            "\\system76-firmware-update\\firmware\\ec.rom"
        } else {
            "\\system76-firmware-update\\firmware\\ec2.rom"
        }
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn validate(&self) -> Result<bool> {
        let data = load(self.path())?;
        Ok(self.validate_data(data))
    }

    fn flash(&self) -> Result<()> {
        find("\\system76-firmware-update\\res\\firmware.nsh")?;

        let cmd = if self.master {
            "\\system76-firmware-update\\res\\firmware.nsh ec flash"
        } else {
            "\\system76-firmware-update\\res\\firmware.nsh ec2 flash"
        };

        let (e_p, _e_v) = match EcFlash::new(self.master) {
            Ok(mut ec) => {
                (ec.project(), ec.version())
            },
            Err(err) => {
                println!("{} Open Error: {}", self.name(), err);
                return Err(Error::NotFound);
            }
        };

        let (f_p, _f_v) = {
            let mut file = EcFile::new(load(self.path())?);
            (file.project(), file.version())
        };

        if e_p != f_p {
            println!("{} Project Mismatch", self.name());
            return Err(Error::DeviceError);
        }

        // We could check e_v vs f_v to verify version, and not flash if up to date
        // Instead, we rely on the Linux side to determine when it is appropriate to flash
        let status = shell(cmd)?;
        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        Ok(())
    }
}
