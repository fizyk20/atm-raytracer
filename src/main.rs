mod coloring;
mod generate;
mod object;
mod params;
mod terrain;
mod utils;

#[macro_use]
extern crate serde_derive;

use crate::{
    generate::{gen_path_cache, gen_terrain_cache, get_single_pixel, ResultPixel},
    params::{Params, Tick},
    terrain::Terrain,
};
use image::{ImageBuffer, Pixel, Rgb};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use libflate::gzip::Encoder;
use rayon::prelude::*;
use rusttype::{FontCollection, Scale};
use std::{
    collections::{hash_map::Entry, HashMap},
    env, fs,
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
};

static FONT: &[u8] = include_bytes!("DejaVuSans.ttf");

struct DrawTick {
    size: u32,
    azimuth: f64,
    labelled: bool,
}

fn into_draw_ticks(tick: &Tick, params: &Params) -> Vec<(u32, DrawTick)> {
    match *tick {
        Tick::Single {
            azimuth,
            size,
            labelled,
        } => {
            let x = params.azimuth_to_x(azimuth);
            vec![(
                x,
                DrawTick {
                    azimuth,
                    size,
                    labelled,
                },
            )]
        }
        Tick::Multiple {
            bias,
            step,
            size,
            labelled,
        } => {
            let min_az = params.view.frame.direction - params.view.frame.fov / 2.0;
            let max_az = params.view.frame.direction + params.view.frame.fov / 2.0;
            let mut current_az = ((min_az - bias) / step).ceil() * step + bias;
            let mut result = Vec::new();
            while current_az < max_az {
                let x = params.azimuth_to_x(current_az);
                result.push((
                    x,
                    DrawTick {
                        size,
                        labelled,
                        azimuth: current_az,
                    },
                ));
                current_az += step;
            }
            result
        }
    }
}

fn gen_ticks(params: &Params) -> HashMap<u32, DrawTick> {
    let mut result = HashMap::new();
    for tick in &params.output.ticks {
        let new_ticks = into_draw_ticks(tick, params);
        for (x, tick) in new_ticks {
            match result.entry(x) {
                Entry::Vacant(v) => {
                    v.insert(tick);
                }
                Entry::Occupied(mut o) => {
                    if o.get().size < tick.size {
                        o.insert(tick);
                    }
                }
            }
        }
    }
    result
}

fn draw_ticks(img: &mut ImageBuffer<Rgb<u8>, Vec<<Rgb<u8> as Pixel>::Subpixel>>, params: &Params) {
    let font = FontCollection::from_bytes(FONT)
        .unwrap()
        .into_font()
        .unwrap();
    let height = 15.0;
    let scale = Scale {
        x: height,
        y: height,
    };
    let ticks = gen_ticks(params);
    for (x, tick) in ticks {
        draw_line_segment_mut(
            img,
            (x as f32, 0.0),
            (x as f32, tick.size as f32),
            Rgb([255, 255, 255]),
        );
        if tick.labelled {
            draw_text_mut(
                img,
                Rgb([255, 255, 255]),
                x - 8,
                tick.size + 5,
                scale,
                &font,
                &format!("{}", tick.azimuth),
            );
        }
    }
}

fn draw_eye_level(
    img: &mut ImageBuffer<Rgb<u8>, Vec<<Rgb<u8> as Pixel>::Subpixel>>,
    params: &Params,
) {
    if params.output.show_eye_level {
        let y = params.eye_level_to_y() as f32;
        draw_line_segment_mut(
            img,
            (0.0_f32, y),
            (params.output.width as f32, y),
            Rgb([255, 128, 255]),
        );
    }
}

fn fog(fog_dist: f64, pixel_dist: f64, color: Rgb<u8>) -> Rgb<u8> {
    let fog_coeff = 1.0 - (-pixel_dist / fog_dist).exp();
    let fog_color = Rgb([160u8, 160, 160]);
    let mut new_color = Rgb([0u8; 3]);
    for i in 0..3 {
        new_color.0[i] =
            (color.0[i] as f64 * (1.0 - fog_coeff) + fog_color.0[i] as f64 * fog_coeff) as u8;
    }
    new_color
}

