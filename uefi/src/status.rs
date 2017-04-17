#[derive(Debug)]
pub enum Error {
    LoadError,
    InvalidParameter,
    Unsupported,
    BadBufferSize,
    BufferTooSmall,
    NotReady,
    DeviceError,
    WriteProtected,
    OutOfResources,
    VolumeCorrupted,
    VolumeFull,
    NoMedia,
    MediaChanged,
    NotFound,
    AccessDenied,
    NoResponse,
    NoMapping,
    Timeout,
    NotStarted,
    AlreadyStarted,
    Aborted,
    IcmpError,
    TftpError,
    ProtocolError,
    IncompatibleVersion,
    SecurityViolation,
    CrcError,
    EndOfMedia,
    EndOfFile,
    InvalidLanguage,
    CompromisedData,
    HttpError,
    Unknown
}

impl Error {
    fn from(value: usize) -> Self {
        use self::Error::*;
        match value {
            1 => LoadError,
            2 => InvalidParameter,
            3 => Unsupported,
            4 => BadBufferSize,
            5 => BufferTooSmall,
            6 => NotReady,
            7 => DeviceError,
            8 => WriteProtected,
            9 => OutOfResources,
            10 => VolumeCorrupted,
            11 => VolumeFull,
            12 => NoMedia,
            13 => MediaChanged,
            14 => NotFound,
            15 => AccessDenied,
            16 => NoResponse,
            17 => NoMapping,
            18 => Timeout,
            19 => NotStarted,
            20 => AlreadyStarted,
            21 => Aborted,
            22 => IcmpError,
            23 => TftpError,
            24 => ProtocolError,
            25 => IncompatibleVersion,
            26 => SecurityViolation,
            27 => CrcError,
            28 => EndOfMedia,
            31 => EndOfFile,
            32 => InvalidLanguage,
            33 => CompromisedData,
            35 => HttpError,
            _ => Unknown
        }
    }
}

#[derive(Debug)]
pub struct Status(usize);

impl Status {
    pub fn new(value: usize) -> Self {
        Status(value)
    }

    pub fn res(&self) -> Result<usize, Error> {
        let max_bit = 1 << 63;
        if self.0 & max_bit == 0 {
            Ok(self.0)
        } else {
            Err(Error::from(self.0 & !(max_bit)))
        }
    }
}
