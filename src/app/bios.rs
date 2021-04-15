// SPDX-License-Identifier: GPL-3.0-only

use alloc::collections::BTreeMap;
use core::char;
use coreboot_fs::Rom;
use ecflash::EcFlash;
use intel_spi::{HsfStsCtl, Spi, SpiKbl, SpiCnl};
use plain::Plain;
use std::fs::{find, load};
use std::ptr;
use std::vars::{get_boot_item, get_boot_order, set_boot_item, set_boot_order};
use uefi::reset::ResetType;
use uefi::status::{Error, Result, Status};

use super::{FIRMWARECAP, FIRMWAREDIR, FIRMWARENSH, FIRMWAREROM, H2OFFT, IFLASHV, UEFIFLASH, shell, Component};

pub struct BiosComponent {
    capsule: bool,
    bios_vendor: String,
    bios_version: String,
    system_version: String,
}

impl BiosComponent {
    pub fn new() -> BiosComponent {
        let capsule = find(FIRMWARECAP).is_ok();

        let mut bios_vendor = String::new();
        let mut bios_version = String::new();
        let mut system_version = String::new();

        for table in crate::dmi::dmi() {
            match table.header.kind {
                0 => if let Ok(info) = dmi::BiosInfo::from_bytes(&table.data) {
                    let index = info.vendor;
                    if index > 0 {
                        if let Some(value) = table.strings.get((index - 1) as usize) {
                            bios_vendor = value.trim().to_string();
                        }
                    }

                    let index = info.version;
                    if index > 0 {
                        if let Some(value) = table.strings.get((index - 1) as usize) {
                            bios_version = value.trim().to_string();
                        }
                    }
                },
                1 => if let Ok(info) = dmi::SystemInfo::from_bytes(&table.data) {
                    let index = info.version;
                    if index > 0 {
                        if let Some(value) = table.strings.get((index - 1) as usize) {
                            system_version = value.trim().to_string();
                        }
                    }
                },
                _ => {}
            }
        }

        BiosComponent {
            capsule,
            bios_vendor,
            bios_version,
            system_version,
        }
    }

