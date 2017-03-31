use self::system::SystemTable;

pub mod boot;
pub mod config;
pub mod externs;
pub mod guid;
pub mod panic;
pub mod runtime;
pub mod system;
pub mod text;

#[no_mangle]
pub extern "win64" fn _start(_ImageHandle: *const (), sys_table: &mut SystemTable) -> isize {
    ::main(sys_table);
    0
}
