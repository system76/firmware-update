use alloc::boxed::Box;
use core::{char, mem};
use core::ops::Deref;
use orbclient::{Color, Renderer};
use uefi::Handle;
use uefi::boot::InterfaceType;
use uefi::guid::SIMPLE_TEXT_OUTPUT_GUID;
use uefi::status::{Result, Status};
use uefi::text::TextOutputMode;

use display::{Display, Output};
use proto::Protocol;

#[repr(C)]
#[allow(non_snake_case)]
pub struct TextDisplay {
    pub Reset: extern "win64" fn(&mut TextDisplay, bool) -> Status,
    pub OutputString: extern "win64" fn(&mut TextDisplay, *const u16) -> Status,
    pub TestString: extern "win64" fn(&mut TextDisplay, *const u16) -> Status,
    pub QueryMode: extern "win64" fn(&mut TextDisplay, usize, &mut usize, &mut usize) -> Status,
    pub SetMode: extern "win64" fn(&mut TextDisplay, usize) -> Status,
    pub SetAttribute: extern "win64" fn(&mut TextDisplay, usize) -> Status,
    pub ClearScreen: extern "win64" fn(&mut TextDisplay) -> Status,
    pub SetCursorPosition: extern "win64" fn(&mut TextDisplay, usize, usize) -> Status,
    pub EnableCursor: extern "win64" fn(&mut TextDisplay, bool) -> Status,
    pub Mode: &'static TextOutputMode,

    pub mode: Box<TextOutputMode>,
    pub cols: usize,
    pub rows: usize,
    pub display: Display,
}

extern "win64" fn reset(_output: &mut TextDisplay, _extra: bool) -> Status {
    Status(0)
}

extern "win64" fn output_string(output: &mut TextDisplay, string: *const u16) -> Status {
    let mut i = 0;
    loop {
        let w = unsafe { *string.offset(i) };
        if w == 0 {
            break;
        }
        output.char(unsafe { char::from_u32_unchecked(w as u32) });
        i += 1;
    }
    Status(0)
}

extern "win64" fn test_string(_output: &mut TextDisplay, _string: *const u16) -> Status {
    Status(0)
}

extern "win64" fn query_mode(output: &mut TextDisplay, _mode: usize, columns: &mut usize, rows: &mut usize) -> Status {
    *columns = output.cols;
    *rows = output.rows;
    Status(0)
}

extern "win64" fn set_mode(_output: &mut TextDisplay, _mode: usize) -> Status {
    Status(0)
}

extern "win64" fn set_attribute(output: &mut TextDisplay, attribute: usize) -> Status {
    output.mode.Attribute = attribute as i32;
    Status(0)
}

extern "win64" fn clear_screen(output: &mut TextDisplay) -> Status {
    output.clear();
    Status(0)
}

extern "win64" fn set_cursor_position(output: &mut TextDisplay, column: usize, row: usize) -> Status {
    output.mode.CursorColumn = column as i32;
    output.mode.CursorRow = row as i32;
    Status(0)
}

extern "win64" fn enable_cursor(output: &mut TextDisplay, enable: bool) -> Status {
    output.mode.CursorVisible = enable;
    Status(0)
}

impl TextDisplay {
    pub fn new(display: Display) -> TextDisplay {
        let mode = Box::new(TextOutputMode {
            MaxMode: 0,
            Mode: 0,
            Attribute: 0,
            CursorColumn: 0,
            CursorRow: 0,
            CursorVisible: false,
        });

        let cols = display.width() as usize/8;
        let rows = display.height() as usize/16;

        TextDisplay {
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

            mode: mode,
            cols: cols,
            rows: rows,
            display: display,
        }
    }

    pub fn clear(&mut self) {
        // Clears are ignored
        //let bg = Color::rgb(0, 0, 0);
        //self.display.set(bg);
        //self.display.sync();
    }

    pub fn char(&mut self, c: char) {
        let bg = Color::rgb(0, 0, 0);
        let fg = Color::rgb(255, 255, 255);

        if self.mode.CursorColumn as usize >= self.cols {
            self.mode.CursorColumn = 0;
            self.mode.CursorRow += 1;
        }

        while self.mode.CursorRow as usize >= self.rows {
            self.display.scroll(16, bg);
            self.display.sync();
            self.mode.CursorRow -= 1;
        }

        match c {
            '\r'=> {
                self.mode.CursorColumn = 0;
            },
            '\n' => {
                self.mode.CursorRow += 1;
            },
            _ => {
                let x = self.mode.CursorColumn * 8;
                let y = self.mode.CursorRow * 16;
                self.display.rect(x, y, 8, 16, bg);
                self.display.char(x, y, c, fg);

                let w = self.display.width();
                self.display.blit(0, y as usize, w as usize, 16);

                self.mode.CursorColumn += 1;
            }
        }
    }
}

pub fn pipe<T, F: FnMut() -> Result<T>>(mut f: F) -> Result<T> {
    let uefi = unsafe { &mut *::UEFI };

    let mut stdout = TextDisplay::new(Display::new(Output::one()?));
    let mut stdout_handle = Handle(0);
    (uefi.BootServices.InstallProtocolInterface)(&mut stdout_handle, &SIMPLE_TEXT_OUTPUT_GUID, InterfaceType::Native, (&mut stdout) as *mut _ as usize)?;

    let old_stdout_handle = uefi.ConsoleOutHandle;
    let old_stdout = uefi.ConsoleOut as *mut _;
    let old_stderr_handle = uefi.ConsoleErrorHandle;
    let old_stderr = uefi.ConsoleError as *mut _;

    uefi.ConsoleOutHandle = stdout_handle;
    uefi.ConsoleOut = unsafe { mem::transmute(&mut stdout) };
    uefi.ConsoleErrorHandle = stdout_handle;
    uefi.ConsoleError = unsafe { mem::transmute(&mut stdout) };

    let res = f();

    uefi.ConsoleOutHandle = old_stdout_handle;
    uefi.ConsoleOut = unsafe { mem::transmute(&mut *old_stdout) };
    uefi.ConsoleErrorHandle = old_stderr_handle;
    uefi.ConsoleError = unsafe { mem::transmute(&mut *old_stderr) };

    let _ = (uefi.BootServices.UninstallProtocolInterface)(stdout_handle, &SIMPLE_TEXT_OUTPUT_GUID, (&mut stdout) as *mut _ as usize);

    res
}