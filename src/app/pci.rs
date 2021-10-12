use hwio::{Io, Pio};

pub fn pci_read(bus: u8, dev: u8, func: u8, offset: u8) -> Result<u32, String> {
    if dev > 0x1f {
        return Err(format!("pci_read dev 0x{:x} is greater than 0x1f", dev));
    }

    if func > 0x7 {
        return Err(format!("pci_read func 0x{:x} is greater than 0x7", func));
    }

    let address = 0x80000000 |
        (u32::from(bus) << 16) |
        (u32::from(dev) << 11) |
        (u32::from(func) << 8) |
        u32::from(offset);
    Pio::<u32>::new(0xCF8).write(address);
    Ok(Pio::<u32>::new(0xCFC).read())
}
