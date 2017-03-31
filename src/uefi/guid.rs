use core::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Guid(pub u32, pub u16, pub u16, pub [u8; 8]);

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:>08x}, {:>04x}, {:>04x}, [", self.0, self.1, self.2)?;
        for (i, b) in self.3.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{:>02x}", b)?;
        }
        write!(f, "])")?;
        Ok(())
    }
}
