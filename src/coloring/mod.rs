mod shading;
mod simple;

use crate::generator::TracePoint;

use image::Rgb;

pub use self::{
    shading::{ColorPalette, Shading},
    simple::SimpleColors,
};

pub trait ColoringMethod {
    fn color_for_pixel(&self, pixel: &TracePoint) -> Rgb<u8>;
    fn sky_color(&self) -> Rgb<u8>;
    fn fog_color(&self) -> Rgb<u8>;
}
