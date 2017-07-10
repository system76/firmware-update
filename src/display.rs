use alloc::boxed::Box;
use core::ops::Try;
use orbclient::{Color, Renderer};
use uefi::graphics::{GraphicsOutput, GraphicsBltOp, GraphicsBltPixel};
use uefi::guid::{Guid, GRAPHICS_OUTPUT_PROTOCOL_GUID};

use proto::Protocol;

pub struct Output(pub &'static mut GraphicsOutput);

impl Protocol<GraphicsOutput> for Output {
    fn guid() -> Guid {
        GRAPHICS_OUTPUT_PROTOCOL_GUID
    }

    fn new(inner: &'static mut GraphicsOutput) -> Self {
        Output(inner)
    }
}

pub struct Display {
    output: Output,
    w: u32,
    h: u32,
    data: Box<[Color]>
}

impl Display {
    pub fn new(output: Output) -> Self {
        let w = output.0.Mode.Info.HorizontalResolution;
        let h = output.0.Mode.Info.VerticalResolution;
        Self {
            output: output,
            w: w,
            h: h,
            data: vec![Color::rgb(0, 0, 0); w as usize * h as usize].into_boxed_slice()
        }
    }

    pub fn scroll(&mut self, rows: usize, color: Color) {
        let width = self.w as usize;
        let height = self.h as usize;
        if rows > 0 && rows < height {
            let off1 = rows * width;
            let off2 = height * width - off1;
            unsafe {
                let data_ptr = self.data.as_mut_ptr() as *mut u32;
                fast_copy(data_ptr as *mut u8, data_ptr.offset(off1 as isize) as *const u8, off2 as usize * 4);
                fast_set32(data_ptr.offset(off2 as isize), color.data, off1 as usize);
            }
        }
    }

    pub fn blit(&mut self, x: usize, y: usize, w: usize, h: usize) -> bool {
        let status = (self.output.0.Blt)(self.output.0, self.data.as_mut_ptr() as *mut GraphicsBltPixel, GraphicsBltOp::BufferToVideo, x, y, x, y, w, h, 0);
        status.into_result().is_ok()
    }
}

impl Renderer for Display {
    /// Get the width of the image in pixels
    fn width(&self) -> u32 {
        self.w
    }

    /// Get the height of the image in pixels
    fn height(&self) -> u32 {
        self.h
    }

    /// Return a reference to a slice of colors making up the image
    fn data(&self) -> &[Color] {
        &self.data
    }

    /// Return a mutable reference to a slice of colors making up the image
    fn data_mut(&mut self) -> &mut [Color] {
        &mut self.data
    }

    fn sync(&mut self) -> bool {
        let w = self.w as usize;
        let h = self.h as usize;
        self.blit(0, 0, w, h)
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
#[cold]
pub unsafe fn fast_copy(dst: *mut u8, src: *const u8, len: usize) {
    asm!("cld
        rep movsb"
        :
        : "{rdi}"(dst as usize), "{rsi}"(src as usize), "{rcx}"(len)
        : "cc", "memory", "rdi", "rsi", "rcx"
        : "intel", "volatile");
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
#[cold]
pub unsafe fn fast_set32(dst: *mut u32, src: u32, len: usize) {
    asm!("cld
        rep stosd"
        :
        : "{rdi}"(dst as usize), "{eax}"(src), "{rcx}"(len)
        : "cc", "memory", "rdi", "rcx"
        : "intel", "volatile");
}
