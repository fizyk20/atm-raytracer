use std::{
    collections::{hash_map::Entry, HashMap},
    env,
};

use crate::{
    generator::{
        params::{Params, Tick, VerticalTick},
        ResultPixel,
    },
    utils::{rgb_to_vec3, vec3_to_rgb},
};

use image::{ImageBuffer, Pixel, Rgb};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut};
use rusttype::{Font, Scale};

static FONT: &[u8] = include_bytes!("DejaVuSans.ttf");

struct DrawTick {
    size: u32,
    angle: f64,
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

fn azimuth_to_x(azimuth: f64, pixels: &[Vec<ResultPixel>]) -> Option<u32> {
    let candidate = pixels[0]
        .iter()
        .enumerate()
        .min_by(|(_, pixel1), (_, pixel2)| {
            diff_azimuth(azimuth, pixel1.azimuth)
                .abs()
                .partial_cmp(&diff_azimuth(azimuth, pixel2.azimuth).abs())
                .unwrap()
        })
        .map(|(idx, _)| idx as u32)
        .unwrap();
    let neighboring_idx = if candidate == 0 { 1 } else { candidate - 1 };
    let diff_per_pixel = diff_azimuth(
        pixels[0][candidate as usize].azimuth,
        pixels[0][neighboring_idx as usize].azimuth,
    )
    .abs();
    (diff_azimuth(pixels[0][candidate as usize].azimuth, azimuth).abs() < diff_per_pixel * 1.5)
        .then(|| candidate)
}

fn elevation_to_y(elevation: f64, pixels: &[Vec<ResultPixel>]) -> Option<u32> {
    let candidate = pixels
        .iter()
        .map(|pixels_row| &pixels_row[0])
        .enumerate()
        .min_by(|(_, pixel1), (_, pixel2)| {
            (elevation - pixel1.elevation_angle)
                .abs()
                .partial_cmp(&(elevation - pixel2.elevation_angle).abs())
                .unwrap()
        })
        .map(|(idx, _)| idx as u32)
        .unwrap();
    let neighboring_idx = if candidate == 0 { 1 } else { candidate - 1 };
    let diff_per_pixel = (pixels[candidate as usize][0].elevation_angle
        - pixels[neighboring_idx as usize][0].elevation_angle)
        .abs();
    ((pixels[candidate as usize][0].elevation_angle - elevation).abs() < diff_per_pixel * 1.5)
        .then(|| candidate)
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
            if let Some(x) = azimuth_to_x(azimuth, pixels) {
                vec![(
                    x,
                    DrawTick {
                        angle: azimuth,
                        size,
                        labelled,
                    },
                )]
            } else {
                vec![]
            }
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
                if let Some(x) = azimuth_to_x(current_az, pixels) {
                    result.push((
                        x,
                        DrawTick {
                            size,
                            labelled,
                            angle: azimuth,
                        },
                    ));
                }
                current_az += step;
            }
            result
        }
    }
}

fn into_draw_ticks_vertical(
    tick: &VerticalTick,
    params: &Params,
    pixels: &[Vec<ResultPixel>],
) -> Vec<(u32, DrawTick)> {
    match *tick {
        VerticalTick::Single {
            elevation,
            size,
            labelled,
        } => {
            if let Some(y) = elevation_to_y(elevation, pixels) {
                vec![(
                    y,
                    DrawTick {
                        angle: elevation,
                        size,
                        labelled,
                    },
                )]
            } else {
                vec![]
            }
        }
        VerticalTick::Multiple {
            bias,
            step,
            size,
            labelled,
        } => {
            let aspect = params.output.height as f64 / params.output.width as f64;
            let min_elev = params.view.frame.tilt - params.view.frame.fov * aspect / 2.0;
            let max_elev = params.view.frame.tilt + params.view.frame.fov * aspect / 2.0;
            let mut current_elev = ((min_elev - bias) / step).ceil() * step + bias;
            let mut result = Vec::new();
            while current_elev < max_elev {
                let elevation = if current_elev < -90.0 {
                    -180.0 - current_elev
                } else if current_elev > 90.0 {
                    180.0 - current_elev
                } else {
                    current_elev
                };
                if let Some(y) = elevation_to_y(elevation, pixels) {
                    result.push((
                        y,
                        DrawTick {
                            size,
                            labelled,
                            angle: elevation,
                        },
                    ));
                }
                current_elev += step;
            }
            result
        }
    }
}

