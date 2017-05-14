#[repr(packed)]
pub struct SimpleFileSystem {
    Revision: u64,
    pub OpenVolume: extern "win64" fn (&mut SimpleFileSystem, Root: &mut *mut File),
}

#[repr(packed)]
pub struct File {
    Revision: u64,
    pub Open: extern "win64" fn (&mut File, NewHandle: &mut *mut File, FileName: *const u16, OpenMode: u64, Attributes: u64),
    pub Close: extern "win64" fn (&mut File),
    pub Delete: extern "win64" fn (&mut File),
    pub Read: extern "win64" fn (&mut File, BufferSize: &mut usize, Buffer: *mut u8),
    pub Write: extern "win64" fn (&mut File, BufferSize: &mut usize, Buffer: *const u8),
    pub SetPosition: extern "win64" fn (&mut File, Position: u64),
    pub GetPosition: extern "win64" fn (&mut File, Position: &mut u64),
    GetInfo: extern "win64" fn (),
    SetInfo: extern "win64" fn (),
    pub Flush: extern "win64" fn (&mut File),
}
