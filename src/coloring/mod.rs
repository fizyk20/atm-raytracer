mod shading;
mod simple;

use crate::generator::TracePoint;

use image::Rgb;

pub use self::{shading::Shading, simple::SimpleColors};

pub trait ColoringMethod {
    fn color_for_pixel(&self, pixel: &TracePoint) -> Rgb<u8>;
}
