use ecflash::{Ec, EcFile, EcFlash, Flasher};
use hwio::{Io, Pio};
use std::fs::{find, load};
use uefi::status::{Error, Result};

use super::{ECROM, EC2ROM, FIRMWAREDIR, FIRMWARENSH, shell, Component};

pub struct Ps2 {
    data_port: Pio<u8>,
    cmd_port: Pio<u8>
}

impl Ps2 {
    pub fn new() -> Self {
        Self {
            data_port: Pio::new(0x60),
            cmd_port: Pio::new(0x64),
        }
    }

    pub unsafe fn can_read(&mut self) -> bool {
        self.cmd_port.read() & 1 == 1
    }

    pub unsafe fn wait_read(&mut self) {
        while ! self.can_read() {}
    }

    pub unsafe fn can_write(&mut self) -> bool {
        self.cmd_port.read() & 2 == 0
    }

    pub unsafe fn wait_write(&mut self) {
        while ! self.can_write() {}
    }

    pub unsafe fn flush(&mut self) {
        while self.can_read() {
            self.data_port.read();
        }
    }

    pub unsafe fn cmd(&mut self, data: u8) {
        self.wait_write();
        self.cmd_port.write(data);
        self.wait_write();
    }
}

pub struct EcComponent {
    master: bool,
    model: String,
    version: String,
}

impl EcComponent {
    pub fn new(master: bool) -> EcComponent {
        let mut model = String::new();
        let mut version = String::new();

        if let Ok(mut ec) = EcFlash::new(master) {
            model = ec.project();
            version = ec.version();
        }

        EcComponent {
            master: master,
            model: model,
            version: version,
        }
    }

    pub fn validate_data(&self, data: Vec<u8>) -> bool {
        match EcFlash::new(self.master).map(|mut ec| ec.project()) {
            Ok(project) => {
                if EcFile::new(data).project() == project {
                    true
                } else {
                    false
                }
            },
            Err(_err) => {
                false
            }
        }
    }

    pub fn flasher(&self) -> Option<EcFlash> {
        match self.model.as_str() {
            "N130BU" | "N130WU" | "N140WU" | "N130ZU" | "N150ZU" | "N140CU" | "N150CU" => {
                if let Ok(ec) = EcFlash::new(self.master) {
                    //TODO Some(ec)
                    None
                } else {
                    None
                }
            },
            _ => None,
        }
    }
}

impl Component for EcComponent {
    fn name(&self) -> &str {
        if self.master {
            "EC"
        } else {
            "EC2"
        }
    }

    fn path(&self) -> &str {
        if self.master {
            ECROM
        } else {
            EC2ROM
        }
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn validate(&self) -> Result<bool> {
        let data = load(self.path())?;
        Ok(self.validate_data(data))
    }

    fn flash(&self) -> Result<()> {
        if let Some(mut ec) = self.flasher() {
            let size = ec.size();

            let data = load(self.path())?;

            let blocks = (size / 0x1_0000) as u8;

            let mut success = false;
            unsafe {
                let mut ps2 = Ps2::new();
                ps2.flush();
                ps2.cmd(0xAD);

                let unlocked = {
                    println!("unlock");
                    ec.cmd(0xDE);
                    ec.cmd(0xDC);
                    ec.cmd(0xF0);
                    ec.read() == Ok(1)
                };

                if unlocked {

                    {
                        print!("erase: ");
                        ec.cmd(0x01);
                        ec.cmd(0x00);
                        ec.cmd(0x00);
                        ec.cmd(0x00);
                        ec.cmd(0x00);

                        for i in 0..64 {
                            print!("*");
                        }
                        println!();
                    }

                    for block in 0..blocks {
                        print!("verify block {}: ", block);

                        ec.cmd(0x03);
                        ec.cmd(block);
                        for i in 0x0000..0x1_0000 {
                            if i % 1024 == 0 {
                                print!("*");
                            }

                            if ec.read() != Ok(0xFF) {
                                panic!("erase failed at block {}, address 0x{:04X}", block, i);
                            }
                        }
                        println!("*");
                    }

                    for block in 0..blocks {
                        print!("write block {}: ", block);

                        let start = if block == 0 {
                            // The first 1024 bytes are programmed later
                            print!(".");
                            0x0400
                        } else {
                            0x0000
                        };

                        ec.cmd(0x02);
                        ec.cmd(0x00);
                        ec.cmd(block);
                        ec.cmd((start >> 8) as u8);
                        ec.cmd(start as u8);
                        for i in start..0x1_0000 {
                            if i % 1024 == 0 {
                                print!("*");
                            }

                            let value = data.get((block as usize) * 0x1_0000 + i).unwrap_or(&0xFF);
                            ec.write(*value);
                        }
                        println!("*");
                    }

                    {
                        print!("write block 0: ");
                        ec.cmd(0x06);
                        for i in 0x0000..0x0400 {
                            ec.write(data[i]);
                        }
                        println!("*");
                    }

                    for block in 0..blocks {
                        print!("verify block {}: ", block);

                        ec.cmd(0x03);
                        ec.cmd(block);
                        for i in 0x0000..0x1_0000 {
                            if i % 1024 == 0 {
                                print!("*");
                            }

                            let value = data.get((block as usize) * 0x1_0000 + i).unwrap_or(&0xFF);
                            if ec.read() != Ok(*value) {
                                panic!("write failed at block {}, address 0x{:04X}", block, i);
                            }
                        }
                        println!("*");
                    }

                    println!("lock");
                    ec.cmd(0xFE);

                    println!("successfully flashed {} KiB", size/1024);

                    success = true;
                }

                ps2.cmd(0xAE);
            }

            if ! success {
                return Err(Error::DeviceError);
            }
        } else {
            find(FIRMWARENSH)?;

            let cmd = if self.master {
                format!("{} {} ec flash", FIRMWARENSH, FIRMWAREDIR)
            } else {
                format!("{} {} ec2 flash", FIRMWARENSH, FIRMWAREDIR)
            };

            let (e_p, _e_v) = match EcFlash::new(self.master) {
                Ok(mut ec) => {
                    (ec.project(), ec.version())
                },
                Err(err) => {
                    println!("{} Open Error: {}", self.name(), err);
                    return Err(Error::NotFound);
                }
            };

            let (f_p, _f_v) = {
                let mut file = EcFile::new(load(self.path())?);
                (file.project(), file.version())
            };

            if e_p != f_p {
                println!("{} Project Mismatch", self.name());
                return Err(Error::DeviceError);
            }

            // We could check e_v vs f_v to verify version, and not flash if up to date
            // Instead, we rely on the Linux side to determine when it is appropriate to flash
            let status = shell(&cmd)?;
            if status != 0 {
                println!("{} Flash Error: {}", self.name(), status);
                return Err(Error::DeviceError);
            }
        }

        Ok(())
    }
}
