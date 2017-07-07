use uefi::guid::{Guid, LOADED_IMAGE_PROTOCOL_GUID};
use uefi::loaded_image::LoadedImage as UefiLoadedImage;

use proto::Protocol;

pub struct LoadedImage(pub &'static mut UefiLoadedImage);

impl Protocol<UefiLoadedImage> for LoadedImage {
    fn guid() -> Guid {
        LOADED_IMAGE_PROTOCOL_GUID
    }

    fn new(inner: &'static mut UefiLoadedImage) -> Self {
        LoadedImage(inner)
    }
}
