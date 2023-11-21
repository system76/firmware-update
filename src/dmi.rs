// SPDX-License-Identifier: GPL-3.0-only

use core::slice;
use std::prelude::*;
use std::uefi::guid;

pub fn dmi() -> Vec<dmi::Table> {
    for table in std::system_table().config_tables() {
        let data_opt = match table.VendorGuid {
            guid::SMBIOS_TABLE_GUID => unsafe {
                let smbios = &*(table.VendorTable as *const dmi::Smbios);
                //TODO: smbios is_valid fails on bonw14, assume UEFI is right
                Some(slice::from_raw_parts(
                    smbios.table_address as *const u8,
                    smbios.table_length as usize,
                ))
            },
            guid::SMBIOS3_TABLE_GUID => unsafe {
                let smbios = &*(table.VendorTable as *const dmi::Smbios3);
                //TODO: smbios is_valid fails on bonw14, assume UEFI is right
                Some(slice::from_raw_parts(
                    smbios.table_address as *const u8,
                    smbios.table_length as usize,
                ))
            },
            _ => None,
        };

        if let Some(data) = data_opt {
            return dmi::tables(data);
        }
    }

    vec![]
}
