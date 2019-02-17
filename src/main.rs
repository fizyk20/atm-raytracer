mod generate;
mod params;
mod terrain;

use crate::generate::get_single_pixel;
use crate::params::Params;
use crate::terrain::Terrain;
use image::{ImageBuffer, Rgb};
use rayon::prelude::*;
use std::env;
use std::fs;

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

fn color_from_elev_dist(params: &Params, elev: f64, dist: f64) -> Rgb<u8> {
    let dist_ratio = dist / params.max_dist;
    if elev == 0.0 {
        let mul = 1.0 - dist_ratio * 0.6;
        Rgb([0, (128.0 * mul) as u8, (255.0 * mul) as u8])
    } else {
        let elev_ratio = elev / 4500.0;
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

fn main() {
    let params = params::parse_params();

    let mut terrain = Terrain::new();
    let mut terrain_folder = env::current_dir().unwrap();
    terrain_folder.push(&params.terrain_folder);

    for dir_entry in fs::read_dir(terrain_folder).expect("Error opening the terrain data directory")
    {
        let file_path = dir_entry
            .expect("Error reading an entry in the terrain directory")
            .path();
        println!("Loading terrain file: {:?}", file_path);
        terrain.load_dted(&file_path);
    }

    let mut img = ImageBuffer::new(params.pic_width as u32, params.pic_height as u32);
    img.enumerate_pixels_mut()
        .par_bridge()
        .for_each(|(x, y, px)| {
            if x == 0 && y % 10 == 0 {
                println!("x = {}, y = {}", x, y);
            }
            let pixel = get_single_pixel(&params, &terrain, x as u16, y as u16);
            if let Some(pixel) = pixel {
                *px = color_from_elev_dist(&params, pixel.elevation, pixel.distance);
            } else {
                *px = Rgb([28, 28, 28]);
            }
        });

    let mut output_file = env::current_dir().unwrap();
    output_file.push(&params.output_file);

    img.save(output_file).unwrap();
}
