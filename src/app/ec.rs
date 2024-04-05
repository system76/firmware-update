// SPDX-License-Identifier: GPL-3.0-only

use core::ops::{ControlFlow, Try};
use ecflash::{Ec, EcFile, EcFlash};
use ectool::{timeout, Access, AccessLpcDirect, Firmware, SecurityState, Spi, SpiRom, SpiTarget, Timeout};
use plain::Plain;
use std::prelude::*;
use std::uefi::{
    self,
    reset::ResetType,
    status::{Error, Result, Status},
};
use std::{
    cell::Cell,
    ffi::wstr,
    fs::{find, load},
    ptr,
    str,
};

use super::{pci_read, shell, Component, EC2ROM, ECROM, ECTAG, FIRMWAREDIR, FIRMWARENSH};

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

pub enum EcKind {
    Pang(ectool::Pmc<UefiTimeout>, String),
    System76(ectool::Ec<AccessLpcDirect<UefiTimeout>>, ectool::Pmc<UefiTimeout>),
    Legacy(EcFlash),
    Unknown,
}

impl EcKind {
    pub unsafe fn new(primary: bool) -> Self {
        // Special case for pang12, pang13, and pang14
        {
            let mut system_version = String::new();

            for table in crate::dmi::dmi() {
                match table.header.kind {
                    1 => {
                        if let Ok(info) = dmi::SystemInfo::from_bytes(&table.data) {
                            let index = info.version;
                            if index > 0 {
                                if let Some(value) = table.strings.get((index - 1) as usize) {
                                    system_version = value.trim().to_string();
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if system_version == "pang12" || system_version == "pang13" || system_version == "pang14" {
                return EcKind::Pang(
                    ectool::Pmc::new(0x62, UefiTimeout::new(100_000)),
                    system_version
                );
            }
        }

        if let Ok(access) = AccessLpcDirect::new(UefiTimeout::new(100_000)) {
            if let Ok(ec) = ectool::Ec::new(access) {
                return EcKind::System76(
                    ec,
                    ectool::Pmc::new(0x62, UefiTimeout::new(100_000))
                );
            }
        }

        if let Ok(ec) = EcFlash::new(primary) {
            return EcKind::Legacy(ec);
        }

        EcKind::Unknown
    }

    pub unsafe fn ac_connected(&mut self) -> bool {
        match self {
            EcKind::Pang(ref mut pmc, _system_version) => {
                let ecwr = pmc.acpi_read(0x80).unwrap_or(0);
                (ecwr & 0x01) == 0x01
            },
            EcKind::System76(_ec, ref mut pmc) => {
                let adp = pmc.acpi_read(0x10).unwrap_or(0);
                (adp & 0x01) == 0x01
            },
            EcKind::Legacy(ref mut ec) => {
                let adp = ec.get_param(0x10).unwrap_or(0);
                (adp & 0x01) == 0x01
            },
            EcKind::Unknown => true,
        }
    }

    unsafe fn model(&mut self) -> String {
        match self {
            EcKind::Pang(_pmc, system_version) => {
                return system_version.clone();
            },
            EcKind::System76(ec, _pmc) => {
                let data_size = ec.access().data_size();
                let mut data = vec![0; data_size];
                if let Ok(count) = ec.board(&mut data) {
                    if let Ok(string) = str::from_utf8(&data[..count]) {
                        return string.to_string();
                    }
                }
            }
            EcKind::Legacy(ec) => {
                return ec.project();
            }
            EcKind::Unknown => (),
        }
        String::new()
    }

    unsafe fn version(&mut self) -> String {
        match self {
            EcKind::Pang(pmc, _system_version) => {
                let mut hms = [0u8; 3];
                for i in 0..hms.len() {
                    match pmc.acpi_read(0x08 + i as u8) {
                        Ok(value) => hms[i] = value,
                        Err(err) => {
                            println!("Failed to read build time: {:?}", err);
                            return String::new();
                        },
                    }
                }

                let mut ymd = [0u8; 3];
                for i in 0..ymd.len() {
                    match pmc.acpi_read(0x0C + i as u8) {
                        Ok(value) => ymd[i] = value,
                        Err(err) => {
                            println!("Failed to read build date: {:?}", err);
                            return String::new();
                        },
                    }
                }

                return format!(
                    "20{:02}/{:02}/{:02}_{:02}:{:02}:{:02}",
                    ymd[0], ymd[1], ymd[2],
                    hms[0], hms[1], hms[2]
                );
            },
            EcKind::System76(ec, _pmc) => {
                let data_size = ec.access().data_size();
                let mut data = vec![0; data_size];
                if let Ok(count) = ec.version(&mut data) {
                    if let Ok(string) = str::from_utf8(&data[..count]) {
                        return string.to_string();
                    }
                }
            }
            EcKind::Legacy(ec) => {
                return ec.version();
            }
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
        // Special case for pang12, pang13, and pang14
        match &self.ec {
            EcKind::Pang(_pmc, _system_version) => {
                // XXX: Get flash size programatically?
                return (data.len() == 128 * 1024 || data.len() == 256 * 1024)
                    && &data[0x50..=0x05F] == b"ITE EC-V14.6   \0";
            }
            _ => (),
        }

        let normalize_model = |model: &str| -> String {
            match model {
                "L140CU" => "system76/lemp9".to_string(),
                "L140MU" => "system76/lemp10".to_string(),
                "L140PU" => "system76/lemp11".to_string(),
                "L140AU" => "system76/lemp12".to_string(),
                "L2x0TU" => "system76/lemp13".to_string(),
                "N130ZU" => "system76/galp3-c".to_string(),
                "N140CU" => "system76/galp4".to_string(),
                "N150ZU" => "system76/darp5".to_string(),
                "N150CU" => "system76/darp6".to_string(),
                "NH50DB" | "NH5xDC" => "system76/gaze15".to_string(),
                "NH5xHX" => "system76/gaze16-3050".to_string(),
                "NH5_7HPQ" => {
                    // If the builtin ethernet at 00:1f.6 is present, this is a -b variant
                    if pci_read(0x00, 0x1f, 0x6, 0x00).unwrap() == 0x15fa8086 {
                        "system76/gaze16-3060-b".to_string()
                    } else {
                        "system76/gaze16-3060".to_string()
                    }
                }
                "NPxxPNJ_K" => "system76/gaze17-3050".to_string(),
                "NPxxPNP" => {
                    // If the builtin ethernet at 00:1f.6 is present, this is a -b variant
                    let pciid = pci_read(0x00, 0x1f, 0x6, 0x00).unwrap();
                    if pciid == 0x1a1e8086 || pciid == 0x1a1f8086 {
                        "system76/gaze17-3060-b".to_string()
                    } else {
                        "system76/gaze17-3060".to_string()
                    }
                }
                "NPxxRNx" => "system76/gaze18".to_string(),
                "NPxxSNx" => "system76/addw3".to_string(),
                "V3x0SNx" => "system76/addw4".to_string(),
                "NS50MU" => "system76/darp7".to_string(),
                "NS50_70PU" => "system76/darp8".to_string(),
                "NS50_70AU" => "system76/darp9".to_string(),
                "V5x0TU" => "system76/darp10".to_string(),
                "NV40Mx" | "NV40Mx-DV" | "NV40MJ" => "system76/galp5".to_string(),
                "NV4xPZ" => "system76/galp6".to_string(),
                "NV40RZ" => "system76/galp7".to_string(),
                "PB50Ex" => "system76/addw1".to_string(),
                "PBx0Dx2" => "system76/addw2".to_string(),
                "P950Ex" => "system76/oryp5".to_string(),
                "PCx0Dx2" => "system76/oryp6".to_string(),
                "PCx0Dx" => "system76/oryp7".to_string(),
                "PCxxHX" => "system76/oryp8".to_string(),
                "PDxxPNx" => {
                    // If the unit uses DDR5, it is oryp10
                    let mem = memory_kind().unwrap_or(0x02);
                    match mem {
                        0x1A => "system76/oryp9".to_string(),
                        0x22 => "system76/oryp10".to_string(),
                        _ => model.to_string(),
                    }
                }
                "PE6xRNx" => "system76/oryp11".to_string(),
                "PE60SNx" => "system76/oryp12".to_string(),
                "PDxxSNx" => "system76/serw13".to_string(),
                "X170SM-G" => "system76/bonw14".to_string(),
                "X370SNx" => "system76/bonw15".to_string(),
                _ => model.to_string(),
            }
        };
        let firmware_model = self.ec.firmware_model(data);
        !self.model.is_empty()
            && !self.version.is_empty()
            && normalize_model(&firmware_model) == normalize_model(&self.model)
    }
}

struct SpiLegacy<T: Timeout> {
    pmc: ectool::Pmc<UefiTimeout>,
    timeout: T,
}

impl<T: Timeout> SpiLegacy<T> {
    unsafe fn new(timeout: T) -> Self {
        Self {
            pmc: ectool::Pmc::new(0x62, UefiTimeout::new(0)),
            timeout,
        }
    }

    fn block_size(&self) -> usize {
        64 * 1024
    }

    fn page_size(&self) -> usize {
        256
    }

    unsafe fn pmc_cmd(&mut self, data: u8) -> core::result::Result<(), ectool::Error> {
        self.timeout.reset();
        timeout!(self.timeout, self.pmc.command(data))
    }

    unsafe fn pmc_read(&mut self) -> core::result::Result<u8, ectool::Error> {
        self.timeout.reset();
        timeout!(self.timeout, self.pmc.read())
    }

    unsafe fn pmc_write(&mut self, data: u8) -> core::result::Result<(), ectool::Error> {
        self.timeout.reset();
        timeout!(self.timeout, self.pmc.write(data))
    }

    unsafe fn scratch(&mut self) -> core::result::Result<u8, ectool::Error> {
        self.pmc_cmd(0xDE)?;
        self.pmc_cmd(0xDC)?;
        self.pmc_cmd(0xF0)?;
        self.pmc_read()
    }

    unsafe fn erase_page(&mut self, page: u16) -> core::result::Result<(), ectool::Error> {
        self.pmc_cmd(0x05)?;
        self.pmc_cmd((page >> 8) as u8)?;
        self.pmc_cmd(page as u8)?;
        self.pmc_cmd(0)?;
        Ok(())
    }

    unsafe fn read(&mut self, data: &mut [u8]) -> core::result::Result<(), ectool::Error> {
        let block_size = self.block_size();
        let blocks = (data.len() + block_size - 1) / block_size;
        for block in 0..blocks {
            self.pmc_cmd(0x03)?;
            self.pmc_cmd(block as u8)?;
            for i in 0..block_size {
                let byte = self.pmc_read()?;
                let addr = block * block_size + i;
                if addr % self.page_size() == 0 {
                    print!("\r{}%", (addr * 100) / (blocks * block_size));
                }
                if addr < data.len() {
                    data[addr] = byte;
                }
            }
        }
        println!("\r100%");
        Ok(())
    }

    unsafe fn write(&mut self, data: &[u8]) -> core::result::Result<(), ectool::Error> {
        let block_size = self.block_size();
        let blocks = (data.len() + block_size - 1) / block_size;
        for block in 0..blocks {
            self.pmc_cmd(0x02)?;
            self.pmc_cmd(0x00)?;
            self.pmc_cmd(block as u8)?;
            self.pmc_cmd(0x00)?;
            self.pmc_cmd(0x00)?;
            for i in 0..block_size {
                let addr = block * block_size + i;
                if addr % self.page_size() == 0 {
                    print!("\r{}%", (addr * 100) / (blocks * block_size));
                }
                let byte = if addr < data.len() { data[addr] } else { 0xFF };
                self.pmc_write(byte)?;
            }
        }
        println!("\r100%");
        Ok(())
    }
}

unsafe fn flash_legacy(firmware_data: &[u8]) -> core::result::Result<(), ectool::Error> {
    let mut spi = SpiLegacy::new(UefiTimeout::new(1_000_000));

    let new_rom = firmware_data.to_vec();

    // XXX: Get flash size programatically?
    let rom_size = new_rom.len();
    if rom_size % 1024 != 0 {
        println!("ROM size of {} is not valid", rom_size);
        return Err(ectool::Error::Verify);
    }

    println!("Entering scratch ROM");
    let _ = spi.scratch()?;

    println!("Erasing ROM");
    let pages = rom_size / spi.page_size();
    for page in 0..pages {
        print!("\r{}%", (page * 100) / pages);
        spi.erase_page(page as u16)?;
    }
    println!("\r100%");

    println!("Verifying ROM erase");
    let mut erased = vec![0; rom_size];
    spi.read(&mut erased)?;
    for (addr, byte) in erased.iter().enumerate() {
        if *byte != 0xFF {
            println!(
                "Failed to erase ROM: {:04X} is {:02X} not {:02X}",
                addr, byte, 0xFF,
            );
            return Err(ectool::Error::Verify);
        }
    }

    println!("Writing ROM");
    spi.write(&new_rom)?;

    println!("Verifying ROM write");
    let mut written = vec![0; rom_size];
    spi.read(&mut written)?;
    for (addr, byte) in written.iter().enumerate() {
        if *byte != written[addr] {
            println!(
                "Failed to write ROM: {:04X} is {:02X} not {:02X}",
                addr, byte, written[addr],
            );
            return Err(ectool::Error::Verify);
        }
    }

    Ok(())
}

pub unsafe fn security_unlock() -> core::result::Result<(), ectool::Error> {
    let access = AccessLpcDirect::new(UefiTimeout::new(100_000))?;
    let mut ec = ectool::Ec::new(access)?;

    match ec.security_get() {
        Ok(state) => match state {
            // If already unlocked, continue
            SecurityState::Unlock => Ok(()),
            // If not unlocked, send the prepare to unlock command and shut off
            _ => {
                ec.security_set(SecurityState::PrepareUnlock)?;

                (std::system_table().RuntimeServices.ResetSystem)(
                    ResetType::Shutdown,
                    Status(0),
                    0,
                    ptr::null()
                );
            }
        },
        Err(err) => match err {
            // Firmware is older than security state support, assume unlocked
            ectool::Error::Protocol(1) => Ok(()),
            // Otherwise return error
            _ => Err(err),
        },
    }
}

unsafe fn flash_read<S: Spi>(
    spi: &mut SpiRom<S, UefiTimeout>,
    rom: &mut [u8],
    sector_size: usize,
) -> core::result::Result<(), ectool::Error> {
    let mut address = 0;
    while address < rom.len() {
        print!("\rSPI Read {}K", address / 1024);
        let next_address = address + sector_size;
        let count = spi.read_at(address as u32, &mut rom[address..next_address])?;
        if count != sector_size {
            println!(
                "\ncount {} did not match sector size {}",
                count, sector_size
            );
            return Err(ectool::Error::Verify);
        }
        address = next_address;
    }
    println!("\rSPI Read {}K", address / 1024);
    Ok(())
}

unsafe fn flash(
    firmware_data: &[u8],
    target: SpiTarget,
) -> core::result::Result<(), ectool::Error> {
    let access = AccessLpcDirect::new(UefiTimeout::new(100_000))?;
    let mut ec = ectool::Ec::new(access)?;
    let data_size = ec.access().data_size();

    println!(
        "Programming EC {} ROM",
        match target {
            SpiTarget::Main => "Main",
            SpiTarget::Backup => "Backup",
        }
    );

    {
        let mut data = vec![0; data_size];
        let size = ec.board(&mut data)?;

        let ec_board = &data[..size];
        println!("ec board: {:?}", str::from_utf8(ec_board));
    }

    {
        let mut data = vec![0; data_size];
        let size = ec.version(&mut data)?;

        let ec_version = &data[..size];
        println!("ec version: {:?}", str::from_utf8(ec_version));
    }

    let new_rom = firmware_data.to_vec();

    // XXX: Get flash size programatically?
    let rom_size = new_rom.len();
    if rom_size % 1024 != 0 {
        println!("ROM size of {} is not valid", rom_size);
        return Err(ectool::Error::Verify);
    }

    let mut spi_bus = ec.spi(SpiTarget::Main, true)?;
    let mut spi = SpiRom::new(&mut spi_bus, UefiTimeout::new(1_000_000));
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

            if !matches {
                if !erased {
                    spi.erase_sector(address as u32)?;
                }
                if !new_erased {
                    let count = spi.write_at(address as u32, &new_rom[address..next_address])?;
                    if count != sector_size {
                        println!(
                            "\nWrite count {} did not match sector size {}",
                            count, sector_size
                        );
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
                println!(
                    "Failed to program: {:X} is {:X} instead of {:X}",
                    i, rom[i], new_rom[i]
                );
                return Err(ectool::Error::Verify);
            }
        }
    }

    println!(
        "Successfully programmed EC {} ROM",
        match target {
            SpiTarget::Main => "Main",
            SpiTarget::Backup => "Backup",
        }
    );

    Ok(())
}

struct I2EC {
    sio: ectool::SuperIo,
}

impl I2EC {
    unsafe fn new() -> Self {
        Self {
            sio: ectool::SuperIo::new(0x2E),
        }
    }

    unsafe fn d2_read(&mut self, addr: u8) -> u8 {
        self.sio.write(0x2E, addr);
        self.sio.read(0x2F)
    }

    unsafe fn d2_write(&mut self, addr: u8, value: u8) {
        self.sio.write(0x2E, addr);
        self.sio.write(0x2F, value);
    }

    unsafe fn read(&mut self, addr: u16) -> u8 {
        self.d2_write(0x11, (addr >> 8) as u8);
        self.d2_write(0x10, addr as u8);
        self.d2_read(0x12)
    }

    unsafe fn write(&mut self, addr: u16, value: u8) {
        self.d2_write(0x11, (addr >> 8) as u8);
        self.d2_write(0x10, addr as u8);
        self.d2_write(0x12, value);
    }
}

unsafe fn watchdog_reset(global: bool) {
    let mut i2ec = I2EC::new();

    let mut rsts = i2ec.read(0x2006);
    if global {
        rsts |= 1 << 2;
    } else {
        rsts &= !(1 << 2);
    }
    i2ec.write(0x2006, rsts);

    let etwcfg = i2ec.read(0x1F01);
    i2ec.write(0x1F01, etwcfg | (1 << 5));
    i2ec.write(0x1F07, 0);
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
        if let Some(firmware) = Firmware::new(&firmware_data) {
            // System76 EC requires reset to load new firmware
            requires_reset = true;
            println!("file board: {:?}", str::from_utf8(firmware.board));
            println!("file version: {:?}", str::from_utf8(firmware.version));
        }

        let result = match &self.ec {
            EcKind::Pang(_pmc, _system_version) => {
                find(FIRMWARENSH)?;
                let command = if self.master { "ec" } else { "ec2" };
                let status = shell(&format!(
                    "{} {} {} flash",
                    FIRMWARENSH, FIRMWAREDIR, command
                ))?;
                if status == 0 {
                    Ok(())
                } else {
                    println!("{} Flash Error: {}", self.name(), status);
                    Err(Error::DeviceError)
                }
            },
            EcKind::System76(_ec, _pmc) => {
                // System76 EC requires reset to load new firmware
                requires_reset = true;

                // Flash main ROM
                match unsafe { flash(&firmware_data, SpiTarget::Main) } {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        println!("{} Flash Error: {:X?}", self.name(), err);
                        Err(Error::DeviceError)
                    }
                }
            }
            EcKind::Legacy(_ec) => {
                requires_reset = true;

                // Use open source flashing code
                match unsafe { flash_legacy(&firmware_data) } {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        println!("{} Flash Error: {:X?}", self.name(), err);
                        Err(Error::DeviceError)
                    }
                }
            }
            EcKind::Unknown => {
                println!("{} Failed to flash EcKind::Unknown", self.name());
                Err(Error::DeviceError)
            }
        };

        if requires_reset {
            match find(FIRMWAREDIR) {
                Ok((_, firmware_dir)) => {
                    //Try to create tag file without running shell
                    let filename = wstr(ECTAG);
                    let mut file = std::ptr::null_mut::<uefi::fs::File>();
                    match (firmware_dir.0.Open)(
                        firmware_dir.0,
                        &mut file,
                        filename.as_ptr(),
                        uefi::fs::FILE_MODE_CREATE
                            | uefi::fs::FILE_MODE_READ
                            | uefi::fs::FILE_MODE_WRITE,
                        0,
                    )
                    .branch()
                    {
                        ControlFlow::Continue(_) => {
                            unsafe {
                                let _ = ((*file).Close)(&mut *file);
                            }
                            println!("EC tag: created successfully");
                        }
                        ControlFlow::Break(err) => {
                            println!("EC tag: failed to create {}: {:?}", ECTAG, err);
                        }
                    }
                }
                Err(err) => {
                    println!("EC tag: failed to find {}: {:?}", FIRMWAREDIR, err);
                }
            }

            println!("System will shut off in 5 seconds");
            let _ = (std::system_table().BootServices.Stall)(5_000_000);

            // Reset EC
            unsafe {
                watchdog_reset(true);
            }
        }

        result
    }
}

fn memory_kind() -> Result<u8> {
    let tables = crate::dmi::dmi();

    for table in tables {
        if table.header.kind != 17 {
            continue;
        }

        if let Ok(info) = dmi::MemoryDevice::from_bytes(&table.data) {
            return Ok(info.memory_kind);
        } else {
            return Err(Error::DeviceError);
        }
    }

    Err(Error::DeviceError)
}
