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
    let data_size = data.len();
    (uefi.RuntimeServices.SetVariable)(wname.as_ptr(), &GLOBAL_VARIABLE_GUID, 0, data_size, data.as_ptr())?;
    Ok(data_size)
}

pub fn get_boot_current() -> Result<u16> {
    let mut data = [0; 2];
    let count = get("BootCurrent", &mut data)?;
    if count != 2 {
        Err(Error::LoadError)
    } else {
        Ok((data[0] as u16) | ((data[1] as u16) << 8))
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
