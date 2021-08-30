use std::env;

use crate::{
    params::Position,
    terrain::Terrain,
    utils::{rgba_to_vec4, spherical_to_cartesian, vec4_to_rgba, Coords},
};

use atm_refraction::EarthShape;
use image::{open, DynamicImage, GenericImageView, Rgba};
use nalgebra::Vector3;

#[derive(Clone, Serialize, Deserialize)]
pub enum ConfShape {
    Cylinder {
        radius: f64,
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
            ConfShape::Cylinder { radius, height } => Shape::Cylinder { radius, height },
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
    Cylinder {
        radius: f64,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfObject {
    position: Position,
    shape: ConfShape,
    color: Color,
}

impl ConfObject {
    pub fn into_object(self, terrain: &Terrain) -> Object {
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
        Object {
            position,
            shape,
            color: self.color,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Object {
    pub position: Coords,
    pub shape: Shape,
    pub color: Color,
}

impl Object {
    #[allow(clippy::many_single_char_names)]
    pub fn check_collision(
        &self,
        earth_shape: &EarthShape,
        point1: Coords,
        point2: Coords,
    ) -> Option<(f64, Vector3<f64>, Color)> {
        let pos1 = point1.to_cartesian(earth_shape);
        let pos2 = point2.to_cartesian(earth_shape);
        let obj_pos = self.position.to_cartesian(earth_shape);
        match self.shape {
            Shape::Cylinder { radius, height } => {
                let p1 = pos1 - obj_pos;
                let p1sq = p1.dot(&p1);

                if p1sq > 2.0 * (radius * radius + height * height) {
                    return None;
                }

                let v = {
                    match earth_shape {
                        EarthShape::Spherical { .. } => {
                            let lat = self.position.lat;
                            let lon = self.position.lon;
                            spherical_to_cartesian(1.0, lat, lon)
                        }
                        EarthShape::Flat => Vector3::new(0.0, 0.0, 1.0),
                    }
                };

                let w = pos2 - pos1;

                let wsq = w.dot(&w);
                let p1v = p1.dot(&v);
                let p1w = p1.dot(&w);
                let wv = w.dot(&v);

                let a = wsq - wv * wv;
                let b = 2.0 * (p1w - p1v * wv);
                let c = p1sq - p1v * p1v - radius * radius;

                let delta = b * b - 4.0 * a * c;

                if delta < 0.0 {
                    None
                } else {
                    let x1 = (-b - delta.sqrt()) / 2.0 / a;
                    let x2 = (-b + delta.sqrt()) / 2.0 / a;

                    let x = if x1 < x2 { x1 } else { x2 };

                    if !(0.0..1.0).contains(&x) {
                        return None;
                    }

                    let intersection = p1 + w * x;

                    let h = intersection.dot(&v);

                    if !(0.0..height).contains(&h) {
                        return None;
                    }

                    let normal = intersection - h * v;

                    let n_len = normal.dot(&normal).sqrt();
                    let normal = normal / n_len;

                    Some((x, normal, self.color))
                }
            }
            Shape::Billboard {
                width,
                height,
                ref texture,
            } => {
                let ray = pos2 - pos1;
                let up = {
                    match earth_shape {
                        EarthShape::Spherical { .. } => {
                            let lat = self.position.lat;
                            let lon = self.position.lon;
                            spherical_to_cartesian(1.0, lat, lon)
                        }
                        EarthShape::Flat => Vector3::new(0.0, 0.0, 1.0),
                    }
                };
                let right = ray.cross(&up);
                let right_len = right.dot(&right).sqrt();
                let right = right / right_len;
                let front = right.cross(&up);

                let p1 = pos1 - obj_pos;

                let prop = -p1.dot(&front) / ray.dot(&front);

                if !(0.0..1.0).contains(&prop) {
                    // intersection outside of the current interval
                    return None;
                }

                let intersection = p1 + ray * prop;
                let y = intersection.dot(&up);
                let x = intersection.dot(&right);

                if !(0.0..height).contains(&y) || !(-width / 2.0..width / 2.0).contains(&x) {
                    // intersection outside of the rectangle
                    return None;
                }

                let x = (x + width / 2.0) / width;
                let y = y / height;
                let pixel = texture.get_pixel(x, y);

                let color = Color {
                    r: pixel.0[0] as f64 / 255.0,
                    g: pixel.0[1] as f64 / 255.0,
                    b: pixel.0[2] as f64 / 255.0,
                    a: pixel.0[3] as f64 / 255.0,
                };

                Some((prop, front, color))
            }
        }
    }

    pub fn is_close(&self, earth_shape: &EarthShape, sim_step: f64, lat: f64, lon: f64) -> bool {
        let obj_pos = self.position.to_cartesian(earth_shape);
        let pos = Coords {
            lat,
            lon,
            elev: self.position.elev,
        }
        .to_cartesian(earth_shape);
        let dist_v = pos - obj_pos;

        match self.shape {
            Shape::Cylinder { radius, .. } => {
                dist_v.dot(&dist_v) < 2.0 * (radius + sim_step) * (radius + sim_step)
            }
            Shape::Billboard { width, .. } => {
                dist_v.dot(&dist_v) < 2.0 * (width + sim_step) * (width + sim_step)
            }
        }
    }
}
