use alloc::vec::Vec;
use core::ptr;
use uefi::guid::GLOBAL_VARIABLE_GUID;
use uefi::status::{Error, Result};

use string::wstr;

fn get(name: &str, data: &mut [u8]) -> Result<usize> {
    let uefi = unsafe { &mut *::UEFI };

    let wname = wstr(name);
    let mut data_size = data.len();
    (uefi.RuntimeServices.GetVariable)(wname.as_ptr(), &GLOBAL_VARIABLE_GUID, ptr::null_mut(), &mut data_size, data.as_mut_ptr())?;
    Ok(data_size)
}

fn set(name: &str, data: &[u8]) -> Result<usize> {
    let uefi = unsafe { &mut *::UEFI };

    let wname = wstr(name);
    let access = 1 | 2 | 4;
    let data_size = data.len();
    (uefi.RuntimeServices.SetVariable)(wname.as_ptr(), &GLOBAL_VARIABLE_GUID, access, data_size, data.as_ptr())?;
    Ok(data_size)
}

pub fn get_boot_current() -> Result<u16> {
    let mut data = [0; 2];
    let count = get("BootCurrent", &mut data)?;
    if count == 2 {
        Ok((data[0] as u16) | ((data[1] as u16) << 8))
    } else {
        Err(Error::LoadError)
    }
}

pub fn get_boot_next() -> Result<u16> {
    let mut data = [0; 2];
    let count = get("BootNext", &mut data)?;
    if count == 2 {
        Ok((data[0] as u16) | ((data[1] as u16) << 8))
    } else {
        Err(Error::LoadError)
    }
}

pub fn set_boot_next(num_opt: Option<u16>) -> Result<usize> {
    if let Some(num) = num_opt {
        set("BootNext", &[
            num as u8,
            (num >> 8) as u8
        ])
    } else {
        set("BootNext", &[])
    }
}

pub fn get_boot_order() -> Result<Vec<u16>> {
    let mut data = [0; 4096];
    let count = get("BootOrder", &mut data)?;

    let mut order = vec![];
    for chunk in data[..count].chunks(2) {
        if chunk.len() == 2 {
            order.push((chunk[0] as u16) | (chunk[1] as u16) << 8);
        }
    }
    Ok(order)
}

pub fn get_boot_item(num: u16) -> Result<Vec<u8>> {
    let mut data = [0; 4096];
    let count = get(&format!("Boot{:>04X}", num), &mut data)?;
    if count < 6 {
        Err(Error::LoadError)
    } else {
        Ok(data[..count].to_vec())
    }
}

pub fn set_boot_item(num: u16, data: &[u8]) -> Result<usize> {
    set(&format!("Boot{:>04X}", num), &data)
}

pub fn get_os_indications() -> Result<u64> {
    let mut data = [0; 8];
    let count = get("OsIndications", &mut data)?;
    if count == 8 {
        Ok(
            (data[0] as u64) |
            ((data[1] as u64) << 8) |
            ((data[2] as u64) << 16) |
            ((data[3] as u64) << 24) |
            ((data[4] as u64) << 32) |
            ((data[5] as u64) << 40) |
            ((data[6] as u64) << 48) |
            ((data[7] as u64) << 56)
        )
    } else {
        Err(Error::LoadError)
    }
}

pub fn set_os_indications(indications_opt: Option<u64>) -> Result<usize> {
    if let Some(indications) = indications_opt {
        set("OsIndications", &[
            indications as u8,
            (indications >> 8) as u8,
            (indications >> 16) as u8,
            (indications >> 24) as u8,
            (indications >> 32) as u8,
            (indications >> 40) as u8,
            (indications >> 48) as u8,
            (indications >> 56) as u8
        ])
    } else {
        set("OsIndications", &[])
    }
}

pub fn get_os_indications_supported() -> Result<u64> {
    let mut data = [0; 8];
    let count = get("OsIndicationsSupported", &mut data)?;
    if count == 8 {
        Ok(
            (data[0] as u64) |
            ((data[1] as u64) << 8) |
            ((data[2] as u64) << 16) |
            ((data[3] as u64) << 24) |
            ((data[4] as u64) << 32) |
            ((data[5] as u64) << 40) |
            ((data[6] as u64) << 48) |
            ((data[7] as u64) << 56)
        )
    } else {
        Err(Error::LoadError)
    }
}
