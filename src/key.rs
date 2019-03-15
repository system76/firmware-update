use core::char;
use uefi::status::Result;
use uefi::text::TextInputKey;

#[derive(Debug)]
pub enum Key {
    Backspace,
    Tab,
    Enter,
    Character(char),
    Up,
    Down,
    Right,
    Left,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Escape,
    Scancode(u16),
}

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

pub fn key() -> Result<Key> {
    let raw_key = raw_key()?;
    Ok(match raw_key.ScanCode {
        0 => match unsafe { char::from_u32_unchecked(raw_key.UnicodeChar as u32) } {
            '\u{8}' => Key::Backspace,
            '\t' => Key::Tab,
            '\r' => Key::Enter,
            c => Key::Character(c),
        },
        1 => Key::Up,
        2 => Key::Down,
        3 => Key::Right,
        4 => Key::Left,
        5 => Key::Home,
        6 => Key::End,
        7 => Key::Insert,
        8 => Key::Delete,
        9 => Key::PageUp,
        10 => Key::PageDown,
        11 => Key::F1,
        12 => Key::F2,
        13 => Key::F3,
        14 => Key::F4,
        15 => Key::F5,
        16 => Key::F6,
        17 => Key::F7,
        18 => Key::F8,
        19 => Key::F9,
        20 => Key::F10,
        21 => Key::F11,
        22 => Key::F12,
        23 => Key::Escape,
        scancode => Key::Scancode(scancode),
    })
}
