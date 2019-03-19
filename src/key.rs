use uefi::status::Result;
use uefi::text::TextInputKey;

pub fn raw_key() -> Result<TextInputKey> {
    let uefi = std::system_table();

    let mut index = 0;
    (uefi.BootServices.WaitForEvent)(1, &uefi.ConsoleIn.WaitForKey, &mut index)?;

    let mut key = TextInputKey {
        ScanCode: 0,
        UnicodeChar: 0
    };

    (uefi.ConsoleIn.ReadKeyStroke)(uefi.ConsoleIn, &mut key)?;

    Ok(key)
}
