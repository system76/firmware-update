// SPDX-License-Identifier: GPL-3.0-only

use alloc::string::String;
use plain::Plain;
use std::fs::{find, load};
use std::prelude::*;
use std::vars::{get_boot_item, get_boot_order, set_boot_item, set_boot_order};

use super::{
    shell, Component, FIRMWARECAP, FIRMWAREDIR, FIRMWARENSH, FIRMWAREROM,
};

pub struct BiosComponent {
    capsule: bool,
    bios_version: String,
}

impl BiosComponent {
    pub fn new() -> BiosComponent {
        let capsule = find(FIRMWARECAP).is_ok();

        let mut bios_version = String::new();

        for table in crate::dmi::dmi() {
            match table.header.kind {
                0 => {
                    if let Ok(info) = dmi::BiosInfo::from_bytes(&table.data) {
                        let index = info.version;
                        if index > 0 {
                            if let Some(value) = table.strings.get((index - 1) as usize) {
                                bios_version = value.trim().to_string();
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        BiosComponent {
            capsule,
            bios_version,
        }
    }
}

impl Component for BiosComponent {
    fn name(&self) -> &str {
        "BIOS"
    }

    fn path(&self) -> &str {
        if self.capsule {
            FIRMWARECAP
        } else {
            FIRMWAREROM
        }
    }

    fn version(&self) -> &str {
        &self.bios_version
    }

    fn validate(&self) -> Result<bool> {
        if self.capsule {
            Ok(true)
        } else {
            let data = load(self.path())?;

            Ok(
                data.len() == 8 * 1024 * 1024 ||
                data.len() == 16 * 1024 * 1024 ||
                data.len() == 32 * 1024 * 1024
            )
        }
    }

    fn flash(&self) -> Result<()> {
        find(FIRMWARENSH)?;

        let mut boot_options: Vec<(u16, Vec<u8>)> = vec![];

        let order = get_boot_order();
        if order.is_ok() {
            println!("Preserving boot order");
            for num in order.clone().unwrap() {
                if let Ok(item) = get_boot_item(num) {
                    boot_options.push((num, item));
                } else {
                    println!("Failed to read Boot{:>04X}", num);
                }
            }
        } else {
            println!("Failed to preserve boot order");
        }

        let cmd = format!("{} {} bios flash", FIRMWARENSH, FIRMWAREDIR);
        let status = shell(&cmd)?;

        if let Ok(order) = order {
            if set_boot_order(&order).is_ok() {
                for (num, data) in boot_options {
                    if set_boot_item(num, &data).is_err() {
                        println!("Failed to write Boot{:>04X}", num);
                    }
                }
                println!("Restored boot order");
            } else {
                println!("Failed to restore boot order");
            }
        }

        if status != 0 {
            println!("{} Flash Error: {}", self.name(), status);
            return Err(Status::DEVICE_ERROR);
        }

        Ok(())
    }
}
