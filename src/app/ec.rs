use core::ops::Try;
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
    ffi::wstr,
    fs::{find, load},
    str,
};
use uefi::status::{Error, Result};

use super::{ECROM, EC2ROM, ECTAG, FIRMWAREDIR, FIRMWARENSH, shell, Component};

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
        if let Some(firmware) = Firmware::new(&data) {
            if let Ok(string) = str::from_utf8(firmware.board) {
                string.to_string()
            } else {
                String::new()
            }
        } else {
            EcFile::new(data).project()
        }
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
        let normalize_model = |model: &str| -> String {
            match model {
                "L140CU" => "system76/lemp9".to_string(),
                "L140MU" => "system76/lemp10".to_string(),
                "N130ZU" => "system76/galp3-c".to_string(),
                "N140CU" => "system76/galp4".to_string(),
                "N150ZU" => "system76/darp5".to_string(),
                "N150CU" => "system76/darp6".to_string(),
                "NH50DB" | "NH5xDC" => "system76/gaze15".to_string(),
                "NS50MU" => "system76/darp7".to_string(),
                "NV40Mx" | "NV40Mx-DV" => "system76/galp5".to_string(),
                "PB50Ex" => "system76/addw1".to_string(),
                "PBx0Dx2" => "system76/addw2".to_string(),
                "P950Ex" => "system76/oryp5".to_string(),
                "PCx0Dx2" => "system76/oryp6".to_string(),
                "X170SM-G" => "system76/bonw14".to_string(),
                _ => model.to_string(),
            }
        };
        let firmware_model = self.ec.firmware_model(data);
        ! self.model.is_empty() &&
        ! self.version.is_empty() &&
        normalize_model(&firmware_model) == normalize_model(&self.model)
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

unsafe fn flash(firmware_data: &[u8], target: SpiTarget) -> core::result::Result<(), ectool::Error> {
    let mut ec = ectool::Ec::new(UefiTimeout::new(1_000_000))?;

    println!("Programming EC {} ROM", match target {
        SpiTarget::Main => "Main",
        SpiTarget::Backup => "Backup",
    });

    {
        let mut data = [0; 256];
        let size = ec.board(&mut data)?;

        let ec_board = &data[..size];
        println!("ec board: {:?}", str::from_utf8(ec_board));
    }

    {
        let mut data = [0; 256];
        let size = ec.version(&mut data)?;

        let ec_version = &data[..size];
        println!("ec version: {:?}", str::from_utf8(ec_version));
    }

    let rom_size = 128 * 1024;

    let mut new_rom = firmware_data.to_vec();
    while new_rom.len() < rom_size {
        new_rom.push(0xFF);
    }

    let mut spi_bus = ec.spi(SpiTarget::Main, true)?;
    let mut spi = SpiRom::new(
        &mut spi_bus,
        UefiTimeout::new(1_000_000)
    );
    let sector_size = spi.sector_size();

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

    println!("Successfully programmed EC {} ROM", match target {
        SpiTarget::Main => "Main",
        SpiTarget::Backup => "Backup",
    });

    Ok(())
}

unsafe fn watchdog_reset() {
    let d2_read = |addr: u8| -> u8 {
        let mut super_io = ectool::SuperIo::new(0x2E);
        super_io.write(0x2E, addr);
        super_io.read(0x2F)
    };

    let d2_write = |addr: u8, value: u8| {
        let mut super_io = ectool::SuperIo::new(0x2E);
        super_io.write(0x2E, addr);
        super_io.write(0x2F, value);
    };

    let i2ec_read = |addr: u16| -> u8 {
        d2_write(0x11, (addr >> 8) as u8);
        d2_write(0x10, addr as u8);
        d2_read(0x12)
    };

    let i2ec_write = |addr: u16, value: u8| {
        d2_write(0x11, (addr >> 8) as u8);
        d2_write(0x10, addr as u8);
        d2_write(0x12, value);
    };

    i2ec_write(0x1F01, i2ec_read(0x1F01) | (1 << 5));
    i2ec_write(0x1F07, 0);
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
        let mut requires_reset = false;

        let firmware_data = load(self.path())?;
        match Firmware::new(&firmware_data) {
            Some(firmware) => {
                // System76 EC requires reset to load new firmware
                requires_reset = true;
                println!("file board: {:?}", str::from_utf8(firmware.board));
                println!("file version: {:?}", str::from_utf8(firmware.version));
            },
            None => (),
        }

        let result = match &self.ec {
            EcKind::System76(_) => {
                // System76 EC requires reset to load new firmware
                requires_reset = true;

                // Flash backup ROM first
                match unsafe { flash(&firmware_data, SpiTarget::Backup) } {
                    Ok(()) => (),
                    Err(err) => {
                        println!("{} Backup Flash Error: {:X?}", self.name(), err);
                        return Err(Error::DeviceError);
                    }
                }

                // Flash main ROM after ensuring backup ROM is good
                match unsafe { flash(&firmware_data, SpiTarget::Main) } {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        println!("{} Flash Error: {:X?}", self.name(), err);
                        Err(Error::DeviceError)
                    }
                }
            },
            EcKind::Legacy(_) => {
                find(FIRMWARENSH)?;
                let command = if self.master { "ec" } else { "ec2" };
                let status = shell(&format!("{} {} {} flash", FIRMWARENSH, FIRMWAREDIR, command))?;
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
        };

        if requires_reset {
            match find(FIRMWAREDIR) {
                Ok((_, firmware_dir)) => {
                    //Try to create tag file without running shell
                    let filename = wstr(ECTAG);
                    let mut file = 0 as *mut uefi::fs::File;
                    match (firmware_dir.0.Open)(
                        firmware_dir.0,
                        &mut file,
                        filename.as_ptr(),
                        uefi::fs::FILE_MODE_CREATE | uefi::fs::FILE_MODE_READ | uefi::fs::FILE_MODE_WRITE,
                        0
                    ).into_result() {
                        Ok(_) => {
                            unsafe {
                                let _ = ((*file).Close)(&mut *file);
                            }
                            println!("EC tag: created successfully");
                        },
                        Err(err) => {
                            println!("EC tag: failed to create {}: {:?}", ECTAG, err);
                        }
                    }
                },
                Err(err) => {
                    println!("EC tag: failed to find {}: {:?}", FIRMWAREDIR, err);
                }
            }

            println!("System will shut off in 5 seconds");
            let _ = (std::system_table().BootServices.Stall)(5_000_000);

            // Reset EC
            unsafe { watchdog_reset(); }
        }

        result
    }
}
