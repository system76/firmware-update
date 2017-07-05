#![allow(dead_code)]
#![allow(non_snake_case)]
#![no_std]

pub mod boot;
pub mod config;
pub mod fs;
pub mod graphics;
pub mod guid;
pub mod runtime;
pub mod shell;
pub mod status;
pub mod system;
pub mod text;
pub mod time;

#[derive(Copy, Clone, Debug)]
pub struct Handle(pub usize);

#[repr(C)]
pub struct TableHeader {
    Signature: u64,
    Revision: u32,
    HeaderSize: u32,
    CRC32: u32,
    Reserved: u32
}
