use core::mem;
use core::ops::Deref;
use uefi::Handle;
use uefi::boot::InterfaceType;
use uefi::guid::SIMPLE_TEXT_OUTPUT_GUID;
use uefi::status::{Result, Status};
use uefi::text::TextOutputMode;

#[repr(C)]
#[allow(non_snake_case)]
pub struct NullDisplay {
    pub Reset: extern "win64" fn(&mut NullDisplay, bool) -> Status,
    pub OutputString: extern "win64" fn(&mut NullDisplay, *const u16) -> Status,
    pub TestString: extern "win64" fn(&mut NullDisplay, *const u16) -> Status,
    pub QueryMode: extern "win64" fn(&mut NullDisplay, usize, &mut usize, &mut usize) -> Status,
    pub SetMode: extern "win64" fn(&mut NullDisplay, usize) -> Status,
    pub SetAttribute: extern "win64" fn(&mut NullDisplay, usize) -> Status,
    pub ClearScreen: extern "win64" fn(&mut NullDisplay) -> Status,
    pub SetCursorPosition: extern "win64" fn(&mut NullDisplay, usize, usize) -> Status,
    pub EnableCursor: extern "win64" fn(&mut NullDisplay, bool) -> Status,
    pub Mode: &'static TextOutputMode,

    pub mode: Box<TextOutputMode>,
}

extern "win64" fn reset(_output: &mut NullDisplay, _extra: bool) -> Status {
    Status(0)
}

extern "win64" fn output_string(_output: &mut NullDisplay, _string: *const u16) -> Status {
    Status(0)
}

extern "win64" fn test_string(_output: &mut NullDisplay, _string: *const u16) -> Status {
    Status(0)
}

extern "win64" fn query_mode(_output: &mut NullDisplay, _mode: usize, columns: &mut usize, rows: &mut usize) -> Status {
    *columns = 80;
    *rows = 30;
    Status(0)
}

extern "win64" fn set_mode(_output: &mut NullDisplay, _mode: usize) -> Status {
    Status(0)
}

extern "win64" fn set_attribute(output: &mut NullDisplay, attribute: usize) -> Status {
    output.mode.Attribute = attribute as i32;
    Status(0)
}

extern "win64" fn clear_screen(_output: &mut NullDisplay) -> Status {
    Status(0)
}

extern "win64" fn set_cursor_position(output: &mut NullDisplay, column: usize, row: usize) -> Status {
    output.mode.CursorColumn = column as i32;
    output.mode.CursorRow = row as i32;
    Status(0)
}

extern "win64" fn enable_cursor(output: &mut NullDisplay, enable: bool) -> Status {
    output.mode.CursorVisible = enable;
    Status(0)
}

impl NullDisplay {
    pub fn new() -> NullDisplay {
        let mode = Box::new(TextOutputMode {
            MaxMode: 0,
            Mode: 0,
            Attribute: 0,
            CursorColumn: 0,
            CursorRow: 0,
            CursorVisible: false,
        });

        NullDisplay {
            Reset: reset,
            OutputString: output_string,
            TestString: test_string,
            QueryMode: query_mode,
            SetMode: set_mode,
            SetAttribute: set_attribute,
            ClearScreen: clear_screen,
            SetCursorPosition: set_cursor_position,
            EnableCursor: enable_cursor,
            Mode: unsafe { mem::transmute(&*mode.deref()) },

            mode: mode
        }
    }

    pub fn pipe<T, F: FnMut() -> Result<T>>(&mut self, mut f: F) -> Result<T> {
        let uefi = unsafe { std::system_table_mut() };

        let stdout = self as *mut _;
        let mut stdout_handle = Handle(0);
        (uefi.BootServices.InstallProtocolInterface)(&mut stdout_handle, &SIMPLE_TEXT_OUTPUT_GUID, InterfaceType::Native, stdout as usize)?;

        let old_stdout_handle = uefi.ConsoleOutHandle;
        let old_stdout = uefi.ConsoleOut as *mut _;
        let old_stderr_handle = uefi.ConsoleErrorHandle;
        let old_stderr = uefi.ConsoleError as *mut _;

        uefi.ConsoleOutHandle = stdout_handle;
        uefi.ConsoleOut = unsafe { mem::transmute(&mut *stdout) };
        uefi.ConsoleErrorHandle = stdout_handle;
        uefi.ConsoleError = unsafe { mem::transmute(&mut *stdout) };

        let res = f();

        uefi.ConsoleOutHandle = old_stdout_handle;
        uefi.ConsoleOut = unsafe { mem::transmute(&mut *old_stdout) };
        uefi.ConsoleErrorHandle = old_stderr_handle;
        uefi.ConsoleError = unsafe { mem::transmute(&mut *old_stderr) };

        let _ = (uefi.BootServices.UninstallProtocolInterface)(stdout_handle, &SIMPLE_TEXT_OUTPUT_GUID, stdout as usize);

        res
    }
}

pub fn pipe<T, F: FnMut() -> Result<T>>(f: F) -> Result<T> {
    NullDisplay::new().pipe(f)
}
