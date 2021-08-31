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

impl TracePoint {
    pub fn interpolate(&self, other: &TracePoint, coeff: f64) -> Self {
        Self {
            lat: self.lat * (1.0 - coeff) + other.lat * coeff,
            lon: self.lon * (1.0 - coeff) + other.lon * coeff,
            distance: self.distance * (1.0 - coeff) + other.distance * coeff,
            elevation: self.elevation * (1.0 - coeff) + other.elevation * coeff,
            path_length: self.path_length * (1.0 - coeff) + other.path_length * coeff,
            normal: self.normal * (1.0 - coeff) + other.normal * coeff,
            color: self.color.interpolate(&other.color, coeff),
        }
    }
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

    pub fn same_class(&self, other: &PixelColor) -> bool {
        match (self, other) {
            (PixelColor::Terrain, PixelColor::Terrain) => true,
            (PixelColor::Rgba(_), PixelColor::Rgba(_)) => true,
            _ => false,
        }
    }

    pub fn interpolate(&self, other: &PixelColor, coeff: f64) -> PixelColor {
        match (self, other) {
            (PixelColor::Terrain, PixelColor::Terrain) => PixelColor::Terrain,
            (PixelColor::Rgba(color1), PixelColor::Rgba(color2)) => {
                PixelColor::Rgba(color1.interpolate(*color2, coeff))
            }
            _ => PixelColor::Terrain,
        }
    }
}

pub trait Generator {
    fn generate(&self) -> Vec<Vec<ResultPixel>>;
}
