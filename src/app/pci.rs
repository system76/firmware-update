// SPDX-License-Identifier: GPL-3.0-only

use core::{mem, slice};
use hwio::{Io, Pio};
use std::prelude::*;
use std::uefi::guid;

#[allow(dead_code)]
#[repr(packed)]
struct Rsdp {
    signature: [u8; 8], // b"RSD PTR "
    chksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
    // the following fields are only available for ACPI 2.0, and are reserved otherwise
    length: u32,
    xsdt_addr: u64,
    extended_chksum: u8,
    _rsvd: [u8; 3],
}

#[allow(dead_code)]
#[repr(packed)]
struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: u64,
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

unsafe fn rsdp_mcfg(rsdp: &Rsdp) -> Option<&'static [u8]> {
    if rsdp.signature != *b"RSD PTR " {
        return None;
    }

    if rsdp.rsdt_addr != 0 {
        let rsdt = &*(rsdp.rsdt_addr as *const SdtHeader);
        if let Some(rsdt_data_len) = (rsdt.length as usize).checked_sub(mem::size_of::<SdtHeader>())
        {
            let entries = slice::from_raw_parts(
                (rsdt as *const SdtHeader).offset(1) as *const u32,
                rsdt_data_len / mem::size_of::<u32>(),
            );
            for &entry in entries {
                let sdt = &*(entry as *const SdtHeader);
                if sdt.signature == *b"MCFG" {
                    return Some(slice::from_raw_parts(
                        sdt as *const SdtHeader as *const u8,
                        sdt.length as usize,
                    ));
                }
            }
        }
    }

    if rsdp.revision >= 2 && rsdp.xsdt_addr != 0 {
        let xsdt = &*(rsdp.xsdt_addr as *const SdtHeader);
        if let Some(rsdt_data_len) = (xsdt.length as usize).checked_sub(mem::size_of::<SdtHeader>())
        {
            let entries = slice::from_raw_parts(
                (xsdt as *const SdtHeader).offset(1) as *const u64,
                rsdt_data_len / mem::size_of::<u64>(),
            );
            for &entry in entries {
                let sdt = &*(entry as *const SdtHeader);
                if sdt.signature == *b"MCFG" {
                    return Some(slice::from_raw_parts(
                        sdt as *const SdtHeader as *const u8,
                        sdt.length as usize,
                    ));
                }
            }
        }
    }

    None
}

pub fn pci_mcfg() -> Option<&'static [u8]> {
    for table in std::system_table().config_tables() {
        match table.VendorGuid {
            guid::ACPI_TABLE_GUID | guid::ACPI_20_TABLE_GUID => unsafe {
                let rsdp = &*(table.VendorTable as *const Rsdp);
                if let Some(some) = rsdp_mcfg(rsdp) {
                    return Some(some);
                }
            },
            _ => (),
        };
    }
    None
}

pub fn pci_read(bus: u8, dev: u8, func: u8, offset: u8) -> core::result::Result<u32, String> {
    if dev > 0x1f {
        return Err(format!("pci_read dev 0x{:x} is greater than 0x1f", dev));
    }

    if func > 0x7 {
        return Err(format!("pci_read func 0x{:x} is greater than 0x7", func));
    }

    let address = 0x80000000
        | (u32::from(bus) << 16)
        | (u32::from(dev) << 11)
        | (u32::from(func) << 8)
        | u32::from(offset);
    Pio::<u32>::new(0xCF8).write(address);
    Ok(Pio::<u32>::new(0xCFC).read())
}
