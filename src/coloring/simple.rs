use super::ColoringMethod;

use crate::rendering::TracePoint;

use image::Rgb;

#[derive(Debug, Clone, Copy)]
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
    fn color_for_pixel(&self, pixel: &TracePoint) -> Rgb<u8> {
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
    let (rp, gp, bp) = if (0.0..60.0).contains(&h) {
        (c, x, 0.0)
    } else if (60.0..120.0).contains(&h) {
        (x, c, 0.0)
    } else if (120.0..180.0).contains(&h) {
        (0.0, c, x)
    } else if (180.0..240.0).contains(&h) {
        (0.0, x, c)
    } else if (240.0..300.0).contains(&h) {
        (x, 0.0, c)
    } else if (300.0..360.0).contains(&h) {
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