fn output_image(pixels: &[Vec<Option<ResultPixel>>], params: &Params) {
    let mut img = ImageBuffer::new(params.output.width as u32, params.output.height as u32);
    let coloring = params.view.coloring.coloring_method();
    for (x, y, px) in img.enumerate_pixels_mut() {
        if let Some(pixel) = pixels[y as usize][x as usize] {
            let color = coloring.color_for_pixel(&pixel);
            if let Some(fog_dist) = params.view.fog_distance {
                *px = fog(fog_dist, pixel.path_length, color);
            } else {
                *px = color;
            }
        } else if params.view.fog_distance.is_some() {
            *px = Rgb([160, 160, 160]);
        } else {
            *px = Rgb([28, 28, 28]);
        };
    }

    draw_ticks(&mut img, &params);
    draw_eye_level(&mut img, &params);

    let mut output_file = env::current_dir().unwrap();
    output_file.push(&params.output.file);

    img.save(output_file).unwrap();
}

#[derive(Clone, Serialize, Deserialize)]
struct AllData {
    params: Params,
    result: Vec<Vec<Option<ResultPixel>>>,
}

fn output_metadata(filename: &str, pixels: Vec<Vec<Option<ResultPixel>>>, params: Params) {
    let mut file = fs::File::create(filename).expect("failed to create a metadata file");
    let all_data = AllData {
        params,
        result: pixels,
    };

    let all_data_bytes = bincode::serialize(&all_data).expect("failed to serialize metadata");
    let mut gzip_encoder = Encoder::new(Vec::new()).expect("failed to create a GZip encoder");
    gzip_encoder
        .write_all(&all_data_bytes)
        .expect("failed to deflate metadata");
    let zipped_data = gzip_encoder
        .finish()
        .into_result()
        .expect("failed to finish deflating metadata");

    file.write_all(&zipped_data)
        .expect("failed to write metadata to the file");
}

fn main() {
    let mut params = params::parse_params();

    let mut terrain_folder = env::current_dir().unwrap();
    terrain_folder.push(&params.scene.terrain_folder);

    println!("Using terrain data directory: {:?}", terrain_folder);

    let terrain = Terrain::from_folder(terrain_folder);

    // Convert object altitudes to absolute
    for object in &mut params.scene.objects {
        object.position.altitude.convert_into_absolute(
            &terrain,
            object.position.latitude,
            object.position.longitude,
        );
    }

    println!("Generating terrain cache...");
    let terrain_cache = (0..params.output.width)
        .into_par_iter()
        .map(|x| gen_terrain_cache(&params, &terrain, x as u16))
        .collect::<Vec<_>>();
    println!("Generating path cache...");
    let path_cache = (0..params.output.height)
        .into_par_iter()
        .map(|y| gen_path_cache(&params, &terrain, y as u16))
        .collect::<Vec<_>>();

    println!("Calculating pixels...");
    let count_pixels = AtomicUsize::new(0);
    let total_pixels = params.output.width as usize * params.output.height as usize;
    let result_pixels = (0..params.output.height)
        .into_par_iter()
        .map(|y| {
            (0..params.output.width)
                .into_par_iter()
                .map(|x| {
                    let pixel = get_single_pixel(
                        &terrain_cache[x as usize],
                        &path_cache[y as usize],
                        &params.scene.objects,
                        &params.env.shape,
                    );
                    let pixels_done = count_pixels.fetch_add(1, Ordering::SeqCst);
                    let prev_percent = pixels_done * 100 / total_pixels;
                    let new_percent = (pixels_done + 1) * 100 / total_pixels;
                    if new_percent > prev_percent {
                        println!("{}%...", new_percent);
                    }
                    pixel
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    println!("Outputting image...");
    output_image(&result_pixels, &params);

    if let Some(ref filename) = params.output.file_metadata {
        println!("Outputting metadata...");
        output_metadata(filename, result_pixels, params.clone());
    }
}
