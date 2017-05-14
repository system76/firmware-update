use collections::Vec;
use core::{mem, slice};
use orbclient::{Color, Renderer};
use uefi;
use uefi::boot::LocateSearchType;
use uefi::guid::Guid;

pub struct Display {
    pub width: u32,
    pub height: u32,
    pub onscreen: &'static mut [Color],
}

static DISPLAY_GUID: Guid = uefi::guid::EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;

impl Display {
    fn new(output: &mut uefi::graphics::GraphicsOutput) -> Self {
        let mode = &output.Mode;

        let width = mode.Info.HorizontalResolution;
        let height = mode.Info.VerticalResolution;

        let onscreen_ptr = mode.FrameBufferBase;
        let onscreen = unsafe { slice::from_raw_parts_mut(onscreen_ptr as *mut Color, mode.FrameBufferSize/mem::size_of::<Color>()) };

        Display {
            width: width,
            height: height,
            onscreen: onscreen,
        }
    }

    fn handle_protocol(handle: uefi::Handle) -> Option<Self> {
        let uefi = unsafe { &mut *::UEFI };

        let mut interface = 0;
        let status = (uefi.BootServices.HandleProtocol)(handle, &DISPLAY_GUID, &mut interface);
        if status != 0 {
            return None;
        }

        let output = unsafe { &mut *(interface as *mut uefi::graphics::GraphicsOutput) };
        Some(Display::new(output))
    }

    fn locate_handle() -> Vec<uefi::Handle> {
        let uefi = unsafe { &mut *::UEFI };

        let mut handles = Vec::with_capacity(32);

        let mut len = handles.capacity() * mem::size_of::<uefi::Handle>();
        (uefi.BootServices.LocateHandle)(LocateSearchType::ByProtocol, &DISPLAY_GUID, 0, &mut len, handles.as_mut_ptr());

        unsafe { handles.set_len(len / mem::size_of::<uefi::Handle>()); }

        handles
    }

    pub fn all() -> Vec<Self> {
        let mut displays = Vec::new();

        for handle in Self::locate_handle() {
            if let Some(display) = Self::handle_protocol(handle) {
                displays.push(display);
            } else {
                println!("Display::all: {:?} not a display", handle);
            }
        }

        displays
    }

    pub fn scroll(&mut self, rows: usize, color: Color) {
        let width = self.width as usize;
        let height = self.height as usize;
        if rows > 0 && rows < height {
            let off1 = rows * width;
            let off2 = height * width - off1;
            unsafe {
                let data_ptr = self.onscreen.as_mut_ptr() as *mut u32;
                fast_copy(data_ptr as *mut u8, data_ptr.offset(off1 as isize) as *const u8, off2 as usize * 4);
                fast_set32(data_ptr.offset(off2 as isize), color.data, off1 as usize);
            }
        }
    }
}

impl Renderer for Display {
    /// Get width
    fn width(&self) -> u32 {
        self.width
    }

    /// Get height
    fn height(&self) -> u32 {
        self.height
    }

    /// Access the pixel buffer
    fn data(&self) -> &[Color] {
        self.onscreen
    }

    /// Access the pixel buffer mutably
    fn data_mut(&mut self) -> &mut [Color] {
        self.onscreen
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