    pub fn spi(&self) -> Option<(&'static mut dyn Spi, HsfStsCtl)> {
        match self.bios_vendor.as_str() {
            "coreboot" => match self.system_version.as_str() {
                "galp2" |
                "galp3" |
                "galp3-b" => {
                    let spi_kbl = unsafe {
                        &mut *(SpiKbl::address() as *mut SpiKbl)
                    };
                    let hsfsts_ctl = spi_kbl.hsfsts_ctl();
                    Some((spi_kbl as &mut dyn Spi, hsfsts_ctl))
                },
                "addw1" |
                "addw2" |
                "bonw14" |
                "darp5" |
                "darp6" |
                "darp7" | // Technically TGL-U but protocol is the same
                "galp3-c" |
                "galp4" |
                "galp5" | // Technically TGL-U but protocol is the same
                "gaze14" |
                "gaze15" |
                "lemp9" |
                "lemp10" | // Technically TGL-U but protocol is the same
                "oryp5" |
                "oryp6" |
                "oryp7" => {
                    let spi_cnl = unsafe {
                        &mut *(SpiCnl::address() as *mut SpiCnl)
                    };
                    let hsfsts_ctl = spi_cnl.hsfsts_ctl();
                    Some((spi_cnl as &mut dyn Spi, hsfsts_ctl))
                },
                _ => None,
            },
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn spi_unlock() {
        if let Ok(mut ec) = EcFlash::new(true) {
            unsafe {
                println!("GetParam(WINF)");
                let mut value = ec.get_param(0xDA).unwrap_or(0x00);
                println!("GetParam(WINF) = 0x{:>02X}", value);
                value |= 0x08;
                println!("SetParam(WINF, 0x{:>02X})", value);
                let _ = ec.set_param(0xDA, value);

                println!("SetPOnTimer(0, 2)");
                let _ = ec.cmd(0x97);
                let _ = ec.write(0x00);
                let _ = ec.write(0x02);

                println!("PowerOff");
                let _ = ec.cmd(0x95);
            }

            println!("Halt");
            loop {}
        } else {
            println!("Failed to locate EC");
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

    fn model(&self) -> &str {
        &self.system_version
    }

    fn version(&self) -> &str {
        &self.bios_version
    }

    fn validate(&self) -> Result<bool> {
        let data = load(self.path())?;
        if let Some((spi, _hsfsts_ctl)) = self.spi() {
            // if hsfsts_ctl.contains(HsfStsCtl::FDOPSS) {
            //     println!("SPI currently locked, attempting to unlock");
            //     Self::spi_unlock();
            // }

            let len = spi.len().map_err(|_| Error::DeviceError)?;
            Ok(data.len() == len)
        } else if self.capsule {
            Ok(true)
        } else {
            Ok(
                data.len() == 8 * 1024 * 1024 ||
                data.len() == 16 * 1024 * 1024 ||
                data.len() == 32 * 1024 * 1024 ||
                //TODO: rename firmware.rom to firmware.cap in these cases
                find(H2OFFT).is_ok() || // H2OFFT capsule support
                find(IFLASHV).is_ok() || // meer5 capsule support
                find(UEFIFLASH).is_ok() // meer4 capsule support
            )
        }
    }

    fn flash(&self) -> Result<()> {
        if let Some((spi, _hsfsts_ctl)) = self.spi() {
            // Read new data
            let mut new;
            {
                let loading = "Loading";
                print!("SPI FILE: {}", loading);
                // TODO: Do not require two load operations
                new = load(self.path())?;
                for _c in loading.chars() {
                    print!("\x08");
                }
                println!("{} MB", new.len() / (1024 * 1024));
            }

            // Grab new FMAP areas area, if they exist
            let mut new_areas = BTreeMap::new();
            {
                let rom = Rom::new(&new);
                if let Some(fmap) = rom.fmap() {
                    let mut name = String::new();
                    for &b in fmap.name.iter() {
                        if b == 0 {
                            break;
                        }
                        name.push(b as char);
                    }

                    println!("  {}", name);

                    for i in 0..fmap.nareas {
                        let area = fmap.area(i);

                        let mut name = String::new();
                        for &b in area.name.iter() {
                            if b == 0 {
                                break;
                            }
                            name.push(b as char);
                        }

                        println!("    {}: {}", i, name);

                        new_areas.insert(name, *area);
                    }
                }
            }

            // Check ROM size
            let len = spi.len().map_err(|_| Error::DeviceError)?;
            println!("SPI ROM: {} MB", len / (1024 * 1024));
            if len != new.len() {
                println!("firmware.rom size invalid");
                return Err(Error::DeviceError);
            }

            // Read current data
            let mut data;
            {
                data = Vec::with_capacity(len);
                let mut print_mb = !0; // Invalid number to force first print
                while data.len() < len {
                    let mut buf = [0; 4096];
                    let read = spi.read(data.len(), &mut buf).map_err(|_| Error::DeviceError)?;
                    data.extend_from_slice(&buf[..read]);

                    // Print output once per megabyte
                    let mb = data.len() / (1024 * 1024);
                    if mb != print_mb {
                        print!("\rSPI READ: {} MB", mb);
                        print_mb = mb;
                    }
                }
                println!();
            }

            // Grab old FMAP areas, if they exist
            let mut areas = BTreeMap::new();
            {
                let rom = Rom::new(&data);
                if let Some(fmap) = rom.fmap() {
                    let mut name = String::new();
                    for &b in fmap.name.iter() {
                        if b == 0 {
                            break;
                        }
                        name.push(b as char);
                    }

                    println!("  {}", name);

                    for i in 0..fmap.nareas {
                        let area = fmap.area(i);

                        let mut name = String::new();
                        for &b in area.name.iter() {
                            if b == 0 {
                                break;
                            }
                            name.push(b as char);
                        }

                        println!("    {}: {}", i, name);

                        areas.insert(name, *area);
                    }
                }
            }

            // Copy old areas to new areas
            let area_names = [
                "SMMSTORE".to_string(),
            ];
            for area_name in &area_names {
                if let Some(new_area) = new_areas.get(area_name) {
                    let new_offset = new_area.offset as usize;
                    let new_size = new_area.size as usize;
                    println!(
                        "{}: found in new firmware: offset {:#X}, size {} KB",
                        area_name,
                        new_offset,
                        new_size / 1024
                    );
                    let new_slice = new.get_mut(
                        new_offset .. new_offset + new_size
                    ).ok_or(Error::DeviceError)?;

                    if let Some(area) = areas.get(area_name) {
                        let offset = area.offset as usize;
                        let size = area.size as usize;
                        println!(
                            "{}: found in old firmware: offset {:#X}, size {} KB",
                            area_name,
                            new_offset,
                            new_size / 1024
                        );
                        let slice = data.get(
                            offset .. offset + size
                        ).ok_or(Error::DeviceError)?;

                        if slice.len() == new_slice.len() {
                            if area_name == "SMMSTORE" {
                                println!("{}: compacting region data", area_name);
                                let compact = smmstore::compact(&slice);
                                new_slice.copy_from_slice(compact.as_slice());
                            } else {
                                new_slice.copy_from_slice(slice);
                            };

                            println!(
                                "{}: copied from old firmware to new firmware",
                                area_name
                            );
                        } else {
                            println!(
                                "{}: old firmware size {} does not match new firmware size {}, not copying",
                                area_name,
                                slice.len(),
                                new_slice.len()
                            );
                        }
                    } else {
                        println!(
                            "{}: found in new firmware, but not found in old firmware",
                            area_name
                        );
                    }
                } else if areas.get(area_name).is_some() {
                    println!(
                        "{}: found in old firmware, but not found in new firmware",
                        area_name
                    );
                }
            }

            // Erase and write
            {
                let erase_byte = 0xFF;
                let erase_size = 4096;
                let mut i = 0;
                let mut print_mb = !0; // Invalid number to force first print
                for (chunk, new_chunk) in data.chunks(erase_size).zip(new.chunks(erase_size)) {
                    // Data matches, meaning sector can be skipped
                    let mut matching = true;
                    // Data is erased, meaning sector can be erased instead of written
                    let mut erased = true;
                    for (&byte, &new_byte) in chunk.iter().zip(new_chunk.iter()) {
                        if new_byte != byte {
                            matching = false;
                        }
                        if new_byte != erase_byte {
                            erased = false;
                        }
                    }

                    if ! matching {
                        spi.erase(i).unwrap();
                        if ! erased {
                            spi.write(i, &new_chunk).unwrap();
                        }
                    }

                    i += chunk.len();

                    // Print output once per megabyte
                    let mb = i / (1024 * 1024);
                    if mb != print_mb {
                        print!("\rSPI WRITE: {} MB", mb);
                        print_mb = mb;
                    }
                }
                println!();
            }

            // Verify
            {
                data.clear();
                let mut print_mb = !0; // Invalid number to force first print
                while data.len() < len {
                    let mut address = data.len();

                    let mut buf = [0; 4096];
                    let read = spi.read(address, &mut buf).unwrap();
                    data.extend_from_slice(&buf[..read]);

                    while address < data.len() {
                        if data[address] != new[address] {
                            println!(
                                "\nverification failed as {:#x}: {:#x} != {:#x}",
                                address,
                                data[address],
                                new[address]
                            );
                            return Err(Error::DeviceError);
                        }
                        address += 1;
                    }

                    let mb = data.len() / (1024 * 1024);
                    if mb != print_mb {
                        print!("\rSPI VERIFY: {} MB", mb);
                        print_mb = mb;
                    }
                }
                println!();
            }
        } else {
            find(FIRMWARENSH)?;

            let mut boot_options: Vec<(u16, Vec<u8>)> = vec!();

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

            #[allow(clippy::single_match)]
            match self.system_version.as_str() {
                "thelio-b2" => {
                    // thelio-b2 sometimes has issues with keyboard input after flashing,
                    // so we will shut down after a short delay

                    println!("System will shut off in 5 seconds");
                    let _ = (std::system_table().BootServices.Stall)(5_000_000);

                    (std::system_table().RuntimeServices.ResetSystem)(ResetType::Shutdown, Status(0), 0, ptr::null());
                },
                _ => (),
            }

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
                return Err(Error::DeviceError);
            }
        }

        Ok(())
    }
}
