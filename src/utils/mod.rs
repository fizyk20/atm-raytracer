mod earth_model;

use image::{Rgb, Rgba};
use nalgebra::{Vector3, Vector4};

pub use earth_model::{DirectionalCalc, EarthModel};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Coords {
    pub lat: f64,
    pub lon: f64,
    pub elev: f64,
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
