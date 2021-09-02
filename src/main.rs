mod coloring;
mod generators;
mod object;
mod params;
mod terrain;
mod utils;

#[macro_use]
extern crate serde_derive;

use std::{
    collections::{hash_map::Entry, HashMap},
    env, fs,
    io::Write,
    time::SystemTime,
};

use crate::{
    generators::{FastGenerator, Generator, RectilinearGenerator, ResultPixel},
    params::{GeneratorDef, Params, Tick},
    terrain::Terrain,
    utils::{rgb_to_vec3, vec3_to_rgb},
};

use image::{ImageBuffer, Pixel, Rgb};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use libflate::gzip::Encoder;
use rusttype::{FontCollection, Scale};

static FONT: &[u8] = include_bytes!("DejaVuSans.ttf");

struct DrawTick {
    size: u32,
    azimuth: f64,
    labelled: bool,
}

fn diff_azimuth(az1: f64, az2: f64) -> f64 {
    let diff = az1 - az2;
    if diff < -180.0 {
        diff + 360.0
    } else if diff > 180.0 {
        diff - 360.0
    } else {
        diff
    }
}

fn azimuth_to_x(azimuth: f64, pixels: &[Vec<ResultPixel>]) -> u32 {
    pixels[0]
        .iter()
        .enumerate()
        .min_by(|(_, pixel1), (_, pixel2)| {
            diff_azimuth(azimuth, pixel1.azimuth)
                .abs()
                .partial_cmp(&diff_azimuth(azimuth, pixel2.azimuth).abs())
                .unwrap()
        })
        .map(|(idx, _)| idx as u32)
        .unwrap()
}

fn into_draw_ticks(
    tick: &Tick,
    params: &Params,
    pixels: &[Vec<ResultPixel>],
) -> Vec<(u32, DrawTick)> {
    match *tick {
        Tick::Single {
            azimuth,
            size,
            labelled,
        } => {
            let x = azimuth_to_x(azimuth, pixels);
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
                let azimuth = if current_az < 0.0 {
                    current_az + 360.0
                } else if current_az >= 360.0 {
                    current_az - 360.0
                } else {
                    current_az
                };
                let x = azimuth_to_x(current_az, pixels);
                result.push((
                    x,
                    DrawTick {
                        size,
                        labelled,
                        azimuth,
                    },
                ));
                current_az += step;
            }
            result
        }
    }
}

fn gen_ticks(params: &Params, pixels: &[Vec<ResultPixel>]) -> HashMap<u32, DrawTick> {
    let mut result = HashMap::new();
    for tick in &params.output.ticks {
        let new_ticks = into_draw_ticks(tick, params, pixels);
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

fn draw_ticks(
    img: &mut ImageBuffer<Rgb<u8>, Vec<<Rgb<u8> as Pixel>::Subpixel>>,
    params: &Params,
    pixels: &[Vec<ResultPixel>],
) {
    let font = FontCollection::from_bytes(FONT)
        .unwrap()
        .into_font()
        .unwrap();
    let height = 15.0;
    let scale = Scale {
        x: height,
        y: height,
    };
    let ticks = gen_ticks(params, pixels);
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

fn find_eye_level(pixels: &[Vec<ResultPixel>], column: u32) -> u32 {
    let mut min_elev = f64::INFINITY;
    let mut min_elev_idx = 0;
    for (y, row) in pixels.iter().enumerate() {
        if row[column as usize].elevation_angle.abs() < min_elev {
            min_elev = row[column as usize].elevation_angle.abs();
            min_elev_idx = y;
        }
    }
    min_elev_idx as u32
}

fn draw_eye_level(
    img: &mut ImageBuffer<Rgb<u8>, Vec<<Rgb<u8> as Pixel>::Subpixel>>,
    params: &Params,
    pixels: &[Vec<ResultPixel>],
) {
    if params.output.show_eye_level {
        let mut y_old = find_eye_level(pixels, 0);
        for x in 1..params.output.width {
            let y_new = find_eye_level(pixels, x as u32);
            draw_line_segment_mut(
                img,
                ((x - 1) as f32, y_old as f32),
                (x as f32, y_new as f32),
                Rgb([255, 128, 255]),
            );
            y_old = y_new;
        }
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

fn blend(rgb1: Rgb<u8>, rgb2: Rgb<u8>, a: f64) -> Rgb<u8> {
    let color1 = rgb_to_vec3(rgb1);
    let color2 = rgb_to_vec3(rgb2);
    let result = color1 * a + color2 * (1.0 - a);
    vec3_to_rgb(result)
}

fn output_image(pixels: &[Vec<ResultPixel>], params: &Params) {
    let mut img = ImageBuffer::new(params.output.width as u32, params.output.height as u32);
    let coloring = params.view.coloring.coloring_method();
    for (x, y, px) in img.enumerate_pixels_mut() {
        let def_color = if params.view.fog_distance.is_some() {
            Rgb([160, 160, 160])
        } else {
            Rgb([28, 28, 28])
        };
        let mut result = def_color;
        let mut curr_alpha = 0.0;

        for pixel in &pixels[y as usize][x as usize].trace_points {
            let color1 = coloring.color_for_pixel(pixel);
            let color2 = if let Some(fog_dist) = params.view.fog_distance {
                fog(fog_dist, pixel.path_length, color1)
            } else {
                color1
            };
            result = blend(result, color2, curr_alpha);
            curr_alpha += (1.0 - curr_alpha) * pixel.color.alpha();
        }

        *px = blend(result, def_color, curr_alpha);
    }

    draw_ticks(&mut img, &params, pixels);
    draw_eye_level(&mut img, &params, pixels);

    let mut output_file = env::current_dir().unwrap();
    output_file.push(&params.output.file);

    img.save(output_file).unwrap();
}

#[derive(Clone, Serialize, Deserialize)]
struct AllData {
    params: Params,
    result: Vec<Vec<ResultPixel>>,
}

fn output_metadata(filename: &str, pixels: Vec<Vec<ResultPixel>>, params: Params) {
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
    let config = match params::parse_config() {
        Ok(config) => config,
        Err(_) => {
            return;
        }
    };

    let mut terrain_folder = env::current_dir().unwrap();
    terrain_folder.push(config.terrain_folder());

    let start = SystemTime::now();

    println!(
        "{}: Using terrain data directory: {:?}",
        start.elapsed().unwrap().as_secs_f64(),
        terrain_folder
    );

    let terrain = Terrain::from_folder(terrain_folder);

    let params = config.into_params(&terrain);

    let generator: Box<dyn Generator> = match params.output.generator {
        GeneratorDef::Fast => Box::new(FastGenerator::new(&params, &terrain, start)),
        GeneratorDef::Rectilinear => Box::new(RectilinearGenerator::new(&params, &terrain, start)),
    };

    let result_pixels = generator.generate();

    println!(
        "{}: Outputting image...",
        start.elapsed().unwrap().as_secs_f64()
    );
    output_image(&result_pixels, &params);

    if let Some(ref filename) = params.output.file_metadata {
        println!(
            "{}: Outputting metadata...",
            start.elapsed().unwrap().as_secs_f64()
        );
        output_metadata(filename, result_pixels, params.clone());
    }
}
