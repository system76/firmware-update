use core::ptr;
use uefi::guid::GLOBAL_VARIABLE_GUID;
use uefi::status::{Error, Result};

use string::{nstr, wstr};

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let boot_current = {
        let name = wstr("BootCurrent");
        let mut data = [0; 2];
        let mut data_size = data.len();
        (uefi.RuntimeServices.GetVariable)(name.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;
        if data_size != 2 {
            return Err(Error::LoadError);
        }
        (data[0] as u16) | ((data[1] as u16) << 8)
    };

    println!("BootCurrent: {:>04X}", boot_current);

    let boot_order = {
        let name = wstr("BootOrder");
        let mut data = [0; 4096];
        let mut data_size = data.len();
        (uefi.RuntimeServices.GetVariable)(name.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;

        let mut order = vec![];
        for chunk in data[..data_size].chunks(2) {
            if chunk.len() == 2 {
                order.push((chunk[0] as u16) | (chunk[1] as u16) << 8);
            }
        }
        order
    };

    print!("BootOrder: ");
    for i in 0..boot_order.len() {
        if i > 0 {
            print!(",");
        }
        print!("{:>04X}", boot_order[i]);
    }
    println!("");

    for &num in boot_order.iter() {
        let name = format!("Boot{:>04X}", num);

        let (attributes, description) = {
            let name = wstr(&name);
            let mut data = [0; 4096];
            let mut data_size = data.len();
            (uefi.RuntimeServices.GetVariable)(name.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;
            if data_size < 6 {
                return Err(Error::LoadError);
            }

            let attributes =
                (data[0] as u32) |
                (data[1] as u32) << 8 |
                (data[2] as u32) << 16 |
                (data[3] as u32) << 24;

            let description = nstr(data[6..].as_ptr() as *const u16);

            (attributes, description)
        };

        println!("{}: {:>08X}: {}", name, attributes, description);
    }

    Ok(())
}
