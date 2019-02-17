mod generate;
mod params;
mod terrain;

use crate::generate::get_single_pixel;
use crate::terrain::Terrain;
use image::{ImageBuffer, Rgb};
use rayon::prelude::*;
use std::env;
use std::fs;

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
                if pixel.elevation == 0.0 {
                    *px = Rgb([0, 128, 255]);
                } else {
                    let dist_ratio = pixel.distance / params.max_dist;
                    let elev_ratio = pixel.elevation / 1500.0;
                    let r = (elev_ratio * 215.0) as u8;
                    let g = ((1.0 - elev_ratio) * 215.0) as u8;
                    let b = ((0.5 - dist_ratio / 2.0) * 255.0) as u8;
                    *px = Rgb([r, g, b]);
                }
            } else {
                *px = Rgb([28, 28, 28]);
            }
        });

    let mut output_file = env::current_dir().unwrap();
    output_file.push(&params.output_file);

    img.save(output_file).unwrap();
}
