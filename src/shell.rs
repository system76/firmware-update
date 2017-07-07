use uefi::guid::{Guid, EFI_SHELL_GUID};
use uefi::shell::Shell as UefiShell;

use proto::Protocol;

pub struct Shell(pub &'static mut UefiShell);

impl Protocol<UefiShell> for Shell {
    fn guid() -> Guid {
        EFI_SHELL_GUID
    }

    fn new(inner: &'static mut UefiShell) -> Self {
        Shell(inner)
    }
}
