#[lang = "eh_personality"]
pub extern "C" fn eh_personality() {}

/// Required to handle panics
#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn panic_fmt(fmt: ::core::fmt::Arguments, file: &str, line: u32) -> ! {
    loop {}
}

#[allow(non_snake_case)]
#[no_mangle]
/// Required to handle panics
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {}
}
