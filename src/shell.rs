use uefi::guid::{Guid, SHELL_GUID};
use uefi::shell::Shell as UefiShell;

use proto::Protocol;

pub struct Shell(pub &'static mut UefiShell);

impl Protocol<UefiShell> for Shell {
    fn guid() -> Guid {
        SHELL_GUID
    }

    fn new(inner: &'static mut UefiShell) -> Self {
        Shell(inner)
    }
}
