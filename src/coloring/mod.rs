mod shading;
mod simple;

use crate::rendering::ResultPixel;

use image::Rgb;

pub use self::{shading::Shading, simple::SimpleColors};

pub trait ColoringMethod {
    fn color_for_pixel(&self, pixel: &ResultPixel) -> Rgb<u8>;
}
