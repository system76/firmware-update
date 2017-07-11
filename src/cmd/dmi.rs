use core::{mem, slice};
use dmi;
use plain;
use uefi::guid::GuidKind;
use uefi::status::Result;

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    for table in uefi.config_tables().iter() {
        if table.VendorGuid.kind() == GuidKind::Smbios {
            let smbios = plain::from_bytes::<dmi::Smbios>(unsafe {
                slice::from_raw_parts(table.VendorTable as *const u8, mem::size_of::<dmi::Smbios>())
            }).unwrap();

            //TODO: Check anchors, checksums

            let tables = dmi::tables(unsafe {
                slice::from_raw_parts(smbios.table_address as *const u8, smbios.table_length as usize)
            });
            for table in tables {
                match table.header.kind {
                    0 => if let Ok(info) = plain::from_bytes::<dmi::BiosInfo>(&table.data){
                        println!("{:?}", info);

                        if let Some(string) = table.get_str(info.vendor) {
                            println!("  Vendor: {}", string);
                        }

                        if let Some(string) = table.get_str(info.version) {
                            println!("  Version: {}", string);
                        }

                        if let Some(string) = table.get_str(info.date) {
                            println!("  Date: {}", string);
                        }
                    },
                    1 => if let Ok(info) = plain::from_bytes::<dmi::SystemInfo>(&table.data) {
                        println!("{:?}", info);

                        if let Some(string) = table.get_str(info.manufacturer) {
                            println!("  Manufacturer: {}", string);
                        }

                        if let Some(string) = table.get_str(info.name) {
                            println!("  Name: {}", string);
                        }

                        if let Some(string) = table.get_str(info.version) {
                            println!("  Version: {}", string);
                        }
                    },
                    _ => ()
                }
            }
        }
    }

    Ok(())
}
