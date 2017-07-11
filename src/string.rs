use alloc::{String, Vec};
use core::char;

pub fn wstr(string: &str) -> Vec<u16> {
    let mut wstring = vec![];

    for c in string.chars() {
        wstring.push(c as u16);
    }
    wstring.push(0);

    wstring
}

pub fn nstr(wstring: *const u16) -> String {
    let mut string = String::new();

    let mut i = 0;
    loop {
        let w = unsafe { *wstring.offset(i) };
        i += 1;
        if w == 0 {
            break;
        }
        let c = unsafe { char::from_u32_unchecked(w as u32) };
        string.push(c);
    }

    string
}
