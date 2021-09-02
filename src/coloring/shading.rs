use super::ColoringMethod;

use crate::generators::{PixelColor, TracePoint};

use image::Rgb;
use nalgebra::Vector3;

#[derive(Debug, Clone, Copy)]
pub struct Shading {
    water_level: f64,
    ambient_light: f64,
    light_dir: Vector3<f64>,
}

impl Shading {
    pub fn new(water_level: f64, ambient_light: f64, light_dir: Vector3<f64>) -> Self {
        Self {
            water_level,
            ambient_light,
            light_dir,
        }
    }

    fn calc_brightness(&self, normal: Vector3<f64>) -> f64 {
        let light_dot = self.light_dir.dot(&normal);
        let light_dot = if light_dot >= 0.0 { light_dot } else { 0.0 };
        self.ambient_light + (1.0 - self.ambient_light) * light_dot * light_dot
    }

    fn elev_to_color(elev: f64) -> Vector3<f64> {
        const THR1: f64 = 300.0;
        const THR2: f64 = 1200.0;
        const THR3: f64 = 1800.0;
        const THR4: f64 = 3000.0;
        let green = Vector3::new(0.0, 1.0, 0.0);
        let green_yellow = Vector3::new(0.6, 1.0, 0.0);
        let grey = Vector3::new(0.5, 0.5, 0.5);
        let white = Vector3::new(1.0, 1.0, 1.0);
        if elev < THR1 {
            green
        } else if elev < THR2 {
            let prop = (elev - THR1) / (THR2 - THR1);
            green_yellow * prop + green * (1.0 - prop)
        } else if elev < THR3 {
            let prop = (elev - THR2) / (THR3 - THR2);
            grey * prop + green_yellow * (1.0 - prop)
        } else if elev < THR4 {
            let prop = (elev - THR3) / (THR4 - THR3);
            white * prop + grey * (1.0 - prop)
        } else {
            white
        }
    }
}

impl ColoringMethod for Shading {
    fn color_for_pixel(&self, pixel: &TracePoint) -> Rgb<u8> {
        let brightness = self.calc_brightness(pixel.normal);

        let color = if let PixelColor::Rgba(color) = pixel.color {
            Vector3::new(color.r, color.g, color.b)
        } else if pixel.elevation <= self.water_level {
            Vector3::new(0.0, 0.5, 1.0)
        } else {
            Self::elev_to_color(pixel.elevation)
        } * brightness;

        Rgb([
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
        ])
    }
}
