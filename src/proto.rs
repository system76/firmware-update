use collections::Vec;
use core::mem;
use uefi::Handle;
use uefi::boot::LocateSearchType;
use uefi::guid::Guid;

pub trait Protocol<T: 'static> {
    fn guid() -> Guid;

    fn new(fs: &'static mut T) -> Self where Self: Sized ;

    fn locate_protocol() -> Result<Self, isize> where Self: Sized {
        let uefi = unsafe { &mut *::UEFI };

        let guid = Self::guid();
        let mut interface = 0;
        let status = (uefi.BootServices.LocateProtocol)(&guid, 0, &mut interface);
        if status != 0 {
            return Err(status);
        }

        Ok(Self::new(unsafe { &mut *(interface as *mut T) }))
    }

    fn handle_protocol(handle: Handle) -> Result<Self, isize> where Self: Sized {
        let uefi = unsafe { &mut *::UEFI };

        let guid = Self::guid();
        let mut interface = 0;
        let status = (uefi.BootServices.HandleProtocol)(handle, &guid, &mut interface);
        if status != 0 {
            return Err(status);
        }

        Ok(Self::new(unsafe { &mut *(interface as *mut T) }))
    }

    fn locate_handle() -> Vec<Self> where Self: Sized {
        let uefi = unsafe { &mut *::UEFI };

        let guid = Self::guid();
        let mut handles = Vec::with_capacity(256);
        let mut len = handles.capacity() * mem::size_of::<Handle>();
        (uefi.BootServices.LocateHandle)(LocateSearchType::ByProtocol, &guid, 0, &mut len, handles.as_mut_ptr());
        unsafe { handles.set_len(len / mem::size_of::<Handle>()); }

        let mut instances = Vec::new();
        for handle in handles {
            if let Ok(instance) = Self::handle_protocol(handle) {
                instances.push(instance);
            }
        }
        instances
    }

    fn one() -> Result<Self, isize> where Self: Sized {
        Self::locate_protocol()
    }

    fn all() -> Vec<Self> where Self: Sized {
        Self::locate_handle()
    }
}
