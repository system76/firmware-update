use uefi::guid::{Guid, SIMPLE_POINTER_GUID};
use uefi::pointer::SimplePointer;

use proto::Protocol;

pub struct Pointer(pub &'static mut SimplePointer);

impl Protocol<SimplePointer> for Pointer {
    fn guid() -> Guid {
        SIMPLE_POINTER_GUID
    }

    fn new(inner: &'static mut SimplePointer) -> Self {
        Pointer(inner)
    }
}
