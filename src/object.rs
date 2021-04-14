use crate::params::Position;

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
    #[allow(clippy::too_many_arguments)]
    pub fn check_collision(
        &self,
        earth_shape: &EarthShape,
        lat1: f64,
        lat2: f64,
        lon1: f64,
        lon2: f64,
        elev1: f64,
        elev2: f64,
    ) -> Option<(f64, Vector3<f64>, Color)> {
        let pos1 = pos_to_3d(earth_shape, lat1, lon1, elev1);
        let pos2 = pos_to_3d(earth_shape, lat2, lon2, elev2);
        let obj_pos = pos_to_3d(
            earth_shape,
            self.position.latitude,
            self.position.longitude,
            self.position.altitude.unwrap(),
        );
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
}

fn spherical_to_cartesian(r: f64, lat: f64, lon: f64) -> Vector3<f64> {
    let x = r * lat.to_radians().cos() * lon.to_radians().cos();
    let y = r * lat.to_radians().cos() * lon.to_radians().sin();
    let z = r * lat.to_radians().sin();
    Vector3::new(x, y, z)
}

fn pos_to_3d(shape: &EarthShape, lat: f64, lon: f64, elev: f64) -> Vector3<f64> {
    match shape {
        EarthShape::Spherical { radius } => spherical_to_cartesian(radius + elev, lat, lon),
        EarthShape::Flat => {
            let z = elev;
            let r = (90.0 - lat) * 10_000_000.0 / 90.0;
            let x = r * lon.to_radians().cos();
            let y = r * lon.to_radians().sin();
            Vector3::new(x, y, z)
        }
    }
}
