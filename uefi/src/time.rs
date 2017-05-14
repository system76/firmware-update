#[derive(Default)]
#[repr(packed)]
pub struct Time {
    pub Year: u16,
    pub Month: u8,
    pub Day: u8,
    pub Hour: u8,
    pub Minute: u8,
    pub Second: u8,
    _Pad1: u8,
    pub Nanosecond: u32,
    pub TimeZone: u16,
    pub Daylight: u8,
    _Pad2: u8
}
