use core::fmt;

#[repr(C)]
pub struct TextInputKey {
    ScanCode: u16,
    UnicodeChar: u16,
}

#[repr(C)]
pub struct TextInput {
    Reset: extern "win64" fn(&TextInput, bool) -> isize,
    ReadKeyStroke: extern "win64" fn(&TextInput, &mut TextInputKey) -> isize,
    WaitForKey: *const (),
}

#[repr(C)]
pub struct TextOutputMode {
    MaxMode: i32,
    Mode: i32,
    Attribute: i32,
    CursorColumn: i32,
    CursorRow: i32,
    CursorVisible: bool,
}

#[repr(C)]
pub struct TextOutput {
    Reset: extern "win64" fn(&TextInput, bool) -> isize,
    OutputString: extern "win64" fn(&TextOutput, *const u16) -> isize,
    TestString: extern "win64" fn(&TextOutput, *const u16) -> isize,
    QueryMode: extern "win64" fn(&TextOutput, usize, &mut usize, &mut usize) -> isize,
    SetMode: extern "win64" fn(&TextOutput, usize) -> isize,
    SetAttribute: extern "win64" fn(&TextOutput, usize) -> isize,
    ClearScreen: extern "win64" fn(&TextOutput) -> isize,
    SetCursorPosition: extern "win64" fn(&TextOutput, usize, usize) -> isize,
    EnableCursor: extern "win64" fn(&TextOutput, bool) -> isize,
    pub Mode: &'static TextOutputMode,
}

impl fmt::Write for TextOutput {
    fn write_str(&mut self, string: &str) -> Result<(), fmt::Error> {
        let mut chars = string.chars();

        loop {
            let mut buf = [0u16; 256];

            let mut i = 0;
            while let Some(c) = chars.next() {
                buf[i] = c as u16; // TODO: won't work with non-BMP
                i += 1;
                if i + 1 >= buf.len() {
                    break;
                }
            }

            if i == 0 {
                break;
            } else {
                (self.OutputString)(self, buf.as_ptr());
            }
        }

        Ok(())
    }
}
