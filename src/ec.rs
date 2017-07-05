extern crate x86;

use alloc::string::String;

use self::x86::io::{inb, outb};

pub struct EcFlash {
    data_port: u16,
    cmd_port: u16
}

impl EcFlash {
    fn cmd(&mut self, data: u8) {
        unsafe {
            while inb(self.cmd_port) & 0x2 == 0x2 {}
            outb(self.cmd_port, data)
        }
    }

    fn read(&mut self) -> u8 {
        unsafe {
            while inb(self.cmd_port) & 0x1 == 0 {}
            inb(self.data_port)
        }
    }

    fn write(&mut self, data: u8) {
        unsafe {
            while inb(self.cmd_port) & 0x2 == 0x2 {}
            outb(self.data_port, data)
        }
    }

    fn flush(&mut self) {
        unsafe {
            while inb(self.cmd_port) & 0x1 == 0x1 {
                inb(self.data_port);
            }
        }
    }

    fn get_param(&mut self, param: u8) -> u8 {
        self.cmd(0x80);
        self.write(param);
        self.read()
    }

    fn set_param(&mut self, param: u8, data: u8) {
        self.cmd(0x81);
        self.write(param);
        self.write(data);
    }

    fn get_str(&mut self, index: u8) -> String {
        let mut string = String::new();

        self.cmd(index);
        for _i in 0..256 {
            let byte = self.read();
            if byte == b'$' {
                break;
            } else {
                string.push(byte as char);
            }
        }

        string
    }

    pub fn new(number: u8) -> Option<Self> {
        // Probe for Super I/O chip
        let id = unsafe {
            outb(0x2e, 0x20);
            let a = inb(0x2f);
            outb(0x2e, 0x21);
            let b = inb(0x2f);
            ((a as u16) << 8) | (b as u16)
        };

        if id != 0x8587 {
            return None;
        }

        let (data_port, cmd_port) = match number {
            0 => (0x60, 0x64),
            1 => (0x62, 0x66),
            2 => (0x68, 0x6c),
            3 => (0x6a, 0x6e),
            _ => {
                return None;
            }
        };

        let ec = Self {
            data_port: data_port,
            cmd_port: cmd_port,
        };

        Some(ec)
    }

    pub fn size(&mut self) -> usize {
        self.flush();

        if self.get_param(0xE5) == 0x80 {
            128 * 1024
        } else {
            64 * 1024
        }
    }

    pub fn project(&mut self) -> String {
        self.flush();

        self.get_str(0x92)
    }

    pub fn version(&mut self) -> String {
        self.flush();

        let mut version = self.get_str(0x93);
        version.insert_str(0, "1.");
        version
    }
}
