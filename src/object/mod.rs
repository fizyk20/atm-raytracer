mod billboard;
mod frustum;

use std::env;

use crate::{
    generator::params::Position,
    terrain::Terrain,
    utils::{rgba_to_vec4, vec4_to_rgba, Coords, EarthModel},
};

use image::{open, DynamicImage, GenericImageView, Rgba};
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

use billboard::Billboard;
use frustum::Frustum;

#[derive(Clone, Serialize, Deserialize)]
pub enum ConfShape {
    Cylinder {
        radius: f64,
        height: f64,
    },
    Cone {
        radius: f64,
        height: f64,
    },
    Frustum {
        r1: f64,
        r2: f64,
        height: f64,
    },
    Billboard {
        width: f64,
        height: f64,
        texture_path: String,
    },
}

impl ConfShape {
    pub fn into_shape(self) -> Shape {
        match self {
            ConfShape::Cylinder { radius, height } => Shape::Frustum {
                r1: radius,
                r2: radius,
                height,
            },
            ConfShape::Cone { radius, height } => Shape::Frustum {
                r1: radius,
                r2: 0.0,
                height,
            },
            ConfShape::Frustum { r1, r2, height } => Shape::Frustum { r1, r2, height },
            ConfShape::Billboard {
                width,
                height,
                texture_path,
            } => {
                let mut texture_full_path = env::current_dir().unwrap();
                texture_full_path.push(&texture_path);
                let image = open(texture_full_path).unwrap();
                let texture = Image {
                    image,
                    path: texture_path,
                };
                Shape::Billboard {
                    width,
                    height,
                    texture,
                }
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Image {
    #[serde(skip)]
    #[serde(default = "default_image")]
    image: DynamicImage,
    path: String,
}

fn default_image() -> DynamicImage {
    DynamicImage::new_rgba8(0, 0)
}

impl Image {
    #[allow(clippy::many_single_char_names)]
    fn get_pixel(&self, x: f64, y: f64) -> Rgba<u8> {
        let w = self.image.width() as f64;
        let h = self.image.height() as f64;
        let x = x * w - 0.5;
        let x1 = x.floor().clamp(0.0, w - 2.0);
        let x2 = x1 + 1.0;
        let (ix1, ix2) = (x1 as u32, x2 as u32);
        let y = (1.0 - y) * h - 0.5;
        let y1 = y.floor().clamp(0.0, h - 2.0);
        let y2 = y1 + 1.0;
        let (iy1, iy2) = (y1 as u32, y2 as u32);

        let px = x - x1;
        let py = y - y1;

        let pix00 = rgba_to_vec4(self.image.get_pixel(ix1, iy1));
        let pix01 = rgba_to_vec4(self.image.get_pixel(ix1, iy2));
        let pix10 = rgba_to_vec4(self.image.get_pixel(ix2, iy1));
        let pix11 = rgba_to_vec4(self.image.get_pixel(ix2, iy2));

        vec4_to_rgba(
            pix00 * (1.0 - px) * (1.0 - py)
                + pix01 * (1.0 - px) * py
                + pix10 * px * (1.0 - py)
                + pix11 * px * py,
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Shape {
    Frustum {
        r1: f64,
        r2: f64,
        height: f64,
    },
    Billboard {
        width: f64,
        height: f64,
        texture: Image,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    #[serde(default = "default_alpha")]
    pub a: f64,
}

fn default_alpha() -> f64 {
    1.0
}

impl Color {
    pub fn interpolate(self, other: Color, coeff: f64) -> Color {
        Color {
            r: self.r * (1.0 - coeff) + other.r * coeff,
            g: self.g * (1.0 - coeff) + other.g * coeff,
            b: self.b * (1.0 - coeff) + other.b * coeff,
            a: self.a * (1.0 - coeff) + other.a * coeff,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfObject {
    position: Position,
    shape: ConfShape,
    color: Color,
}

impl ConfObject {
    pub fn into_serializable_object(self, terrain: &Terrain) -> SerializableObject {
        let position = Coords {
            lat: self.position.latitude,
            lon: self.position.longitude,
            elev: self.position.altitude.abs(
                terrain,
                self.position.latitude,
                self.position.longitude,
            ),
        };
        let shape = self.shape.into_shape();

        SerializableObject {
            position,
            shape,
            color: self.color,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableObject {
    position: Coords,
    shape: Shape,
    color: Color,
}

impl SerializableObject {
    pub fn into_object(&self) -> Box<dyn Object + Sync> {
        match self.shape {
            Shape::Frustum { r1, r2, height } => Box::new(Frustum {
                r1,
                r2,
                height,
                position: self.position,
                color: self.color,
            }),
            Shape::Billboard {
                width,
                height,
                ref texture,
            } => Box::new(Billboard {
                width,
                height,
                texture: texture.clone(),
                position: self.position,
            }),
        }
    }
}

pub trait Object {
    fn check_collision(
        &self,
        earth_model: &EarthModel,
        point1: Coords,
        point2: Coords,
    ) -> Vec<(f64, Vector3<f64>, Color)>;

    fn is_close(&self, earth_model: &EarthModel, sim_step: f64, lat: f64, lon: f64) -> bool;
}
