use uefi::Handle;
use uefi::status::Result;

use fs::load;
use loaded_image::LoadedImage;
use proto::Protocol;
use string::wstr;

pub fn exec_data(data: &[u8], name: &str, args: &[&str]) -> Result<usize> {
    let handle = unsafe { ::HANDLE };
    let uefi = unsafe { &mut *::UEFI };

    let mut image_handle = Handle(0);
    (uefi.BootServices.LoadImage)(false, handle, 0, data.as_ptr(), data.len(), &mut image_handle)?;

    let mut cmdline = format!("\"{}\"", name);
    for arg in args.iter() {
        cmdline.push_str(" \"");
        cmdline.push_str(arg);
        cmdline.push_str("\"");
    }
    cmdline.push('\0');

    let wcmdline = wstr(&cmdline);

    if let Ok(loaded_image) = LoadedImage::handle_protocol(image_handle) {
        loaded_image.0.LoadOptionsSize = (wcmdline.len() as u32) * 2;
        loaded_image.0.LoadOptions = wcmdline.as_ptr();
    }

    let mut exit_size = 0;
    let mut exit_ptr = ::core::ptr::null_mut();
    let ret = (uefi.BootServices.StartImage)(image_handle, &mut exit_size, &mut exit_ptr)?;

    Ok(ret)
}

pub fn exec_path(path: &str, args: &[&str]) -> Result<usize> {
    let data = load(path)?;
    exec_data(&data, path, args)
}
