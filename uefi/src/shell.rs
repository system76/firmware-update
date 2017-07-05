use super::Handle;

#[repr(packed)]
pub struct ShellParameters {
    Argv: * const * const u16,
    Argc: usize,
    StdIn: Handle,
    StdOut: Handle,
    StdErr: Handle
}
