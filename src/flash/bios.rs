use alloc::String;
use alloc::string::ToString;
use dmi;
use plain::Plain;
use uefi::status::{Error, Result};

use exec::shell;
use flash::Component;
use fs::{find, load};
use hw;

pub struct BiosComponent {
    model: String,
    version: String,
}

impl BiosComponent {
    pub fn new() -> BiosComponent {
        let mut model = String::new();
        let mut version = String::new();

        for table in hw::dmi() {
            match table.header.kind {
                0 => if let Ok(info) = dmi::BiosInfo::from_bytes(&table.data) {
                    let index = info.version;
                    if index > 0 {
                        if let Some(value) = table.strings.get((index - 1) as usize) {
                            version = value.trim().to_string();
                        }
                    }
                },
                1 => if let Ok(info) = dmi::SystemInfo::from_bytes(&table.data) {
                    let index = info.version;
                    if index > 0 {
                        if let Some(value) = table.strings.get((index - 1) as usize) {
                            model = value.trim().to_string();
                        }
                    }
                },
                _ => {}
            }
        }

        BiosComponent {
            model: model,
            version: version,
        }
    }
}

impl Component for BiosComponent {
    fn name(&self) -> &str {
        "BIOS"
    }

    fn path(&self) -> &str {
        "\\system76-firmware-update\\firmware\\firmware.rom"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn validate(&self) -> Result<bool> {
        let data = load(self.path())?;
        Ok(data.len() == 8 * 1024 * 1024)
    }

    fn flash(&self) -> Result<()> {
        find("\\system76-firmware-update\\res\\firmware.nsh")?;

        let status = shell("\\system76-firmware-update\\res\\firmware.nsh bios flash")?;
        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        Ok(())
    }
}
