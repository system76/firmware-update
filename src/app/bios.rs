// SPDX-License-Identifier: GPL-3.0-only

// XXX: Remove
#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::char;
use coreboot_fs::Rom;
use ecflash::EcFlash;
use intel_spi::{HsfStsCtl, Spi, SpiKbl, SpiCnl};
use plain::Plain;
use std::fs::{find, load};
use std::ptr;
use std::vars::{get_boot_item, get_boot_order, set_boot_item, set_boot_order};
use std::uefi::reset::ResetType;
use std::uefi::status::{Error, Result, Status};
use std::uefi::guid;

use super::{FIRMWARECAP, FIRMWAREDIR, FIRMWARENSH, FIRMWAREROM, H2OFFT, IFLASHV, UEFIFLASH, shell, Component};

fn copy_region(region: intelflash::RegionKind, old_data: &[u8], new_data: &mut [u8]) -> core::result::Result<bool, String> {
    let old_opt = intelflash::Rom::new(old_data)?.get_region_base_limit(region)?;
    let new_opt = intelflash::Rom::new(new_data)?.get_region_base_limit(region)?;

    if old_opt.is_none() && new_opt.is_none() {
        // Neither ROM has this region, so ignore it
        return Ok(false);
    }

    let old = match old_opt {
        Some((base, limit)) => if base < limit && limit < old_data.len() {
            &old_data[base..limit + 1]
        } else {
            return Err(format!("old region {:#X}:{:#X} is invalid", base, limit));
        },
        None => return Err(format!("missing old region")),
    };

    let new = match new_opt {
        Some((base, limit)) => if base < limit && limit < new_data.len() {
            &mut new_data[base..limit + 1]
        } else {
            return Err(format!("new region {:#X}:{:#X} is invalid", base, limit));
        },
        None => return Err(format!("missing new region")),
    };

    if old.len() != new.len() {
        return Err(format!("old region size {} does not match new region size {}", old.len(), new.len()));
    }

    new.copy_from_slice(old);
    Ok(true)
}

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
                "gaze16-3050" | // Technically TGL-H but protocol is the same
                "gaze16-3060" | // Technically TGL-H but protocol is the same
                "gaze16-3060-b" | // Technically TGL-H but protocol is the same
                "lemp9" |
                "lemp10" | // Technically TGL-U but protocol is the same
                "oryp5" |
                "oryp6" |
                "oryp7" |
                "oryp8" // Technically TGL-H but protocol is the same
                => {
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
            loop {
                unsafe { asm!("cli", "hlt", options(nomem, nostack)); }
            }
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

            // Copy GBE region, if it exists
            match copy_region(intelflash::RegionKind::Ethernet, &data, &mut new) {
                Ok(true) => println!("Ethernet: copied region from old firmware to new firmare"),
                Ok(false) => (),
                Err(err) => {
                    println!("Ethernet: failed to copy: {}", err);
                    return Err(Error::DeviceError)
                },
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
                                smmstore_migrate(slice, new_slice)?;
                            } else {
                                new_slice.copy_from_slice(slice);
                            }

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
                            spi.write(i, new_chunk).unwrap();
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

// TODO: Clean up and move to UEFI crates

// Minimum size of 4 64k blocks;
const STORE_MIN_SIZE: usize = 256 * 1024;

const FVH_REVISION: u8 = 0x02;
const FVH_SIGNATURE: u32 = 0x4856465F; // '_FVH' in LE

const FVB2_READ_DISABLED_CAP: u32   = 1 << 0;
const FVB2_READ_ENABLED_CAP: u32    = 1 << 1;
const FVB2_READ_STATUS: u32         = 1 << 2;
const FVB2_WRITE_DISABLED_CAP: u32  = 1 << 3;
const FVB2_WRITE_ENABLED_CAP: u32   = 1 << 4;
const FVB2_WRITE_STATUS: u32        = 1 << 5;
const FVB2_LOCK_CAP: u32            = 1 << 6;
const FVB2_LOCK_STATUS: u32         = 1 << 7;
// No value for 1 << 8
const FVB2_STICKY_WRITE: u32        = 1 << 9;
const FVB2_MEMORY_MAPPED: u32       = 1 << 10;
const FVB2_ERASE_POLARITY: u32      = 1 << 11;
const FVB2_READ_LOCK_CAP: u32       = 1 << 12;
const FVB2_READ_LOCK_STATUS: u32    = 1 << 13;
const FVB2_WRITE_LOCK_CAP: u32      = 1 << 14;
const FVB2_WRITE_LOCK_STATUS: u32   = 1 << 15;

struct FvbAttributes2(u32);

#[allow(non_snake_case)]
#[repr(C)]
struct FvBlockMapEntry {
    NumBlocks: u32,
    Length: u32,
}

#[allow(non_snake_case)]
#[repr(C)]
struct FirmwareVolumeHeader {
    ZeroVector: [u8; 16],
    FileSystemGuid: guid::Guid,
    FvLength: u64,
    Signature: u32,
    Attributes: FvbAttributes2,
    HeaderLength: u16,
    Checksum: u16,
    ExtHeaderOffset: u16,
    Reserved: [u8; 1],
    Revision: u8,
    // FIXME: This is a VLA in edk2
    BlockMap: [FvBlockMapEntry; 2],
}

unsafe impl Plain for FirmwareVolumeHeader {}

// Create a FV header based on some assumptions.
fn firmware_volume_header(volume: &[u8]) -> Result<FirmwareVolumeHeader> {
    if volume.len() < STORE_MIN_SIZE {
        println!("SMMSTORE region is too small! (Need: {}, Actual: {})", STORE_MIN_SIZE, volume.len());
        return Err(Error::DeviceError);
    }

    let attrs = FvbAttributes2(FVB2_READ_ENABLED_CAP
        | FVB2_READ_STATUS
        | FVB2_STICKY_WRITE
        | FVB2_MEMORY_MAPPED
        | FVB2_ERASE_POLARITY
        | FVB2_WRITE_STATUS
        | FVB2_WRITE_ENABLED_CAP
    );

    let block_map = [
        // FIXME: Hard-coded for a 256 KiB SMMSTORE
        FvBlockMapEntry { NumBlocks: 4, Length: 64 * 1024 },
        FvBlockMapEntry { NumBlocks: 0, Length: 0 },
    ];

    // XXX: FTW spare block not counted?
    const FV_SIZE: u64 = 192 * 1024;

    Ok(FirmwareVolumeHeader {
        ZeroVector: [0u8; 16],
        FileSystemGuid: guid::SYSTEM_NV_DATA_FV_GUID,
        FvLength: FV_SIZE,
        Signature: FVH_SIGNATURE,
        Attributes: attrs,
        HeaderLength: (core::mem::size_of::<FirmwareVolumeHeader>()) as u16,
        Checksum: 0xFA00,
        ExtHeaderOffset: 0,
        Reserved: [0u8; 1],
        Revision: FVH_REVISION,
        BlockMap: block_map,
    })
}

const VARIABLE_STORE_FORMATTED: u8 = 0x5A;
const VARIABLE_STORE_HEALTHY: u8 = 0xFE;

#[allow(non_snake_case)]
#[repr(C)]
struct VariableStoreHeader {
    Signature: guid::Guid,
    Size: u32,
    Format: u8,
    State: u8,
    Reserved: u16,
    Reserved1: u32,
}

unsafe impl Plain for VariableStoreHeader {}

// Create a VarStore header based on some assumptions.
fn variable_store_header() -> VariableStoreHeader {
    // XXX: Only one block is used.
    VariableStoreHeader {
        Signature: guid::AUTHENTICATED_VARIABLE_GUID,
        Size: ((64 * 1024) - core::mem::size_of::<FirmwareVolumeHeader>()) as u32,
        Format: VARIABLE_STORE_FORMATTED,
        State: VARIABLE_STORE_HEALTHY,
        Reserved: 0,
        Reserved1: 0,
    }
}

const VARIABLE_NON_VOLATILE: u32                    = 1 << 0;
const VARIABLE_BOOTSERVICE_ACCESS: u32              = 1 << 1;
const VARIABLE_RUNTIME_ACCESS: u32                  = 1 << 2;
const VARIABLE_HARDWARE_ERROR_RECORD: u32           = 1 << 3;
const VARIABLE_AUTH_WRITE_ACCESS: u32               = 1 << 4;
const VARIABLE_TIME_BASED_AUTH_WRITE_ACCESS: u32    = 1 << 5;
const VARIABLE_APPEND_WRITE: u32                    = 1 << 6;

const VARIABLE_ATTR_NV_BS: u32 = VARIABLE_NON_VOLATILE | VARIABLE_BOOTSERVICE_ACCESS;
const VARIABLE_ATTR_BS_RT: u32 = VARIABLE_BOOTSERVICE_ACCESS | VARIABLE_RUNTIME_ACCESS;
const VARIABLE_ATTR_BS_RT_AT: u32 = VARIABLE_ATTR_BS_RT | VARIABLE_TIME_BASED_AUTH_WRITE_ACCESS;
const VARIABLE_ATTR_NV_BS_RT: u32 = VARIABLE_ATTR_BS_RT | VARIABLE_NON_VOLATILE;
const VARIABLE_ATTR_NV_BS_RT_HR: u32 = VARIABLE_ATTR_NV_BS_RT | VARIABLE_HARDWARE_ERROR_RECORD;
const VARIABLE_ATTR_NV_BS_RT_AT: u32 = VARIABLE_ATTR_NV_BS_RT | VARIABLE_TIME_BASED_AUTH_WRITE_ACCESS;
const VARIABLE_ATTR_AT: u32 = VARIABLE_TIME_BASED_AUTH_WRITE_ACCESS;
const VARIABLE_ATTR_NV_BS_RT_HR_AT: u32 = VARIABLE_ATTR_NV_BS_RT_HR | VARIABLE_ATTR_AT;

const VARIABLE_DATA: u16 = 0x55AA;

const VAR_ADDED: u8 = 0x3F;

#[allow(non_snake_case)]
#[repr(C)]
struct EfiTime {
    Year: u16,
    Month: u8,
    Day: u8,
    Hour: u8,
    Minute: u8,
    Second: u8,
    Pad1: u8,
    Nanosecond: u32,
    TimeZone: u16,
    Daylight: u8,
    Pad2: u8,
}

impl Default for EfiTime {
    fn default() -> Self {
        Self {
            Year: 0,
            Month: 0,
            Day: 0,
            Hour: 0,
            Minute: 0,
            Second: 0,
            Pad1: 0,
            Nanosecond: 0,
            TimeZone: 0,
            Daylight: 0,
            Pad2: 0,
        }
    }
}

// 60 bytes, must be packed or will be padded to 64 bytes.
#[allow(non_snake_case)]
#[repr(C, packed)]
struct AuthenticatedVariableHeader {
    StartId: u16,
    State: u8,
    Reserved: u8,
    Attributes: u32,
    MonotonicCount: u64,
    TimeStamp: EfiTime,
    PubKeyIndex: u32,
    NameSize: u32,
    DataSize: u32,
    VendorGuid: guid::Guid,
}

// Not a UEFI struct
struct Variable {
    header: AuthenticatedVariableHeader,
    name: Vec<u8>,
    data: Vec<u8>,
}

// XXX: Implement TryFrom for Guid?
fn guid_from_slice(slice: &[u8]) -> guid::Guid {
    let mut last: [u8; 8] = [0; 8];
    last.copy_from_slice(&slice[8..16]);

    guid::Guid(
        slice[0] as u32 | (slice[1] as u32) << 8 | (slice[2] as u32) << 16 | (slice[3] as u32) << 24,
        slice[4] as u16 | (slice[5] as u16) << 8,
        slice[6] as u16 | (slice[7] as u16) << 8,
        last
    )
}

fn variable_from_kv(key: &[u8], data: &[u8]) -> Variable {
    // In SMMSTOREv1, the GUID is prepended to the variable name.
    let guid = guid_from_slice(key);
    let name = key[16..].to_vec();

    let header = AuthenticatedVariableHeader {
        StartId: VARIABLE_DATA,
        State: VAR_ADDED,
        Reserved: 0,
        Attributes: VARIABLE_ATTR_NV_BS,
        MonotonicCount: 0,
        TimeStamp: EfiTime::default(),
        PubKeyIndex: 0,
        NameSize: name.len() as u32,
        DataSize: data.len() as u32,
        VendorGuid: guid,
    };

    Variable {
        header,
        name,
        data: data.to_vec(),
    }
}

// Migrate SMMSTOREv1 data to FV data used for SMMSTOREv2.
fn smmstore_migrate(old: &[u8], new: &mut [u8]) -> Result<()> {

    if let Ok(old_fvh) = FirmwareVolumeHeader::from_bytes(old) {
        if old_fvh.FileSystemGuid == guid::SYSTEM_NV_DATA_FV_GUID {
            // Already formatted for SMMSTOREv2.
            new.copy_from_slice(old);
            return Ok(());
        }
    }

    println!("Migrating data to SMMSTOREv2");

    let fv_hdr = firmware_volume_header(new)?;
    let varstore_hdr = variable_store_header();

    // Install the headers
    let mut i = 0;
    unsafe {
        for b in plain::as_bytes(&fv_hdr) {
            new[i] = *b;
            i += 1;
        }

        for b in plain::as_bytes(&varstore_hdr) {
            new[i] = *b;
            i += 1;
        }
    }

    let v1_data = smmstore::deserialize(old);

    for (k, v) in v1_data {
        let var = variable_from_kv(&k, &v);

        unsafe {
            for b in plain::as_bytes(&var.header) {
                new[i] = *b;
                i += 1;
            }
            for b in var.name {
                new[i] = b;
                i += 1;
            }
            for b in var.data {
                new[i] = b;
                i += 1;
            }
        }
    }

    Ok(())
}
