use crate::{
    params::Position,
    utils::{spherical_to_cartesian, Coords},
};

use atm_refraction::EarthShape;
use nalgebra::Vector3;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Shape {
    Cylinder { radius: f64, height: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Object {
    pub position: Position,
    pub shape: Shape,
    pub color: Color,
}

impl Object {
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn check_collision(
        &self,
        earth_shape: &EarthShape,
        point1: Coords,
        point2: Coords,
    ) -> Option<(f64, Vector3<f64>, Color)> {
        let pos1 = point1.to_cartesian(earth_shape);
        let pos2 = point2.to_cartesian(earth_shape);
        let obj_pos = Coords {
            lat: self.position.latitude,
            lon: self.position.longitude,
            elev: self.position.altitude.unwrap(),
        }
        .to_cartesian(earth_shape);
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
                            let lat = self.position.latitude;
                            let lon = self.position.longitude;
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
                let b = 2.0 * (p1v * wv + p1w - 2.0 * p1v * wv - wv * wv);
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
        }
    }

    pub fn is_close(&self, earth_shape: &EarthShape, sim_step: f64, lat: f64, lon: f64) -> bool {
        match self.shape {
            Shape::Cylinder { radius, .. } => {
                let obj_pos = Coords {
                    lat: self.position.latitude,
                    lon: self.position.longitude,
                    elev: self.position.altitude.unwrap(),
                }
                .to_cartesian(earth_shape);
                let pos = Coords {
                    lat,
                    lon,
                    elev: self.position.altitude.unwrap(),
                }
                .to_cartesian(earth_shape);
                let dist_v = pos - obj_pos;
                dist_v.dot(&dist_v) < 2.0 * (radius + sim_step) * (radius + sim_step)
            }
        }
    }
}
