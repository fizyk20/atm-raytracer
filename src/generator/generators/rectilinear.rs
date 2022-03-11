use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use atm_refraction::{PathStepper, RayState};
use nalgebra::{Matrix, Vector3};
use rayon::prelude::*;

use super::{
    utils::{calc_dist, get_single_pixel, PathElem, TerrainData},
    Generator, ResultPixel,
};

use crate::{generator::params::Params, terrain::Terrain};

pub struct RectilinearGenerator<'a, 'b> {
    params: &'a Params,
    terrain: &'b Terrain,
    start: SystemTime,
}

impl<'a, 'b> Generator for RectilinearGenerator<'a, 'b> {
    fn generate(&self) -> Vec<Vec<ResultPixel>> {
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
                        let ray_params = self.get_ray_params(x, y);
                        let pixel = self.gen_pixel(ray_params);
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

#[derive(Clone, Copy)]
struct RayParams {
    elevation: f64,
    direction: f64,
}

impl<'a, 'b> RectilinearGenerator<'a, 'b> {
    pub fn new(params: &'a Params, terrain: &'b Terrain, start: SystemTime) -> Self {
        Self {
            params,
            terrain,
            start,
        }
    }

    fn get_ray_params(&self, x: u16, y: u16) -> RayParams {
        let width = self.params.output.width as f64;

        let x = (x as i16 - self.params.output.width as i16 / 2) as f64;
        let y = (y as i16 - self.params.output.height as i16 / 2) as f64;
        let z = width / 2.0 / (self.params.view.frame.fov.to_radians() / 2.0).tan();

        let rot = Matrix::from_euler_angles(
            0.0,
            -self.params.view.frame.tilt.to_radians(),
            self.params.view.frame.direction.to_radians(),
        );
        // for Euler angles: [forward, right, up]
        let dir_vec = rot.transform_vector(&Vector3::new(z, x, -y)).normalize();

        let direction = dir_vec.y.atan2(dir_vec.x);
        let elevation = dir_vec.z.asin();

        RayParams {
            elevation,
            direction,
        }
    }

    fn gen_pixel(&self, ray_params: RayParams) -> ResultPixel {
        let path_iterator = PathIterator::new(self.params, self.terrain, ray_params);
        let trace_points = get_single_pixel(
            path_iterator,
            &self.params.scene.objects,
            &self.params.model,
        );
        ResultPixel {
            elevation_angle: ray_params.elevation.to_degrees(),
            azimuth: ray_params.direction.to_degrees(),
            trace_points,
        }
    }
}

struct PathIterator<'a, 'b> {
    path_length: f64,
    ray_state: RayState,
    azimuth: f64,
    ray: Box<dyn PathStepper<Item = RayState> + 'a>,
    params: &'a Params,
    terrain: &'b Terrain,
}

impl<'a, 'b> PathIterator<'a, 'b> {
    fn new(params: &'a Params, terrain: &'b Terrain, ray_params: RayParams) -> Self {
        let alt = params.view.position.altitude.abs(
            terrain,
            params.view.position.latitude,
            params.view.position.longitude,
        );
        let mut ray = params
            .env
            .cast_ray_stepper(alt, ray_params.elevation, params.straight_rays);
        ray.set_step_size(params.simulation_step);

        Self {
            path_length: 0.0,
            ray_state: RayState {
                x: 0.0,
                h: alt,
                dh: 0.0,
            },
            azimuth: ray_params.direction,
            ray,
            params,
            terrain,
        }
    }

    fn current_point(&self) -> (TerrainData, PathElem) {
        let elem = PathElem {
            dist: self.ray_state.x,
            elev: self.ray_state.h,
            path_length: self.path_length,
        };
        let (lat, lon) = self.params.model.get_coords_at_dist(
            (
                self.params.view.position.latitude,
                self.params.view.position.longitude,
            ),
            self.azimuth.to_degrees(),
            elem.dist,
        );
        let terrain_data = TerrainData::from_lat_lon(lat, lon, self.params, self.terrain);
        (terrain_data, elem)
    }
}

impl<'a, 'b> Iterator for PathIterator<'a, 'b> {
    type Item = (TerrainData, PathElem);

    fn next(&mut self) -> Option<(TerrainData, PathElem)> {
        let point = self.current_point();
        if point.1.dist > self.params.view.frame.max_distance || point.1.elev < -1000.0 {
            return None;
        }
        let new_state = self.ray.next()?;
        self.path_length += calc_dist(self.params, self.ray_state, new_state);
        self.ray_state = new_state;
        Some(point)
    }
}
