use core::slice;

use super::boot::BootServices;
use super::config::ConfigurationTable;
use super::runtime::RuntimeServices;
use super::text::{TextInput, TextOutput};

#[repr(C)]
struct TableHeader {
    Signature: u64,
    Revision: u32,
    HeaderSize: u32,
    CRC32: u32,
    Reserved: u32
}

#[repr(C)]
pub struct SystemTable {
    header: TableHeader,
    vendor: *const u16,
    revision: u32,
    ConsoleInHandle: * const (),
    pub ConsoleIn: &'static mut TextInput,
    ConsoleOutHandle: * const (),
    pub ConsoleOut: &'static mut TextOutput,
    ConsoleErrorHandle: * const (),
    pub ConsoleError: &'static mut TextOutput,
    pub RuntimeServices: &'static mut RuntimeServices,
    pub BootServices: &'static mut BootServices,
    Entries: usize,
    ConfigurationTables: *const ConfigurationTable
}

impl SystemTable {
    pub fn config_tables(&self) -> &'static [ConfigurationTable] {
        unsafe {
            slice::from_raw_parts(self.ConfigurationTables, self.Entries)
        }
    }
}
