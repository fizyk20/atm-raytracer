use atm_refraction::EarthShape;
use image::{Rgb, Rgba};
use nalgebra::{Vector3, Vector4};

pub fn world_directions(
    shape: &EarthShape,
    lat: f64,
    lon: f64,
) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
    match *shape {
        EarthShape::Flat => {
            let lon_rad = lon.to_radians();

            let sinlon = lon_rad.sin();
            let coslon = lon_rad.cos();

            (
                Vector3::new(-coslon, -sinlon, 0.0),
                Vector3::new(-sinlon, coslon, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
            )
        }
        EarthShape::Spherical { .. } => {
            let lat_rad = lat.to_radians();
            let lon_rad = lon.to_radians();

            let sinlon = lon_rad.sin();
            let coslon = lon_rad.cos();
            let sinlat = lat_rad.sin();
            let coslat = lat_rad.cos();

            // up direction
            let dirup = Vector3::new(coslat * coslon, coslat * sinlon, sinlat);
            // vector tangent to Earth's surface pointing north
            let dirn = Vector3::new(-sinlat * coslon, -sinlat * sinlon, coslat);
            // vector tangent to Earth's surface pointing east
            let dire = Vector3::new(-sinlon, coslon, 0.0);

            (dirn, dire, dirup)
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Coords {
    pub lat: f64,
    pub lon: f64,
    pub elev: f64,
}

impl Coords {
    pub fn to_cartesian(&self, shape: &EarthShape) -> Vector3<f64> {
        match shape {
            EarthShape::Spherical { radius } => {
                spherical_to_cartesian(radius + self.elev, self.lat, self.lon)
            }
            EarthShape::Flat => {
                let z = self.elev;
                let r = (90.0 - self.lat) * 10_000_000.0 / 90.0;
                let x = r * self.lon.to_radians().cos();
                let y = r * self.lon.to_radians().sin();
                Vector3::new(x, y, z)
            }
        }
    }
}

pub fn spherical_to_cartesian(r: f64, lat: f64, lon: f64) -> Vector3<f64> {
    let x = r * lat.to_radians().cos() * lon.to_radians().cos();
    let y = r * lat.to_radians().cos() * lon.to_radians().sin();
    let z = r * lat.to_radians().sin();
    Vector3::new(x, y, z)
}

#[allow(clippy::many_single_char_names)]
pub fn rgb_to_vec3(rgb: Rgb<u8>) -> Vector3<f64> {
    let r = rgb.0[0] as f64 / 255.0;
    let g = rgb.0[1] as f64 / 255.0;
    let b = rgb.0[2] as f64 / 255.0;
    Vector3::new(r, g, b)
}

#[allow(clippy::many_single_char_names)]
pub fn vec3_to_rgb(v: Vector3<f64>) -> Rgb<u8> {
    let r = (v[0] * 255.0) as u8;
    let g = (v[1] * 255.0) as u8;
    let b = (v[2] * 255.0) as u8;
    Rgb([r, g, b])
}

#[allow(clippy::many_single_char_names)]
pub fn rgba_to_vec4(rgba: Rgba<u8>) -> Vector4<f64> {
    let r = rgba.0[0] as f64 / 255.0;
    let g = rgba.0[1] as f64 / 255.0;
    let b = rgba.0[2] as f64 / 255.0;
    let a = rgba.0[3] as f64 / 255.0;
    Vector4::new(r, g, b, a)
}

#[allow(clippy::many_single_char_names)]
pub fn vec4_to_rgba(v: Vector4<f64>) -> Rgba<u8> {
    let r = (v[0] * 255.0) as u8;
    let g = (v[1] * 255.0) as u8;
    let b = (v[2] * 255.0) as u8;
    let a = (v[3] * 255.0) as u8;
    Rgba([r, g, b, a])
}
