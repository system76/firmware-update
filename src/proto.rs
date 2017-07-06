use alloc::Vec;
use core::mem;
use uefi::Handle;
use uefi::boot::LocateSearchType;
use uefi::guid::Guid;
use uefi::status::Result;

pub trait Protocol<T: 'static> {
    fn guid() -> Guid;

    fn new(fs: &'static mut T) -> Self where Self: Sized;

    fn locate_protocol() -> Result<Self> where Self: Sized {
        let uefi = unsafe { &mut *::UEFI };

        let guid = Self::guid();
        let mut interface = 0;
        (uefi.BootServices.LocateProtocol)(&guid, 0, &mut interface)?;

        Ok(Self::new(unsafe { &mut *(interface as *mut T) }))
    }

    fn handle_protocol(handle: Handle) -> Result<Self> where Self: Sized {
        let uefi = unsafe { &mut *::UEFI };

        let guid = Self::guid();
        let mut interface = 0;
        (uefi.BootServices.HandleProtocol)(handle, &guid, &mut interface)?;

        Ok(Self::new(unsafe { &mut *(interface as *mut T) }))
    }

    fn locate_handle() -> Result<Vec<Self>> where Self: Sized {
        let uefi = unsafe { &mut *::UEFI };

        let guid = Self::guid();
        let mut handles = Vec::with_capacity(256);
        let mut len = handles.capacity() * mem::size_of::<Handle>();
        (uefi.BootServices.LocateHandle)(LocateSearchType::ByProtocol, &guid, 0, &mut len, handles.as_mut_ptr())?;
        unsafe { handles.set_len(len / mem::size_of::<Handle>()); }

        let mut instances = Vec::new();
        for handle in handles {
            if let Ok(instance) = Self::handle_protocol(handle) {
                instances.push(instance);
            }
        }
        Ok(instances)
    }

    fn one() -> Result<Self> where Self: Sized {
        Self::locate_protocol()
    }

    fn all() -> Vec<Self> where Self: Sized {
        Self::locate_handle().unwrap_or(Vec::new())
    }
}
