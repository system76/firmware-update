use alloc::String;
use alloc::string::ToString;
use dmi;
use plain::Plain;
use uefi::status::{Error, Result};

use exec::shell;
use flash::{Component, EcComponent};
use fs::find;
use hw;

pub struct BiosComponent {
    ec: EcComponent,
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
            ec: EcComponent::new(true),
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
        "\\system76-firmware-update\\firmware\\bios.rom"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn validate(&self) -> Result<bool> {
        let (_i, mut file) = find(self.path())?;

        let mut data = vec![0; 128 * 1024];
        let count = file.read(&mut data)?;

        if count == data.len() {
            Ok(self.ec.validate_data(data))
        } else {
            Ok(false)
        }
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
