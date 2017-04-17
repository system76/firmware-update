#![feature(allocator)]
#![feature(const_fn)]

#![allocator]
#![no_std]

extern crate uefi;

use uefi::boot::MemoryType;
use uefi::system::SystemTable;

static mut UEFI: *mut SystemTable = 0 as *mut SystemTable;

pub unsafe fn init(table: &'static mut SystemTable) {
    UEFI = table;
}

fn get_uefi() -> Option<&'static mut SystemTable> {
    unsafe {
        if UEFI as usize == 0 {
            None
        } else {
            Some(&mut *UEFI)
        }
    }
}

#[no_mangle]
pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    if let Some(ref mut uefi) = get_uefi() {
        let mut ptr = 0;
        let ret = (uefi.BootServices.AllocatePool)(MemoryType::EfiLoaderData, size, &mut ptr);
        if ret != 0 {
            panic!("__rust_allocate: uefi returned {:X}", ret);
        }
        ptr as *mut u8
    } else {
        panic!("__rust_allocate: uefi not initialized");
    }
}

#[no_mangle]
pub extern fn __rust_deallocate(ptr: *mut u8, size: usize, align: usize) {
    if let Some(ref mut uefi) = get_uefi() {
        (uefi.BootServices.FreePool)(ptr as usize);
    } else {
        panic!("__rust_deallocate: uefi not initialized");
    }
}

#[no_mangle]
pub extern fn __rust_usable_size(size: usize, _align: usize) -> usize {
    size
}

#[no_mangle]
pub extern fn __rust_reallocate_inplace(_ptr: *mut u8, size: usize,
    _new_size: usize, _align: usize) -> usize
{
    size
}

#[no_mangle]
pub extern fn __rust_reallocate(ptr: *mut u8, size: usize, new_size: usize,
                                align: usize) -> *mut u8 {
    use core::{ptr, cmp};

    // from: https://github.com/rust-lang/rust/blob/
    //     c66d2380a810c9a2b3dbb4f93a830b101ee49cc2/
    //     src/liballoc_system/lib.rs#L98-L101

    let new_ptr = __rust_allocate(new_size, align);
    unsafe { ptr::copy(ptr, new_ptr, cmp::min(size, new_size)) };
    __rust_deallocate(ptr, size, align);
    new_ptr
}
