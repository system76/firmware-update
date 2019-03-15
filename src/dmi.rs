use core::slice;
use uefi::guid::GuidKind;

pub fn dmi() -> Vec<dmi::Table> {
    for table in std::system_table().config_tables() {
        let data_opt = match table.VendorGuid.kind() {
            GuidKind::Smbios => unsafe {
                let smbios = &*(table.VendorTable as *const dmi::Smbios);
                if smbios.is_valid() {
                    Some(slice::from_raw_parts(
                        smbios.table_address as *const u8,
                        smbios.table_length as usize
                    ))
                } else {
                    None
                }
            },
            GuidKind::Smbios3 => unsafe {
                let smbios = &*(table.VendorTable as *const dmi::Smbios3);
                if smbios.is_valid() {
                    Some(slice::from_raw_parts(
                        smbios.table_address as *const u8,
                        smbios.table_length as usize
                    ))
                } else {
                    None
                }
            },
            _ => None
        };

        if let Some(data) = data_opt {
            return dmi::tables(data);
        }
    }

    vec![]
}
