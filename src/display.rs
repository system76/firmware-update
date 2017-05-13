use core::{mem, slice};
use collections::Vec;
use orbclient::{Color, Renderer};
use uefi;
use uefi::boot::LocateSearchType;
use uefi::guid::Guid;

pub struct Display {
    pub width: u32,
    pub height: u32,
    pub buffer: &'static mut [Color]
}

static DISPLAY_GUID: Guid = uefi::guid::EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;

impl Display {
    pub fn new(handle: uefi::Handle) -> Option<Display> {
        let uefi = unsafe { &mut *::UEFI };

        let mut interface = 0;
        let status = (uefi.BootServices.HandleProtocol)(handle, &DISPLAY_GUID, &mut interface);
        if status != 0 {
            println!("Failed to get display: {}", status);
            return None;
        }

        let output = unsafe { &mut *(interface as *mut uefi::graphics::GraphicsOutput) };
        let mode = &output.Mode;

        let buffer = unsafe { slice::from_raw_parts_mut(mode.FrameBufferBase as *mut Color, mode.FrameBufferSize/mem::size_of::<Color>()) };

        Some(Display {
            width: mode.Info.HorizontalResolution,
            height: mode.Info.VerticalResolution,
            buffer: buffer
        })
    }

    pub fn all() -> Vec<Display> {
        let mut displays = Vec::new();

        let uefi = unsafe { &mut *::UEFI };

        let mut handles = [uefi::Handle(0); 32];
        let mut len = handles.len() * mem::size_of::<uefi::Handle>();
        (uefi.BootServices.LocateHandle)(LocateSearchType::ByProtocol, &DISPLAY_GUID, 0, &mut len, handles.as_mut_ptr());

        let count = len / mem::size_of::<uefi::Handle>();
        println!("Graphics Outputs: {}", count);
        for i in 0..count {
            if let Some(handle) = handles.get(i) {
                if let Some(display) = Display::new(*handle) {
                    displays.push(display);
                } else {
                    println!("  {}: {:?} not a display", i, handle);
                }
            } else {
                println!("  {}: out of buffer", i);
            }
        }

        displays
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
        self.buffer
    }

    /// Access the pixel buffer mutably
    fn data_mut(&mut self) -> &mut [Color] {
        self.buffer
    }

    /// Flip the buffer
    fn sync(&mut self) -> bool {
        true
    }
}