struct TicksToDraw {
    horizontal: HashMap<u32, DrawTick>,
    vertical: HashMap<u32, DrawTick>,
}

fn gen_ticks(params: &Params, pixels: &[Vec<ResultPixel>]) -> TicksToDraw {
    let mut horizontal = HashMap::new();
    let mut vertical = HashMap::new();
    for tick in &params.output.ticks {
        let new_ticks = into_draw_ticks(tick, params, pixels);
        for (x, tick) in new_ticks {
            match horizontal.entry(x) {
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
    for tick in &params.output.vertical_ticks {
        let new_ticks = into_draw_ticks_vertical(tick, params, pixels);
        for (y, tick) in new_ticks {
            match vertical.entry(y) {
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
    TicksToDraw {
        horizontal,
        vertical,
    }
}

fn draw_ticks(
    img: &mut ImageBuffer<Rgb<u8>, Vec<<Rgb<u8> as Pixel>::Subpixel>>,
    params: &Params,
    pixels: &[Vec<ResultPixel>],
) {
    let font = Font::try_from_bytes(FONT).unwrap();
    let height = 15.0;
    let scale = Scale {
        x: height,
        y: height,
    };
    let TicksToDraw {
        horizontal,
        vertical,
    } = gen_ticks(params, pixels);
    for (x, tick) in horizontal {
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
                x as i32 - 8,
                tick.size as i32 + 5,
                scale,
                &font,
                &format!("{}", tick.angle),
            );
        }
    }
    for (y, tick) in vertical {
        draw_line_segment_mut(
            img,
            (0.0, y as f32),
            (tick.size as f32, y as f32),
            Rgb([255, 255, 255]),
        );
        if tick.labelled {
            draw_text_mut(
                img,
                Rgb([255, 255, 255]),
                tick.size as i32 + 5,
                y as i32 - 7,
                scale,
                &font,
                &format!("{}", tick.angle),
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

fn add(rgb1: Rgb<u8>, rgb2: Rgb<u8>, a: f64) -> Rgb<u8> {
    let color1 = rgb_to_vec3(rgb1);
    let color2 = rgb_to_vec3(rgb2);
    let result = color1 + color2 * a;
    vec3_to_rgb(result)
}

pub fn draw_image(pixels: &[Vec<ResultPixel>], params: &Params) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(params.output.width as u32, params.output.height as u32);
    let coloring = params.view.coloring.coloring_method();
    let def_color = if params.view.fog_distance.is_some() {
        Rgb([160, 160, 160])
    } else {
        Rgb([28, 28, 28])
    };
    for (x, y, px) in img.enumerate_pixels_mut() {
        let mut result = Rgb([0, 0, 0]);
        let mut accum_neg_alpha = 1.0;

        for pixel in &pixels[y as usize][x as usize].trace_points {
            let color1 = coloring.color_for_pixel(pixel);
            let color2 = if let Some(fog_dist) = params.view.fog_distance {
                fog(fog_dist, pixel.path_length, color1)
            } else {
                color1
            };
            result = add(result, color2, accum_neg_alpha * pixel.color.alpha());
            accum_neg_alpha *= 1.0 - pixel.color.alpha();
        }

        *px = add(result, def_color, accum_neg_alpha);
    }

    img
}

pub fn output_image(pixels: &[Vec<ResultPixel>], params: &Params) {
    let mut img = draw_image(pixels, params);

    draw_ticks(&mut img, &params, pixels);
    draw_eye_level(&mut img, &params, pixels);

    let mut output_file = env::current_dir().unwrap();
    output_file.push(&params.output.file);

    img.save(output_file).unwrap();
}
