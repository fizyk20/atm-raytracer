use super::ColoringMethod;

use crate::generate::ResultPixel;

use image::Rgb;

pub struct SimpleColors {
    max_distance: f64,
    water_level: f64,
}

impl SimpleColors {
    pub fn new(max_distance: f64, water_level: f64) -> Self {
        Self {
            max_distance,
            water_level,
        }
    }
}

impl ColoringMethod for SimpleColors {
    fn color_for_pixel(&self, pixel: &ResultPixel) -> Rgb<u8> {
        let dist_ratio = pixel.distance / self.max_distance;
        if pixel.elevation <= self.water_level {
            let mul = 1.0 - dist_ratio * 0.6;
            Rgb([0, (128.0 * mul) as u8, (255.0 * mul) as u8])
        } else {
            let elev_ratio = pixel.elevation / 4500.0;
            let h = 120.0
                - 240.0
                    * if elev_ratio < 0.0 {
                        -(-elev_ratio).powf(0.65)
                    } else {
                        elev_ratio.powf(0.65)
                    };
            let v = if elev_ratio > 0.7 {
                2.1 - elev_ratio * 2.0
            } else {
                0.9 - elev_ratio / 0.7 * 0.2
            } * (1.0 - dist_ratio * 0.6);
            let s = 1.0 - dist_ratio * 0.9;
            hsv(h, s, v)
        }
    }
}

#[allow(clippy::many_single_char_names)]
fn hsv(h: f64, s: f64, v: f64) -> Rgb<u8> {
    let c = v * s;
    let h = if h % 360.0 < 0.0 {
        h % 360.0 + 360.0
    } else {
        h % 360.0
    };
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (rp, gp, bp) = if h >= 0.0 && h < 60.0 {
        (c, x, 0.0)
    } else if h >= 60.0 && h < 120.0 {
        (x, c, 0.0)
    } else if h >= 120.0 && h < 180.0 {
        (0.0, c, x)
    } else if h >= 180.0 && h < 240.0 {
        (0.0, x, c)
    } else if h >= 240.0 && h < 300.0 {
        (x, 0.0, c)
    } else if h >= 300.0 && h < 360.0 {
        (c, 0.0, x)
    } else {
        unreachable!();
    };

    Rgb([
        ((rp + m) * 255.0) as u8,
        ((gp + m) * 255.0) as u8,
        ((bp + m) * 255.0) as u8,
    ])
}
