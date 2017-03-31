use super::guid::Guid;

#[repr(C)]
pub struct ConfigurationTable {
    VendorGuid: Guid,
    VendorTable: *const ()
}
