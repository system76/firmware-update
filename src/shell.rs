use uefi::Handle;
use uefi::guid::{Guid, SHELL_GUID};
use uefi::shell::Shell as UefiShell;
use uefi::status::Result;

use fs::load;
use loaded_image::LoadedImage;
use proto::Protocol;
use string::wstr;

pub fn shell(cmd: &str) -> Result<usize> {
    let handle = unsafe { ::HANDLE };
    let uefi = unsafe { &mut *::UEFI };

    let args = [
        "res\\shell.efi",
        "-nointerrupt",
        "-nomap",
        "-nostartup",
        "-noversion",
        cmd
    ];

    let mut cmdline = format!("\"{}\"", args[0]);
    for arg in args.iter().skip(1) {
        cmdline.push_str(" \"");
        cmdline.push_str(arg);
        cmdline.push_str("\"");
    }

    let wcmdline = wstr(&cmdline);

    let data = load(args[0])?;

    let mut shell_handle = Handle(0);
    (uefi.BootServices.LoadImage)(false, handle, 0, data.as_ptr(), data.len(), &mut shell_handle)?;

    if let Ok(loaded_image) = LoadedImage::handle_protocol(shell_handle) {
        loaded_image.0.LoadOptionsSize = (wcmdline.len() as u32) * 2;
        loaded_image.0.LoadOptions = wcmdline.as_ptr();
    }

    let mut exit_size = 0;
    let mut exit_ptr = ::core::ptr::null_mut();
    let ret = (uefi.BootServices.StartImage)(shell_handle, &mut exit_size, &mut exit_ptr)?;

    Ok(ret)
}

pub struct Shell(pub &'static mut UefiShell);

impl Protocol<UefiShell> for Shell {
    fn guid() -> Guid {
        SHELL_GUID
    }

    fn new(inner: &'static mut UefiShell) -> Self {
        Shell(inner)
    }
}
