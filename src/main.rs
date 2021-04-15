// SPDX-License-Identifier: GPL-3.0-only

#![no_std]
#![no_main]
#![feature(llvm_asm)]
#![feature(const_fn)]
#![feature(core_intrinsics)]
#![feature(prelude_import)]
#![feature(try_trait)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::many_single_char_names)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::transmute_ptr_to_ptr)]

extern crate alloc;
extern crate rlibc;
extern crate system76_firmware_smmstore as smmstore;
#[macro_use]
extern crate uefi_std as std;

#[allow(unused_imports)]
#[prelude_import]
use std::prelude::*;

use core::ops::Try;
use core::ptr;
use uefi::reset::ResetType;
use uefi::status::{Result, Status};

mod app;
mod display;
mod dmi;
pub mod image;
mod io;
mod key;
pub mod null;
pub mod text;

fn set_max_mode(output: &uefi::text::TextOutput) -> Result<()> {
    let mut max_i = None;
    let mut max_w = 0;
    let mut max_h = 0;

    for i in 0..output.Mode.MaxMode as usize {
        let mut w = 0;
        let mut h = 0;
        if (output.QueryMode)(output, i, &mut w, &mut h).into_result().is_ok() {
            if w >= max_w && h >= max_h {
                max_i = Some(i);
                max_w = w;
                max_h = h;
            }
        }
    }

    if let Some(i) = max_i {
        (output.SetMode)(output, i)?;
    }

    Ok(())
}

#[no_mangle]
pub extern "C" fn main() -> Status {
    let uefi = std::system_table();

    let _ = (uefi.BootServices.SetWatchdogTimer)(0, 0, 0, ptr::null());

    if let Err(err) = set_max_mode(uefi.ConsoleOut).into_result() {
        println!("Failed to set max mode: {:?}", err);
    }

    let _ = (uefi.ConsoleOut.SetAttribute)(uefi.ConsoleOut, 0x0F);

    if let Err(err) = app::main() {
        println!("App error: {:?}", err);
        let _ = io::wait_key();
    }

    (uefi.RuntimeServices.ResetSystem)(ResetType::Cold, Status(0), 0, ptr::null());
}
