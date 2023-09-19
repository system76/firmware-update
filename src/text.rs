// SPDX-License-Identifier: GPL-3.0-only

use core::ops::Deref;
use core::{char, mem};
use orbclient::{Color, Renderer};
use std::proto::Protocol;
use std::uefi::boot::InterfaceType;
use std::uefi::guid::SIMPLE_TEXT_OUTPUT_GUID;
use std::uefi::status::{Result, Status};
use std::uefi::text::TextOutputMode;
use std::uefi::Handle;

use crate::display::{Display, Output, ScaledDisplay};

#[repr(C)]
#[allow(non_snake_case)]
pub struct TextDisplay<'a> {
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
    pub off_x: i32,
    pub off_y: i32,
    pub cols: usize,
    pub rows: usize,
    pub display: ScaledDisplay<'a>,
}

extern "win64" fn reset(_output: &mut TextDisplay, _extra: bool) -> Status {
    Status(0)
}

extern "win64" fn output_string(output: &mut TextDisplay, string: *const u16) -> Status {
    unsafe {
        output.write(string);
    }
    Status(0)
}

extern "win64" fn test_string(_output: &mut TextDisplay, _string: *const u16) -> Status {
    Status(0)
}

extern "win64" fn query_mode(
    output: &mut TextDisplay,
    _mode: usize,
    columns: &mut usize,
    rows: &mut usize,
) -> Status {
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

extern "win64" fn set_cursor_position(
    output: &mut TextDisplay,
    column: usize,
    row: usize,
) -> Status {
    output.set_cursor_pos(column as i32, row as i32);
    Status(0)
}

extern "win64" fn enable_cursor(output: &mut TextDisplay, enable: bool) -> Status {
    output.mode.CursorVisible = enable;
    Status(0)
}

impl<'a> TextDisplay<'a> {
    pub fn new(display: ScaledDisplay<'a>) -> TextDisplay<'a> {
        let mode = Box::new(TextOutputMode {
            MaxMode: 0,
            Mode: 0,
            Attribute: 0,
            CursorColumn: 0,
            CursorRow: 0,
            CursorVisible: false,
        });

        let cols = display.width() as usize / 8;
        let rows = display.height() as usize / 16;

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
            Mode: unsafe { mem::transmute(mode.deref()) },

            mode,
            off_x: 0,
            off_y: 0,
            cols,
            rows,
            display,
        }
    }

    pub fn pos(&self) -> (i32, i32) {
        (
            self.mode.CursorColumn * 8 + self.off_x,
            self.mode.CursorRow * 16 + self.off_y,
        )
    }

    pub fn clear(&mut self) {
        // Clears are ignored
        //let bg = Color::rgb(0, 0, 0);
        //self.display.rect(self.off_x, self.off_y, self.cols * 8, self.rows * 16, bg);
        //self.display.blit(0, self.off_y, w, self.rows * 16);
        self.display.sync();
    }

    pub fn scroll(&mut self, color: Color) {
        if self.rows > 0 {
            let w = self.display.width();

            let dst = self.off_y * w as i32;
            let src = (self.off_y + 16) * w as i32;
            let len = (self.rows - 1) * 16 * w as usize;
            unsafe {
                let scale = self.display.scale() as isize;
                let data_ptr = self.display.data_mut().as_mut_ptr() as *mut u32;
                crate::display::fast_copy(
                    data_ptr.offset(dst as isize * scale * scale) as *mut u8,
                    data_ptr.offset(src as isize * scale * scale) as *const u8,
                    len * (scale * scale) as usize * 4,
                );
            }

            self.display.rect(
                self.off_x,
                self.off_y + (self.rows as i32 - 1) * 16,
                self.cols as u32 * 8,
                16,
                color,
            );
        }
    }

    pub fn set_cursor_pos(&mut self, column: i32, _row: i32) {
        self.mode.CursorColumn = column;
    }

    pub unsafe fn write(&mut self, string: *const u16) {
        let bg = Color::rgb(0, 0, 0);
        let fg = Color::rgb(255, 255, 255);

        let mut scrolled = false;
        let mut changed = false;
        let (_sx, sy) = self.pos();

        let mut i = 0;
        loop {
            let w = *string.offset(i);
            if w == 0 {
                break;
            }

            let c = char::from_u32_unchecked(w as u32);

            if self.mode.CursorColumn as usize >= self.cols {
                self.mode.CursorColumn = 0;
                self.mode.CursorRow += 1;
            }

            while self.mode.CursorRow as usize >= self.rows {
                self.scroll(bg);
                self.mode.CursorRow -= 1;
                scrolled = true;
            }

            match c {
                '\x08' => {
                    if self.mode.CursorColumn > 0 {
                        let (x, y) = self.pos();
                        self.display.rect(x, y, 8, 16, bg);
                        self.mode.CursorColumn -= 1;
                        changed = true;
                    }
                }
                '\r' => {
                    self.mode.CursorColumn = 0;
                }
                '\n' => {
                    self.mode.CursorRow += 1;
                }
                _ => {
                    let (x, y) = self.pos();
                    self.display.rect(x, y, 8, 16, bg);
                    self.display.char(x, y, c, fg);
                    self.mode.CursorColumn += 1;
                    changed = true;
                }
            }

            i += 1;
        }

        if scrolled {
            let (cx, cw) = (0, self.display.width() as i32);
            let (cy, ch) = (self.off_y, self.rows as u32 * 16);
            self.display.blit(cx, cy, cw as u32, ch);
        } else if changed {
            let (_x, y) = self.pos();
            let (cx, cw) = (0, self.display.width() as i32);
            let (cy, ch) = (sy, y + 16 - sy);
            self.display.blit(cx, cy, cw as u32, ch as u32);
        }
    }

    pub fn pipe<T, F: FnMut() -> Result<T>>(&mut self, mut f: F) -> Result<T> {
        let uefi = unsafe { std::system_table_mut() };

        let stdout = self as *mut _;
        let mut stdout_handle = Handle(0);
        (uefi.BootServices.InstallProtocolInterface)(
            &mut stdout_handle,
            &SIMPLE_TEXT_OUTPUT_GUID,
            InterfaceType::Native,
            stdout as usize,
        )?;

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

        let _ = (uefi.BootServices.UninstallProtocolInterface)(
            stdout_handle,
            &SIMPLE_TEXT_OUTPUT_GUID,
            stdout as usize,
        );

        res
    }
}

pub fn pipe<T, F: FnMut() -> Result<T>>(f: F) -> Result<T> {
    let mut display = Display::new(Output::one()?);
    TextDisplay::new(ScaledDisplay::new(&mut display)).pipe(f)
}
