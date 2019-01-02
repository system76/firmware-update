use super::{Io, Mmio};

#[repr(packed)]
pub struct EcMem {
    bytes: [Mmio<u8>; 0x100]
}

impl EcMem {
    pub unsafe fn new(address: usize) -> &'static mut EcMem {
        &mut *(address as *mut EcMem)
    }

    pub unsafe fn read(&self, i: u8) -> u8 {
        self.bytes[i as usize].read()
    }

    pub unsafe fn write(&mut self, i: u8, data: u8) {
        self.bytes[i as usize].write(data)
    }

    pub unsafe fn adp(&self) -> bool {
        (self.read(0x10) & 0x01) == 0x01
    }

    pub unsafe fn bat0(&self) -> bool {
        (self.read(0x10) & 0x01) == 0x03
    }
}
