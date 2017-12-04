use alloc::boxed::Box;
use core::cmp;
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
    scale: u32,
    w: u32,
    h: u32,
    data: Box<[Color]>
}

impl Display {
    pub fn new(output: Output) -> Self {
        let w = output.0.Mode.Info.HorizontalResolution;
        let h = output.0.Mode.Info.VerticalResolution;
        let scale = if h > 1440 {
            2
        } else {
            1
        };
        Self {
            output: output,
            scale: scale,
            w: w,
            h: h,
            data: vec![Color::rgb(0, 0, 0); w as usize * h as usize].into_boxed_slice()
        }
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn scroll(&mut self, rows: usize, color: Color) {
        let scale = self.scale as usize;
        self.inner_scroll(rows * scale, color);
    }

    pub fn blit(&mut self, x: i32, y: i32, w: u32, h: u32) -> bool {
        let scale = self.scale;
        self.inner_blit(
            x * scale as i32,
            y * scale as i32,
            w * scale,
            h * scale
        )
    }

    fn inner_blit(&mut self, x: i32, y: i32, w: u32, h: u32) -> bool {
        let status = (self.output.0.Blt)(
            self.output.0,
            self.data.as_mut_ptr() as *mut GraphicsBltPixel,
            GraphicsBltOp::BufferToVideo,
            x as usize,
            y as usize,
            x as usize,
            y as usize,
            w as usize,
            h as usize,
            0
        );
        status.into_result().is_ok()
    }

    fn inner_scroll(&mut self, rows: usize, color: Color) {
        let width = self.w as usize;
        let height = self.h as usize;
        if rows > 0 && rows < height {
            let off1 = rows * width;
            let off2 = height * width - off1;
            unsafe {
                let data_ptr = self.data.as_mut_ptr() as *mut u32;
                fast_copy(data_ptr as *mut u8, data_ptr.offset(off1 as isize) as *const u8, off2 as usize * 4);
                fast_set32(data_ptr.offset(off2 as isize), color.0, off1 as usize);
            }
        }
    }

    fn inner_pixel(&mut self, x: i32, y: i32, color: Color) {
        let w = self.w;
        let h = self.h;

        if x >= 0 && y >= 0 && x < w as i32 && y < h as i32 {
            let new = color.0;

            let alpha = (new >> 24) & 0xFF;
            if alpha > 0 {
                let old = &mut self.data[y as usize * w as usize + x as usize];
                if alpha >= 255 {
                    old.0 = new;
                } else {
                    let n_r = (((new >> 16) & 0xFF) * alpha) >> 8;
                    let n_g = (((new >> 8) & 0xFF) * alpha) >> 8;
                    let n_b = ((new & 0xFF) * alpha) >> 8;

                    let n_alpha = 255 - alpha;
                    let o_a = (((old.0 >> 24) & 0xFF) * n_alpha) >> 8;
                    let o_r = (((old.0 >> 16) & 0xFF) * n_alpha) >> 8;
                    let o_g = (((old.0 >> 8) & 0xFF) * n_alpha) >> 8;
                    let o_b = ((old.0 & 0xFF) * n_alpha) >> 8;

                    old.0 = ((o_a << 24) | (o_r << 16) | (o_g << 8) | o_b) + ((alpha << 24) | (n_r << 16) | (n_g << 8) | n_b);
                }
            }
        }
    }

    fn inner_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        let self_w = self.w;
        let self_h = self.h;

        let start_y = cmp::max(0, cmp::min(self_h as i32 - 1, y));
        let end_y = cmp::max(start_y, cmp::min(self_h as i32, y + h as i32));

        let start_x = cmp::max(0, cmp::min(self_w as i32 - 1, x));
        let len = cmp::max(start_x, cmp::min(self_w as i32, x + w as i32)) - start_x;

        let alpha = (color.0 >> 24) & 0xFF;
        if alpha > 0 {
            if alpha >= 255 {
                for y in start_y..end_y {
                    unsafe {
                        fast_set32(self.data.as_mut_ptr().offset((y * self_w as i32 + start_x) as isize) as *mut u32, color.0, len as usize);
                    }
                }
            } else {
                for y in start_y..end_y {
                    for x in start_x..start_x + len {
                        self.inner_pixel(x, y, color);
                    }
                }
            }
        }
    }
}

impl Renderer for Display {
    /// Get the width of the image in pixels
    fn width(&self) -> u32 {
        self.w/self.scale
    }

    /// Get the height of the image in pixels
    fn height(&self) -> u32 {
        self.h/self.scale
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
        let w = self.width();
        let h = self.height();
        self.blit(0, 0, w, h)
    }

    fn pixel(&mut self, x: i32, y: i32, color: Color) {
        self.rect(x, y, 1, 1, color);
    }

    fn rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        let scale = self.scale;
        self.inner_rect(
            x * scale as i32,
            y * scale as i32,
            w * scale,
            h * scale,
            color
        );
    }

    fn set(&mut self, color: Color) {
        let w = self.width();
        let h = self.height();
        self.rect(0, 0, w, h, color);
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
