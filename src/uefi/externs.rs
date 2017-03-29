#[no_mangle]
pub fn __morestack() {
    // Horrible things will probably happen if this is ever called.
}

#[no_mangle]
pub fn abort() -> ! {
	loop {}
}

#[no_mangle]
pub fn breakpoint() -> ! {
	loop {}
}

/// Memset
///
/// Fill a block of memory with a specified value.
#[no_mangle]
pub unsafe extern fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *((dest as usize + i) as *mut u8) = c as u8;
        i += 1;
    }

    dest
}
