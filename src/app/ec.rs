use ecflash::{Ec, EcFile, EcFlash};
use ectool::{
    Firmware,
    Spi,
    SpiRom,
    SpiTarget,
    Timeout,
};
use std::{
    cell::Cell,
    fs::{find, load},
    str,
};
use uefi::status::{Error, Result};

use super::{ECROM, EC2ROM, FIRMWAREDIR, FIRMWARENSH, shell, Component};

pub struct UefiTimeout {
    duration: u64,
    elapsed: Cell<u64>,
}

impl UefiTimeout {
    pub fn new(duration: u64) -> Self {
        Self {
            duration,
            elapsed: Cell::new(0),
        }
    }
}

impl Timeout for UefiTimeout {
    fn reset(&mut self) {
        self.elapsed.set(0);
    }

    fn running(&self) -> bool {
        let elapsed = self.elapsed.get() + 1;
        let _ = (std::system_table().BootServices.Stall)(1);
        self.elapsed.set(elapsed);
        elapsed < self.duration
    }
}

unsafe fn flash_read<S: Spi>(spi: &mut SpiRom<S, UefiTimeout>, rom: &mut [u8], sector_size: usize) -> core::result::Result<(), ectool::Error> {
    let mut address = 0;
    while address < rom.len() {
        print!("\rSPI Read {}K", address / 1024);
        let next_address = address + sector_size;
        let count = spi.read_at(address as u32, &mut rom[address..next_address])?;
        if count != sector_size {
            println!("\ncount {} did not match sector size {}", count, sector_size);
            return Err(ectool::Error::Verify);
        }
        address = next_address;
    }
    println!("\rSPI Read {}K", address / 1024);
    Ok(())
}

unsafe fn flash_inner(ec: &mut ectool::Ec<UefiTimeout>, firmware: &Firmware, target: SpiTarget, scratch: bool) -> core::result::Result<(), ectool::Error> {
    let rom_size = 128 * 1024;
    let sector_size = 1024;

    let mut new_rom = firmware.data.to_vec();
    while new_rom.len() < rom_size {
        new_rom.push(0xFF);
    }

    let mut spi_bus = ec.spi(target, scratch)?;
    let mut spi = SpiRom::new(
        &mut spi_bus,
        UefiTimeout::new(1_000_000)
    );

    let mut rom = vec![0xFF; rom_size];
    flash_read(&mut spi, &mut rom, sector_size)?;

    // Program chip, sector by sector
    //TODO: write signature last
    {
        let mut address = 0;
        while address < rom_size {
            print!("\rSPI Write {}K", address / 1024);

            let next_address = address + sector_size;

            let mut matches = true;
            let mut erased = true;
            let mut new_erased = true;
            for i in address..next_address {
                if rom[i] != new_rom[i] {
                    matches = false;
                }
                if rom[i] != 0xFF {
                    erased = false;
                }
                if new_rom[i] != 0xFF {
                    new_erased = false;
                }
            }

            if ! matches {
                if ! erased {
                    spi.erase_sector(address as u32)?;
                }
                if ! new_erased {
                    let count = spi.write_at(address as u32, &new_rom[address..next_address])?;
                    if count != sector_size {
                        println!("\nWrite count {} did not match sector size {}", count, sector_size);
                        return Err(ectool::Error::Verify);
                    }
                }
            }

            address = next_address;
        }
        println!("\rSPI Write {}K", address / 1024);

        // Verify chip write
        flash_read(&mut spi, &mut rom, sector_size)?;
        for i in 0..rom.len() {
            if rom[i] != new_rom[i] {
                println!("Failed to program: {:X} is {:X} instead of {:X}", i, rom[i], new_rom[i]);
                return Err(ectool::Error::Verify);
            }
        }
    }

    println!("Successfully programmed SPI ROM");

    Ok(())
}

enum EcKind {
    System76(ectool::Ec<UefiTimeout>),
    Legacy(EcFlash),
    Unknown,
}

impl EcKind {
    unsafe fn new(primary: bool) -> Self {
        if let Ok(ec) = ectool::Ec::new(UefiTimeout::new(100_000)) {
            return EcKind::System76(ec);
        }

        if let Ok(ec) = EcFlash::new(primary) {
            return EcKind::Legacy(ec);
        }

        EcKind::Unknown
    }

