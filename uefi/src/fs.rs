use core::mem;
use guid::Guid;
use time::Time;

#[repr(packed)]
pub struct SimpleFileSystem {
    Revision: u64,
    pub OpenVolume: extern "win64" fn (&mut SimpleFileSystem, Root: &mut *mut File) -> isize,
}

#[repr(packed)]
pub struct FileInfo {
    pub Size: u64,
    pub FileSize: u64,
    pub PhysicalSize: u64,
    pub CreateTime: Time,
    pub LastAccessTime: Time,
    pub ModificationTime: Time,
    pub Attribute: u64,
    pub FileName: [u16; 256],
}

impl Default for FileInfo {
    fn default() -> Self {
        FileInfo {
            Size: Default::default(),
            FileSize: Default::default(),
            PhysicalSize: Default::default(),
            CreateTime: Default::default(),
            LastAccessTime: Default::default(),
            ModificationTime: Default::default(),
            Attribute: Default::default(),
            FileName: [0; 256],
        }
    }
}

#[repr(packed)]
pub struct File {
    Revision: u64,
    pub Open: extern "win64" fn (&mut File, NewHandle: &mut *mut File, FileName: *const u16, OpenMode: u64, Attributes: u64) -> isize,
    pub Close: extern "win64" fn (&mut File) -> isize,
    pub Delete: extern "win64" fn (&mut File) -> isize,
    pub Read: extern "win64" fn (&mut File, BufferSize: &mut usize, Buffer: *mut u8) -> isize,
    pub Write: extern "win64" fn (&mut File, BufferSize: &mut usize, Buffer: *const u8) -> isize,
    pub SetPosition: extern "win64" fn (&mut File, Position: u64) -> isize,
    pub GetPosition: extern "win64" fn (&mut File, Position: &mut u64) -> isize,
    pub GetInfo: extern "win64" fn (&mut File, InformationType: &Guid, BufferSize: &mut usize, Buffer: *mut u8),
    pub SetInfo: extern "win64" fn (&mut File, InformationType: &Guid, BufferSize: &mut usize, Buffer: *const u8),
    pub Flush: extern "win64" fn (&mut File) -> isize,
}
