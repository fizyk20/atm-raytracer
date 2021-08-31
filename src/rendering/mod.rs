mod fast;
mod rectilinear;
mod utils;

use nalgebra::Vector3;

use crate::object::Color;

pub use fast::FastGenerator;
pub use rectilinear::RectilinearGenerator;
pub use utils::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultPixel {
    pub elevation_angle: f64,
    pub azimuth: f64,
    pub trace_points: Vec<TracePoint>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TracePoint {
    pub lat: f64,
    pub lon: f64,
    pub distance: f64,
    pub elevation: f64,
    pub path_length: f64,
    pub normal: Vector3<f64>,
    pub color: PixelColor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PixelColor {
    Terrain,
    Rgba(Color),
}

impl PixelColor {
    pub fn alpha(&self) -> f64 {
        match self {
            PixelColor::Rgba(color) => color.a,
            PixelColor::Terrain => 1.0,
        }
    }
}

pub trait Generator {
    fn generate(&self) -> Vec<Vec<ResultPixel>>;
}
