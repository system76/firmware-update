// SPDX-License-Identifier: GPL-3.0-only

use hwio::{Io, Pio};

pub struct Cmos {
    port: Pio<u8>,
    data: Pio<u8>,
}

impl Cmos {
    pub fn new(port: u16) -> Self {
        Self {
            port: Pio::<u8>::new(port),
            data: Pio::<u8>::new(port + 1),
        }
    }

    pub fn read(&mut self, addr: u8) -> u8 {
        self.port.write(addr);
        self.data.read()
    }

    pub fn write(&mut self, addr: u8, data: u8) {
        self.port.write(addr);
        self.data.write(data);
    }
}

impl Default for Cmos {
    fn default() -> Self {
        Self::new(0x70)
    }
}
