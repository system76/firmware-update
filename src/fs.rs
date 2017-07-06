use core::{mem, slice};
use uefi::fs::{File as InnerFile, FileInfo, SimpleFileSystem, FILE_MODE_READ};
use uefi::guid::{Guid, EFI_FILE_SYSTEM_GUID};

use proto::Protocol;

pub struct FileSystem(pub &'static mut SimpleFileSystem);

impl Protocol<SimpleFileSystem> for FileSystem {
    fn guid() -> Guid {
        EFI_FILE_SYSTEM_GUID
    }

    fn new(inner: &'static mut SimpleFileSystem) -> Self {
        FileSystem(inner)
    }
}

impl FileSystem {
    pub fn root(&mut self) -> Result<Dir, isize> {
        let mut interface = 0 as *mut InnerFile;
        let status = (self.0.OpenVolume)(self.0, &mut interface);
        if status != 0 {
            return Err(status);
        }

        Ok(Dir(File(unsafe { &mut *interface })))
    }
}

pub struct File(pub &'static mut InnerFile);

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

pub struct Dir(pub File);

impl Dir {
    pub fn open(&mut self, filename: &[u16]) -> Result<File, isize> {
        let mut interface = 0 as *mut InnerFile;
        let status = ((self.0).0.Open)((self.0).0, &mut interface, filename.as_ptr(), FILE_MODE_READ, 0);
        if status != 0 {
            return Err(status);
        }

        Ok(File(unsafe { &mut *interface }))
    }

    pub fn open_dir(&mut self, filename: &[u16]) -> Result<Dir, isize> {
        let file = self.open(filename)?;
        Ok(Dir(file))
    }

    pub fn read(&mut self) -> Result<Option<FileInfo>, isize> {
        let mut info = FileInfo::default();
        let buf = unsafe {
            slice::from_raw_parts_mut(
                &mut info as *mut _ as *mut u8,
                mem::size_of_val(&info)
            )
        };
        match self.0.read(buf) {
            Ok(0) => Ok(None),
            Ok(_len) => Ok(Some(info)),
            Err(err) => Err(err)
        }
    }
}
