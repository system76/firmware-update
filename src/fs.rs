use uefi::fs::{File as InnerFile, SimpleFileSystem};
use uefi::guid::{Guid, EFI_FILE_SYSTEM_GUID};

use proto::Protocol;

pub struct FileSystem(&'static mut SimpleFileSystem);

impl Protocol<SimpleFileSystem> for FileSystem {
    fn guid() -> Guid {
        EFI_FILE_SYSTEM_GUID
    }

    fn new(inner: &'static mut SimpleFileSystem) -> Self {
        FileSystem(inner)
    }
}

impl FileSystem {
    pub fn root(&mut self) -> Result<File, isize> {
        let mut interface = 0 as *mut InnerFile;
        let status = (self.0.OpenVolume)(self.0, &mut interface);
        if status != 0 {
            return Err(status);
        }

        Ok(File(unsafe { &mut *interface }))
    }
}

pub struct File(&'static mut InnerFile);

impl File {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, isize> {
        let mut len = buf.len();
        let status = (self.0.Read)(self.0, &mut len, buf.as_mut_ptr());
        if status != 0 {
            return Err(status);
        }

        Ok(len)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, isize> {
        let mut len = buf.len();
        let status = (self.0.Write)(self.0, &mut len, buf.as_ptr());
        if status != 0 {
            return Err(status);
        }

        Ok(len)
    }
}

impl Drop for File {
    fn drop(&mut self) {
        (self.0.Close)(self.0);
    }
}
