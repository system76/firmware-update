use super::guid::*;

#[repr(C)]
pub struct ConfigurationTable {
    pub VendorGuid: Guid,
    VendorTable: *const ()
}
