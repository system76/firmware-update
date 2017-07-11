use core::ops::Try;
use uefi::guid::NULL_GUID;
use uefi::status::{Error, Result};

use string::nstr;

pub fn main() -> Result<()> {
    let uefi = unsafe { &mut *::UEFI };

    let mut name = [0; 4096];
    let mut guid = NULL_GUID;
    loop {
        let name_ptr = name.as_mut_ptr();
        let mut name_size = name.len();

        match (uefi.RuntimeServices.GetNextVariableName)(&mut name_size, name_ptr, &mut guid).into_result() {
            Ok(_) => {
                println!("{}: {}", guid, nstr(name_ptr));
            },
            Err(err) => match err {
                Error::NotFound => break,
                _ => return Err(err)
            }
        }
    }

    Ok(())
}
