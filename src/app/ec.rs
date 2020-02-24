use ecflash::{Ec, EcFile, EcFlash};
use ectool::{
    Firmware,
    Timeout,
};
use std::{
    cell::Cell,
    fs::{find, load},
    str,
};
use uefi::status::{Error, Result};

use super::{ECROM, EC2ROM, FIRMWAREDIR, FIRMWARENSH, shell, Component};

pub struct UefiTimeout {
    duration: u64,
    elapsed: Cell<u64>,
}

impl UefiTimeout {
    pub fn new(duration: u64) -> Self {
        Self {
            duration,
            elapsed: Cell::new(0),
        }
    }
}

impl Timeout for UefiTimeout {
    fn reset(&mut self) {
        self.elapsed.set(0);
    }

    fn running(&self) -> bool {
        let elapsed = self.elapsed.get() + 1;
        let _ = (std::system_table().BootServices.Stall)(1);
        self.elapsed.set(elapsed);
        elapsed < self.duration
    }
}

enum EcKind {
    System76(ectool::Ec<UefiTimeout>),
    Legacy(EcFlash),
    Unknown,
}

impl EcKind {
    unsafe fn new(primary: bool) -> Self {
        if let Ok(ec) = ectool::Ec::new(primary, UefiTimeout::new(100_000)) {
            return EcKind::System76(ec);
        }

        if let Ok(ec) = EcFlash::new(primary) {
            return EcKind::Legacy(ec);
        }

        EcKind::Unknown
    }

    unsafe fn model(&mut self) -> String {
        match self {
            EcKind::System76(ec) => {
                let mut data = [0; 256];
                if let Ok(count) = ec.board(&mut data) {
                    if let Ok(string) = str::from_utf8(&data[..count]) {
                        return string.to_string();
                    }
                }
            },
            EcKind::Legacy(ec) => {
                return ec.project();
            },
            EcKind::Unknown => (),
        }
        String::new()
    }

    unsafe fn version(&mut self) -> String {
        match self {
            EcKind::System76(ec) => {
                let mut data = [0; 256];
                if let Ok(count) = ec.version(&mut data) {
                    if let Ok(string) = str::from_utf8(&data[..count]) {
                        return string.to_string();
                    }
                }
            },
            EcKind::Legacy(ec) => {
                return ec.version();
            },
            EcKind::Unknown => (),
        }
        String::new()
    }

    fn firmware_model(&self, data: Vec<u8>) -> String {
        match self {
            EcKind::System76(_) => {
                if let Some(firmware) = Firmware::new(&data) {
                    if let Ok(string) = str::from_utf8(firmware.version) {
                        return string.to_string();
                    }
                }
            },
            EcKind::Legacy(_) => {
                return EcFile::new(data).project();
            },
            EcKind::Unknown => (),
        }
        String::new()
    }
}

pub struct EcComponent {
    master: bool,
    ec: EcKind,
    model: String,
    version: String,
}

impl EcComponent {
    pub fn new(master: bool) -> EcComponent {
        unsafe {
            let mut ec = EcKind::new(master);
            let model = ec.model();
            let version = ec.version();

            EcComponent {
                ec,
                master,
                model,
                version,
            }
        }
    }

    pub fn validate_data(&self, data: Vec<u8>) -> bool {
        ! self.model.is_empty() &&
        ! self.version.is_empty() &&
        self.ec.firmware_model(data) == self.model
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
            ECROM
        } else {
            EC2ROM
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
        match &self.ec {
            EcKind::System76(_) => {
                println!("{} Failed to flash EcKind::System76", self.name());
                return Err(Error::DeviceError);
            },
            EcKind::Legacy(_) => {
                find(FIRMWARENSH)?;

                let cmd = if self.master {
                    format!("{} {} ec flash", FIRMWARENSH, FIRMWAREDIR)
                } else {
                    format!("{} {} ec2 flash", FIRMWARENSH, FIRMWAREDIR)
                };

                let status = shell(&cmd)?;
                if status != 0 {
                    println!("{} Flash Error: {}", self.name(), status);
                    return Err(Error::DeviceError);
                }

                Ok(())
            },
            EcKind::Unknown => {
                println!("{} Failed to flash EcKind::Unknown", self.name());
                return Err(Error::DeviceError);
            },
        }
    }
}
