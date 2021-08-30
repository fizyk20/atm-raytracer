mod fast;
mod utils;

use std::time::SystemTime;

use nalgebra::Vector3;

use crate::{object::Color, params::Params, terrain::Terrain};

pub use fast::FastGenerator;
pub use utils::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResultPixel {
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
    fn generate(
        &self,
        params: &Params,
        terrain: &Terrain,
        start: SystemTime,
    ) -> Vec<Vec<Vec<ResultPixel>>>;
}
