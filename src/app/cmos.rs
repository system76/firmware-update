// SPDX-License-Identifier: GPL-3.0-only

use hwio::{Io, Pio};

pub struct Cmos {
    port: Pio<u8>,
    data: Pio<u8>,
}

impl Cmos {
    pub const PORT_BANK0: u16 = 0x70;

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
        Self::new(Self::PORT_BANK0)
    }
}

// HACK: All boards use the same option table layout, so hard-code the logic
// so we can get meer9 working.

pub struct CmosOptionTable {
    cmos: Cmos,
}

impl CmosOptionTable {
    /// Offset into CMOS RAM of the table `check_sum`: Bit 984
    const CHECKSUM_OFFSET: u8 = (984 / 8) as u8;
    /// Offset into CMOS RAM of the option `me_state`: Bit 416
    const ME_STATE_OFFSET: u8 = (416 / 8) as u8;

    pub fn new() -> Self {
        Self {
            cmos: Cmos::default(),
        }
    }

    /// Read the checksum from the CMOS option table.
    pub fn checksum(&mut self) -> u16 {
        let hi = u16::from(self.cmos.read(Self::CHECKSUM_OFFSET));
        let lo = u16::from(self.cmos.read(Self::CHECKSUM_OFFSET + 1));

        hi << 8 | lo
    }

    /// Write the checksum to the CMOS option table.
    pub unsafe fn set_checksum(&mut self, cksum: u16) {
        let hi = (cksum >> 8) as u8;
        let lo = cksum as u8;

        self.cmos.write(Self::CHECKSUM_OFFSET, hi);
        self.cmos.write(Self::CHECKSUM_OFFSET + 1, lo);
    }

    // Get CSME state in CMOS option table.
    pub fn me_state(&mut self) -> bool {
        let state = self.cmos.read(Self::ME_STATE_OFFSET);

        // me_state
        //   0: Enable
        //   1: Disable
        state & 0x01 == 0x00
    }

    /// Set CSME state via CMOS option table.
    pub unsafe fn set_me_state(&mut self, state: bool) {
        let old_state = self.cmos.read(Self::ME_STATE_OFFSET);
        let old_cksum = self.checksum();

        // me_state
        //   0: Enable
        //   1: Disable
        let (new_state, new_cksum) = if state {
            (old_state & 0xFE, old_cksum - 1)
        } else {
            (old_state | 0x01, old_cksum + 1)
        };

        self.cmos.write(Self::ME_STATE_OFFSET, new_state);
        self.set_checksum(new_cksum);
    }

    /// Invalidate the 2-byte CMOS checksum to have coreboot erase the option
    /// table and write out the defaults.
    pub unsafe fn invalidate_checksum(&mut self) {
        let cksum = self.checksum();
        self.set_checksum(!cksum);
    }
}