    unsafe fn model(&mut self) -> String {
        match self {
            EcKind::System76(ec) => {
                let mut data = [0; 256];
                if let Ok(count) = ec.board(&mut data) {
                    if let Ok(string) = str::from_utf8(&data[..count]) {
                        return string.to_string();
                    }
                }
            },
            EcKind::Legacy(ec) => {
                return ec.project();
            },
            EcKind::Unknown => (),
        }
        String::new()
    }

    unsafe fn version(&mut self) -> String {
        match self {
            EcKind::System76(ec) => {
                let mut data = [0; 256];
                if let Ok(count) = ec.version(&mut data) {
                    if let Ok(string) = str::from_utf8(&data[..count]) {
                        return string.to_string();
                    }
                }
            },
            EcKind::Legacy(ec) => {
                return ec.version();
            },
            EcKind::Unknown => (),
        }
        String::new()
    }

    fn firmware_model(&self, data: Vec<u8>) -> String {
        match self {
            EcKind::System76(_) => {
                if let Some(firmware) = Firmware::new(&data) {
                    if let Ok(string) = str::from_utf8(firmware.board) {
                        return string.to_string();
                    }
                }
            },
            EcKind::Legacy(_) => {
                return EcFile::new(data).project();
            },
            EcKind::Unknown => (),
        }
        String::new()
    }
}

pub struct EcComponent {
    master: bool,
    ec: EcKind,
    model: String,
    version: String,
}

impl EcComponent {
    pub fn new(master: bool) -> EcComponent {
        unsafe {
            let mut ec = EcKind::new(master);
            let model = ec.model();
            let version = ec.version();

            EcComponent {
                ec,
                master,
                model,
                version,
            }
        }
    }

    pub fn validate_data(&self, data: Vec<u8>) -> bool {
        let firmware_model = self.ec.firmware_model(data);
        ! self.model.is_empty() &&
        ! self.version.is_empty() &&
        firmware_model == self.model
    }
}

unsafe fn flash(firmware_data: &[u8]) -> core::result::Result<(), ectool::Error> {
    let target = SpiTarget::Main;
    let scratch = true;

    let firmware = match Firmware::new(&firmware_data) {
        Some(some) => some,
        None => {
            println!("failed to parse firmware");
            return Err(ectool::Error::Verify);
        }
    };
    println!("file board: {:?}", str::from_utf8(firmware.board));
    println!("file version: {:?}", str::from_utf8(firmware.version));

    let mut ec = ectool::Ec::new(UefiTimeout::new(1_000_000))?;

    {
        let mut data = [0; 256];
        let size = ec.board(&mut data)?;

        let ec_board = &data[..size];
        println!("ec board: {:?}", str::from_utf8(ec_board));

        if ec_board != firmware.board {
            println!("file board does not match ec board");
            return Err(ectool::Error::Verify);
        }
    }

    {
        let mut data = [0; 256];
        let size = ec.version(&mut data)?;

        let ec_version = &data[..size];
        println!("ec version: {:?}", str::from_utf8(ec_version));
    }

    let res = flash_inner(&mut ec, &firmware, target, scratch);
    println!("Result: {:X?}", res);

    if scratch {
        println!("System will shut off in 5 seconds");
        let _ = (std::system_table().BootServices.Stall)(5_000_000);

        ec.reset()?;
    }

    res
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
        match &self.ec {
            EcKind::System76(_) => {
                let firmware_data = load(self.path())?;
                match unsafe { flash(&firmware_data) } {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        println!("{} Flash Error: {:X?}", self.name(), err);
                        Err(Error::DeviceError)
                    }
                }
            },
            EcKind::Legacy(_) => {
                find(FIRMWARENSH)?;

                let cmd = if self.master {
                    format!("{} {} ec flash", FIRMWARENSH, FIRMWAREDIR)
                } else {
                    format!("{} {} ec2 flash", FIRMWARENSH, FIRMWAREDIR)
                };

                let status = shell(&cmd)?;
                if status == 0 {
                    Ok(())
                } else {
                    println!("{} Flash Error: {}", self.name(), status);
                    Err(Error::DeviceError)
                }
            },
            EcKind::Unknown => {
                println!("{} Failed to flash EcKind::Unknown", self.name());
                Err(Error::DeviceError)
            },
        }
    }
}
