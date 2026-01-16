pub struct IntelPmc {
    pwrmbase: usize,
    gen_pmcon_b_offset: usize,
    gen_pmcon_b_rtc_pwr_sts: u32,
}

impl IntelPmc {
    pub fn tigerlake() -> Self {
        Self {
            pwrmbase: 0xfe000000,
            gen_pmcon_b_offset: 0x1024,
            gen_pmcon_b_rtc_pwr_sts: 1 << 2,
        }
    }

    unsafe fn read(&self, offset: usize) -> u32 {
        let ptr = (self.pwrmbase + offset) as *const u32;
        unsafe { ptr.read_volatile() }
    }

    unsafe fn write(&self, offset: usize, value: u32) {
        let ptr = (self.pwrmbase + offset) as *mut u32;
        unsafe { ptr.write_volatile(value) }
    }

    pub unsafe fn set_rtc_pwr_sts(&self) {
        let mut value = unsafe { self.read(self.gen_pmcon_b_offset) };
        value |= self.gen_pmcon_b_rtc_pwr_sts;
        unsafe { self.write(self.gen_pmcon_b_offset, value); }
    }
}