use dmi;
use plain::Plain;
use std::fs::{find, load};
use uefi::status::{Error, Result};

use super::{FIRMWAREDIR, FIRMWARENSH, FIRMWAREROM, shell, Component};

pub struct BiosComponent {
    model: String,
    version: String,
}

impl BiosComponent {
    pub fn new() -> BiosComponent {
        let mut model = String::new();
        let mut version = String::new();

        for table in crate::dmi::dmi() {
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
        FIRMWAREROM
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn validate(&self) -> Result<bool> {
        let data = load(self.path())?;
        Ok(
            data.len() == 8 * 1024 * 1024 ||
            data.len() == 16 * 1024 * 1024 ||
            data.len() == 32 * 1024 * 1024
        )
    }

    fn flash(&self) -> Result<()> {
        find(FIRMWARENSH)?;

        let cmd = format!("{} {} bios flash", FIRMWARENSH, FIRMWAREDIR);

        let status = shell(&cmd)?;
        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Error::DeviceError);
        }

        Ok(())
    }
}
