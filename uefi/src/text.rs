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

#[derive(Clone, Debug)]
#[repr(C)]
pub struct TextOutputMode {
    pub MaxMode: i32,
    pub Mode: i32,
    pub Attribute: i32,
    pub CursorColumn: i32,
    pub CursorRow: i32,
    pub CursorVisible: bool,
}

#[repr(C)]
pub struct TextOutput {
    Reset: extern "win64" fn(&TextInput, bool) -> isize,
    OutputString: extern "win64" fn(&TextOutput, *const u16) -> isize,
    TestString: extern "win64" fn(&TextOutput, *const u16) -> isize,
    pub QueryMode: extern "win64" fn(&TextOutput, usize, &mut usize, &mut usize) -> isize,
    pub SetMode: extern "win64" fn(&TextOutput, usize) -> isize,
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
                if c == '\n' {
                    buf[i] = '\r' as u16;
                    i += 1;
                }
                if i + 2 >= buf.len() {
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
