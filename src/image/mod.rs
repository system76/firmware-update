use core::cell::Cell;
use core::cmp;

use orbclient::{Color, Mode, Renderer};

pub mod bmp;

pub struct ImageRoi<'a> {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    image: &'a Image
}

impl<'a> ImageRoi<'a> {
    /// Draw the ROI on a window
    pub fn draw<R: Renderer>(&self, renderer: &mut R, x: i32, mut y: i32) {
        let stride = self.image.w;
        let mut offset = (self.y * stride + self.x) as usize;
        let last_offset = cmp::min(((self.y + self.h) * stride + self.x) as usize, self.image.data.len());
        while offset < last_offset {
            let next_offset = offset + stride as usize;
            renderer.image_legacy(x, y, self.w, 1, &self.image.data[offset..]);
            offset = next_offset;
            y += 1;
        }
    }
}

#[derive(Clone)]
pub struct Image {
    w: u32,
    h: u32,
    data: Box<[Color]>,
    mode: Cell<Mode>,
}

impl Image {
    /// Create a new image
    pub fn new(width: u32, height: u32) -> Self {
        Self::from_color(width, height, Color::rgb(0, 0, 0))
    }

    /// Create a new image filled whole with color
    pub fn from_color(width: u32, height: u32, color: Color) -> Self {
        Self::from_data(width, height, vec![color; width as usize * height as usize].into_boxed_slice()).unwrap()
    }

    /// Create a new image from a boxed slice of colors
    pub fn from_data(width: u32, height: u32, data: Box<[Color]>) -> Result<Self, String> {
        if (width * height) as usize != data.len() {
            return Err("not enough or too much data given compared to width and height".to_string())
        }

        Ok(Image {
            w: width,
            h: height,
            data: data,
            mode: Cell::new(Mode::Blend),
        })
    }

    /// Create a new empty image
    pub fn default() -> Self {
        Self::new(0, 0)
    }

    /// Get a piece of the image
    pub fn roi<'a>(&'a self, x: u32, y: u32, w: u32, h: u32) -> ImageRoi<'a> {
        let x1 = cmp::min(x, self.width());
        let y1 = cmp::min(y, self.height());
        let x2 = cmp::max(x1, cmp::min(x + w, self.width()));
        let y2 = cmp::max(y1, cmp::min(y + h, self.height()));

        ImageRoi {
            x: x1,
            y: y1,
            w: x2 - x1,
            h: y2 - y1,
            image: self
        }
    }

    /// Return a boxed slice of colors making up the image
    pub fn into_data(self) -> Box<[Color]> {
        self.data
    }

    /// Draw the image on a window
    pub fn draw<R: Renderer>(&self, renderer: &mut R, x: i32, y: i32) {
        renderer.image_legacy(x, y, self.w, self.h, &self.data);
    }
}

impl Renderer for Image {
    /// Get the width of the image in pixels
    fn width(&self) -> u32 {
        self.w
    }

    /// Get the height of the image in pixels
    fn height(&self) -> u32 {
        self.h
    }

    /// Return a reference to a slice of colors making up the image
    fn data(&self) -> &[Color] {
        &self.data
    }

    /// Return a mutable reference to a slice of colors making up the image
    fn data_mut(&mut self) -> &mut [Color] {
        &mut self.data
    }

    fn sync(&mut self) -> bool {
        true
    }

    fn mode(&self) -> &Cell<Mode> {
        &self.mode
    }
}
