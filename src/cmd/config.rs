use uefi::status::Result;

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    for table in uefi.config_tables().iter() {
        println!("{}: {:?}", table.VendorGuid, table.VendorGuid.kind());
    }

    Ok(())
}
