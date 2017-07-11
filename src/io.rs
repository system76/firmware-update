use core::char;
use core::fmt::{self, Write};
use uefi::status;
use uefi::text::TextInputKey;

pub struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, string: &str) -> Result<(), fmt::Error> {
        let uefi = unsafe { &mut *::UEFI };

        for c in string.chars() {
            let _ = (uefi.ConsoleOut.OutputString)(uefi.ConsoleOut, [c as u16, 0].as_ptr());
            if c == '\n' {
                let _ = (uefi.ConsoleOut.OutputString)(uefi.ConsoleOut, ['\r' as u16, 0].as_ptr());
            }
        }

        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

pub fn wait_key() -> Result<char, status::Error> {
    let uefi = unsafe { &mut *::UEFI };

    let mut index = 0;
    (uefi.BootServices.WaitForEvent)(1, &uefi.ConsoleIn.WaitForKey, &mut index)?;

    let mut input = TextInputKey {
        ScanCode: 0,
        UnicodeChar: 0
    };

    (uefi.ConsoleIn.ReadKeyStroke)(uefi.ConsoleIn, &mut input)?;

    Ok(unsafe {
        char::from_u32_unchecked(input.UnicodeChar as u32)
    })
}
