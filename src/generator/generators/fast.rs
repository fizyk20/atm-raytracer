use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use rayon::prelude::*;

use super::{
    utils::{gen_path_cache, gen_terrain_cache, get_single_pixel},
    Generator, ResultPixel,
};

use crate::{generator::params::Params, terrain::Terrain};

pub struct FastGenerator<'a, 'b> {
    params: &'a Params,
    terrain: &'b Terrain,
    start: SystemTime,
}

impl<'a, 'b> Generator for FastGenerator<'a, 'b> {
    fn generate(&self) -> Vec<Vec<ResultPixel>> {
        println!(
            "{:.3}: Generating terrain cache...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let terrain_cache = (0..self.params.output.width)
            .into_par_iter()
            .map(|x| {
                let dir = get_ray_dir(self.params, x as u16);
                gen_terrain_cache(self.params, self.terrain, dir)
            })
            .collect::<Vec<_>>();
        println!(
            "{:.3}: Generating path cache...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let path_cache = (0..self.params.output.height)
            .into_par_iter()
            .map(|y| {
                let ray_elev = get_ray_elev(self.params, y as u16);
                gen_path_cache(self.params, self.terrain, ray_elev)
            })
            .collect::<Vec<_>>();

        println!(
            "{:.3}: Calculating pixels...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let count_pixels = AtomicUsize::new(0);
        let total_pixels = self.params.output.width as usize * self.params.output.height as usize;
        let result = (0..self.params.output.height)
            .into_par_iter()
            .map(|y| {
                (0..self.params.output.width)
                    .into_par_iter()
                    .map(|x| {
                        let trace_points = get_single_pixel(
                            terrain_cache[x as usize]
                                .iter()
                                .cloned()
                                .zip(path_cache[y as usize].iter().copied()),
                            &self.params.scene.objects,
                            &self.params.model,
                        );
                        let mut azimuth = get_ray_dir(self.params, x);
                        if azimuth < 0.0 {
                            azimuth += 360.0;
                        } else if azimuth >= 360.0 {
                            azimuth -= 360.0
                        };
                        let pixel = ResultPixel {
                            elevation_angle: get_ray_elev(self.params, y),
                            azimuth,
                            trace_points,
                        };
                        let pixels_done = count_pixels.fetch_add(1, Ordering::SeqCst);
                        let prev_percent = pixels_done * 100 / total_pixels;
                        let new_percent = (pixels_done + 1) * 100 / total_pixels;
                        if new_percent > prev_percent {
                            println!(
                                "{:.3}: {}%...",
                                self.start.elapsed().unwrap().as_secs_f64(),
                                new_percent,
                            );
                        }
                        pixel
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        println!(
            "{:.3}: Done calculating",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        result
    }
}

impl<'a, 'b> FastGenerator<'a, 'b> {
    pub fn new(params: &'a Params, terrain: &'b Terrain, start: SystemTime) -> Self {
        Self {
            params,
            terrain,
            start,
        }
    }
}

fn get_ray_elev(params: &Params, y: u16) -> f64 {
    let width = params.output.width as f64;
    let height = params.output.height as f64;
    let aspect = width / height;

    let y = (y as i16 - params.output.height as i16 / 2) as f64 / height;
    params.view.frame.tilt - y * params.view.frame.fov / aspect
}

fn get_ray_dir(params: &Params, x: u16) -> f64 {
    let width = params.output.width as f64;
    let x = (x as i16 - params.output.width as i16 / 2) as f64 / width;

    params.view.frame.direction + x * params.view.frame.fov
}
