use super::{Handle, TableHeader};
use guid::Guid;

#[repr(C)]
pub enum MemoryType {
    ///
    /// Not used.
    ///
    EfiReservedMemoryType,
    ///
    /// The code portions of a loaded application.
    /// (Note that UEFI OS loaders are UEFI applications.)
    ///
    EfiLoaderCode,
    ///
    /// The data portions of a loaded application and the default data allocation
    /// type used by an application to allocate pool memory.
    ///
    EfiLoaderData,
    ///
    /// The code portions of a loaded Boot Services Driver.
    ///
    EfiBootServicesCode,
    ///
    /// The data portions of a loaded Boot Serves Driver, and the default data
    /// allocation type used by a Boot Services Driver to allocate pool memory.
    ///
    EfiBootServicesData,
    ///
    /// The code portions of a loaded Runtime Services Driver.
    ///
    EfiRuntimeServicesCode,
    ///
    /// The data portions of a loaded Runtime Services Driver and the default
    /// data allocation type used by a Runtime Services Driver to allocate pool memory.
    ///
    EfiRuntimeServicesData,
    ///
    /// Free (unallocated) memory.
    ///
    EfiConventionalMemory,
    ///
    /// Memory in which errors have been detected.
    ///
    EfiUnusableMemory,
    ///
    /// Memory that holds the ACPI tables.
    ///
    EfiACPIReclaimMemory,
    ///
    /// Address space reserved for use by the firmware.
    ///
    EfiACPIMemoryNVS,
    ///
    /// Used by system firmware to request that a memory-mapped IO region
    /// be mapped by the OS to a virtual address so it can be accessed by EFI runtime services.
    ///
    EfiMemoryMappedIO,
    ///
    /// System memory-mapped IO region that is used to translate memory
    /// cycles to IO cycles by the processor.
    ///
    EfiMemoryMappedIOPortSpace,
    ///
    /// Address space reserved by the firmware for code that is part of the processor.
    ///
    EfiPalCode,
    ///
    /// A memory region that operates as EfiConventionalMemory,
    /// however it happens to also support byte-addressable non-volatility.
    ///
    EfiPersistentMemory,
    EfiMaxMemoryType
}

#[repr(C)]
pub enum LocateSearchType {
    /// Retrieve all the handles in the handle database.
    AllHandles,
    /// Retrieve the next handle fron a RegisterProtocolNotify() event.
    ByRegisterNotify,
    /// Retrieve the set of handles from the handle database that support a specified protocol.
    ByProtocol
}

#[repr(C)]
pub struct BootServices {
    header: TableHeader,
    RaiseTpl: extern "win64" fn(NewTpl: usize) -> usize,
    RestoreTpl: extern "win64" fn(OldTpl: usize),
    AllocatePages: extern "win64" fn(AllocType: usize, MemoryType: MemoryType, Pages: usize, Memory: &mut usize) -> isize,
    FreePages: extern "win64" fn(Memory: usize, Pages: usize) -> isize,
    GetMemoryMap: extern "win64" fn(/* TODO */) -> isize,
    pub AllocatePool: extern "win64" fn(PoolType: MemoryType, Size: usize, Buffer: &mut usize) -> isize,
    pub FreePool: extern "win64" fn(Buffer: usize) -> isize,
    CreateEvent: extern "win64" fn (),
    SetTimer: extern "win64" fn (),
    WaitForEvent: extern "win64" fn (),
    SignalEvent: extern "win64" fn (),
    CloseEvent: extern "win64" fn (),
    CheckEvent: extern "win64" fn (),
    InstallProtocolInterface: extern "win64" fn (),
    ReinstallProtocolInterface: extern "win64" fn (),
    UninstallProtocolInterface: extern "win64" fn (),
    pub HandleProtocol: extern "win64" fn (Handle: Handle, Protocol: &Guid, Interface: &mut usize) -> isize,
    _rsvd: usize,
    RegisterProtocolNotify: extern "win64" fn (),
    pub LocateHandle: extern "win64" fn (SearchType: LocateSearchType, Protocol: &Guid, SearchKey: usize, BufferSize: &mut usize, Buffer: *mut Handle),
    LocateDevicePath: extern "win64" fn (),
    InstallConfigurationTable: extern "win64" fn (),
    LoadImage: extern "win64" fn (),
    StartImage: extern "win64" fn (),
    Exit: extern "win64" fn (),
    UnloadImage: extern "win64" fn (),
    ExitBootServices: extern "win64" fn (),
    GetNextMonotonicCount: extern "win64" fn (),
    Stall: extern "win64" fn (),
    SetWatchdogTimer: extern "win64" fn (),
    ConnectController: extern "win64" fn (),
    DisconnectController: extern "win64" fn (),
    OpenProtocol: extern "win64" fn (),
    CloseProtocol: extern "win64" fn (),
    OpenProtocolInformation: extern "win64" fn (),
    pub ProtocolsPerHandle: extern "win64" fn (Handle: Handle, ProtocolBuffer: *mut Guid, ProtocolBufferCount: usize) -> isize,
    LocateHandleBuffer: extern "win64" fn (SearchType: LocateSearchType, Protocol: &Guid, SearchKey: usize, NoHandles: &mut usize, Buffer: &mut *mut Handle),
    pub LocateProtocol: extern "win64" fn (Protocol: &Guid, Registration: usize, Interface: &mut usize) -> isize,
    InstallMultipleProtocolInterfaces: extern "win64" fn (),
    UninstallMultipleProtocolInterfaces: extern "win64" fn (),
    CalculateCrc32: extern "win64" fn (),
    CopyMem: extern "win64" fn (),
    SetMem: extern "win64" fn (),
    CreateEventEx: extern "win64" fn (),
}
