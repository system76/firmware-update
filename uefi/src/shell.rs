use super::Handle;

#[repr(packed)]
pub struct ShellParameters {
    pub Argv: * const * const u16,
    pub Argc: usize,
    pub StdIn: Handle,
    pub StdOut: Handle,
    pub StdErr: Handle
}
