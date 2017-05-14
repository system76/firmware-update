use core::{mem, slice};
use orbclient::{Color, Renderer};
use uefi::graphics::GraphicsOutput;
use uefi::guid::{Guid, EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID};

use proto::Protocol;

pub struct Display(&'static mut GraphicsOutput);

impl Protocol<GraphicsOutput> for Display {
    fn guid() -> Guid {
        EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID
    }

    fn new(inner: &'static mut GraphicsOutput) -> Self {
        Display(inner)
    }
}

impl Display {
    pub fn scroll(&mut self, rows: usize, color: Color) {
        let width = self.width() as usize;
        let height = self.height() as usize;
        if rows > 0 && rows < height {
            let off1 = rows * width;
            let off2 = height * width - off1;
            unsafe {
                let data_ptr = self.data_mut().as_mut_ptr() as *mut u32;
                fast_copy(data_ptr as *mut u8, data_ptr.offset(off1 as isize) as *const u8, off2 as usize * 4);
                fast_set32(data_ptr.offset(off2 as isize), color.data, off1 as usize);
            }
        }
    }
}

impl Renderer for Display {
    /// Get width
    fn width(&self) -> u32 {
        self.0.Mode.Info.HorizontalResolution
    }

    /// Get height
    fn height(&self) -> u32 {
        self.0.Mode.Info.VerticalResolution
    }

    /// Access the pixel buffer
    fn data(&self) -> &[Color] {
        unsafe {
            slice::from_raw_parts(
                self.0.Mode.FrameBufferBase as *const Color,
                self.0.Mode.FrameBufferSize/mem::size_of::<Color>()
            )
        }
    }

    /// Access the pixel buffer mutably
    fn data_mut(&mut self) -> &mut [Color] {
        unsafe {
            slice::from_raw_parts_mut(
                self.0.Mode.FrameBufferBase as *mut Color,
                self.0.Mode.FrameBufferSize/mem::size_of::<Color>()
            )
        }
    }

    /// Flip the buffer
    fn sync(&mut self) -> bool {
        true
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
