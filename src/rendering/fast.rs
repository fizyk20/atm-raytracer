use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use rayon::prelude::*;

use super::{
    calc_dist, get_coords_at_dist,
    utils::{get_single_pixel, PathElem, TerrainData},
    Generator, ResultPixel,
};

use crate::{params::Params, terrain::Terrain};

pub struct FastGenerator<'a, 'b> {
    params: &'a Params,
    terrain: &'b Terrain,
    start: SystemTime,
}

impl<'a, 'b> Generator for FastGenerator<'a, 'b> {
    fn generate(&self) -> Vec<Vec<ResultPixel>> {
        println!(
            "{}: Generating terrain cache...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let terrain_cache = (0..self.params.output.width)
            .into_par_iter()
            .map(|x| gen_terrain_cache(self.params, self.terrain, x as u16))
            .collect::<Vec<_>>();
        println!(
            "{}: Generating path cache...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let path_cache = (0..self.params.output.height)
            .into_par_iter()
            .map(|y| gen_path_cache(self.params, self.terrain, y as u16))
            .collect::<Vec<_>>();

        println!(
            "{}: Calculating pixels...",
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
                            &self.params.env.shape,
                        );
                        let pixel = ResultPixel {
                            elevation_angle: get_ray_elev(self.params, y),
                            azimuth: get_ray_dir(self.params, x),
                            trace_points,
                        };
                        let pixels_done = count_pixels.fetch_add(1, Ordering::SeqCst);
                        let prev_percent = pixels_done * 100 / total_pixels;
                        let new_percent = (pixels_done + 1) * 100 / total_pixels;
                        if new_percent > prev_percent {
                            println!(
                                "{}: {}%...",
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
            "{}: Done calculating",
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

fn gen_path_cache(params: &Params, terrain: &Terrain, y: u16) -> Vec<PathElem> {
    let ray_elev = get_ray_elev(params, y);
    let alt = params.view.position.altitude.abs(
        terrain,
        params.view.position.latitude,
        params.view.position.longitude,
    );
    let mut ray = params
        .env
        .cast_ray_stepper(alt, ray_elev.to_radians(), params.straight_rays);
    ray.set_step_size(params.simulation_step);

    let mut path = vec![PathElem {
        dist: 0.0,
        elev: alt,
        path_length: 0.0,
    }];
    let mut ray_state = ray.next().unwrap();
    let mut path_length = 0.0;

    loop {
        let new_ray_state = ray.next().unwrap();
        path_length += calc_dist(params, ray_state, new_ray_state);
        path.push(PathElem {
            dist: ray_state.x,
            elev: ray_state.h,
            path_length,
        });
        if ray_state.x > params.view.frame.max_distance || ray_state.h < -1000.0 {
            break;
        }
        ray_state = new_ray_state;
    }

    path
}

fn gen_terrain_cache(params: &Params, terrain: &Terrain, x: u16) -> Vec<TerrainData> {
    let dir = get_ray_dir(params, x);
    let mut distance = 0.0;

    let mut result = vec![];
    while distance < params.view.frame.max_distance {
        distance += params.simulation_step;
        let (lat, lon) = get_coords_at_dist(
            &params.env.shape,
            (
                params.view.position.latitude,
                params.view.position.longitude,
            ),
            dir,
            distance,
        );
        let terrain_data = TerrainData::from_lat_lon(lat, lon, params, terrain);
        result.push(terrain_data);
    }

    result
}
