use core::slice;

use super::{Handle, TableHeader};
use super::boot::BootServices;
use super::config::ConfigurationTable;
use super::runtime::RuntimeServices;
use super::text::{TextInput, TextOutput};

#[repr(C)]
pub struct SystemTable {
    header: TableHeader,
    vendor: *const u16,
    revision: u32,
    pub ConsoleInHandle: Handle,
    pub ConsoleIn: &'static mut TextInput,
    pub ConsoleOutHandle: Handle,
    pub ConsoleOut: &'static mut TextOutput,
    pub ConsoleErrorHandle: Handle,
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
