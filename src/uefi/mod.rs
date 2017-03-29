use core::ops::Deref;

pub mod externs;

pub struct Guid(u32, u16, u16, [u8; 8]);

pub struct Handle<T>(*const T);

impl<T> Deref for Handle<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

struct TableHeader {
    Signature: u64,
    Revision: u32,
    HeaderSize: u32,
    CRC32: u32,
    Reserved: u32
}

pub struct SystemTable {
    Hdr: TableHeader,
    FirmwareVendor: Handle<u16>,
    FirmwareRevision: u32,
    ConsoleInHandle: Handle<()>,
    ConIn: Handle<TextInput>,
    ConsoleOutHandle: Handle<()>,
    pub ConOut: Handle<TextOutput>,
    ConsoleErrorHandle: Handle<()>,
    StdErr: Handle<TextOutput>,
    RuntimeServices: Handle<RuntimeServices>,
    BootServices: Handle<BootServices>,
    NumberOfTableEntries: usize,
    ConfigurationTable: Handle<ConfigurationTable>
}

pub struct TextInput;

pub struct TextOutput {
    Reset: Handle<()>,
    OutputString: extern "win64" fn(*const TextOutput, *const u16),
    // ... and more stuff that we're ignoring.
}

impl TextOutput {
    pub fn write(&self, string: &str) {
        let mut buf = [0u16; 4096];
        let mut i = 0;

        for v in string.chars() {
            if i >= buf.len() {
                break;
            }
            buf[i] = v as u16; // TODO: won't work with non-BMP
            i += 1;
        }

        *buf.last_mut().unwrap() = 0;

        (self.OutputString)(self, buf.as_ptr());
    }
}

struct RuntimeServices;

struct BootServices;

struct ConfigurationTable {
    VendorGuid: Guid,
    VendorTable: Handle<()>
}

#[no_mangle]
pub extern "win64" fn efi_start(_ImageHandle: Handle<()>, sys_table: Handle<SystemTable>) -> isize {
    ::efi_main(sys_table);
    0
}
