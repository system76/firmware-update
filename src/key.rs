// SPDX-License-Identifier: GPL-3.0-only

use std::prelude::*;
use std::uefi::text::TextInputKey;

pub fn raw_key() -> Result<TextInputKey> {
    let uefi = std::system_table();

    let mut index = 0;
    Result::from((uefi.BootServices.WaitForEvent)(1, &uefi.ConsoleIn.WaitForKey, &mut index))?;

    let mut key = TextInputKey {
        ScanCode: 0,
        UnicodeChar: 0,
    };

    Result::from((uefi.ConsoleIn.ReadKeyStroke)(uefi.ConsoleIn, &mut key))?;

    Ok(key)
}
