// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2018-2021 System76 <info@system76.com>

use core::ptr;

// P2SB private registers.
const P2SB_PORTID_SHIFT: u32 = 16;

// GPIO sideband registers.
const REG_PCH_GPIO_PADBAR: u32 = 0xc;

pub struct Sideband {
    pub addr: u64,
}

#[allow(dead_code)]
impl Sideband {
    pub unsafe fn new(sbreg_phys: usize) -> Self {
        // On UEFI, physical memory is identity mapped
        Self {
            addr: sbreg_phys as u64,
        }
    }

    #[must_use]
    pub unsafe fn read(&self, port: u8, reg: u32) -> u32 {
        let offset = (u64::from(port) << P2SB_PORTID_SHIFT) + u64::from(reg);
        if offset < 1 << 24 {
            let addr = self.addr + offset;
            unsafe { ptr::read(addr as *mut u32) }
        } else {
            0
        }
    }

    pub unsafe fn write(&self, port: u8, reg: u32, value: u32) {
        let offset = (u64::from(port) << P2SB_PORTID_SHIFT) + u64::from(reg);
        if offset < 1 << 24 {
            let addr = self.addr + offset;
            unsafe { ptr::write(addr as *mut u32, value) };
        }
    }

    #[must_use]
    pub unsafe fn gpio(&self, port: u8, pad: u8) -> u64 {
        unsafe {
            let padbar: u32 = self.read(port, REG_PCH_GPIO_PADBAR);

            let dw1: u32 = self.read(port, padbar + u32::from(pad) * 8 + 4);
            let dw0: u32 = self.read(port, padbar + u32::from(pad) * 8);

            u64::from(dw0) | (u64::from(dw1) << 32)
        }
    }

    pub unsafe fn set_gpio(&self, port: u8, pad: u8, value: u64) {
        unsafe {
            let padbar: u32 = self.read(port, REG_PCH_GPIO_PADBAR);

            self.write(port, padbar + u32::from(pad) * 8 + 4, (value >> 32) as u32);
            self.write(port, padbar + u32::from(pad) * 8, value as u32);
        }
    }
}
