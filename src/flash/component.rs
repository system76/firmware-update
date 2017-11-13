use uefi::status::Result;

pub trait Component {
    fn name(&self) -> &str;
    fn path(&self) -> &str;
    fn validate(&self) -> Result<bool>;
    fn flash(&self) -> Result<()>;
}

