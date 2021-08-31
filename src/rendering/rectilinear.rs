use std::{
    iter,
    sync::atomic::{AtomicUsize, Ordering},
    time::SystemTime,
};

use nalgebra::{Matrix, Vector3};
use rayon::prelude::*;

use super::{
    calc_dist, get_coords_at_dist,
    utils::{get_single_pixel, PathElem, TerrainData},
    Generator, ResultPixel,
};

use crate::{params::Params, terrain::Terrain};

pub struct RectilinearGenerator<'a, 'b> {
    params: &'a Params,
    terrain: &'b Terrain,
    start: SystemTime,
}

impl<'a, 'b> Generator for RectilinearGenerator<'a, 'b> {
    fn generate(&self) -> Vec<Vec<ResultPixel>> {
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
                        let ray_iterator = self.create_ray_iterator(x, y);
                        let trace_points = get_single_pixel(
                            ray_iterator,
                            &self.params.scene.objects,
                            &self.params.env.shape,
                        );
                        let ray_params = self.get_ray_params(x, y);
                        let pixel = ResultPixel {
                            elevation_angle: ray_params.elevation,
                            azimuth: ray_params.direction,
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

    fn create_ray_iterator(
        &self,
        x: u16,
        y: u16,
    ) -> impl Iterator<Item = (TerrainData, PathElem)> + '_ {
        let RayParams {
            elevation: elev,
            direction: dir,
        } = self.get_ray_params(x, y);
        let alt = self.params.view.position.altitude.abs(
            self.terrain,
            self.params.view.position.latitude,
            self.params.view.position.longitude,
        );
        let mut ray = self
            .params
            .env
            .cast_ray_stepper(alt, elev, self.params.straight_rays);
        ray.set_step_size(self.params.simulation_step);

        let mut path_length = 0.0;
        let mut old_state = ray.next().unwrap();

        iter::once(old_state)
            .chain(ray)
            .take_while(move |ray_state| {
                ray_state.x <= self.params.view.frame.max_distance && ray_state.h >= -1000.0
            })
            .map(move |ray_state| {
                path_length += calc_dist(self.params, old_state, ray_state);
                let path_elem = PathElem {
                    dist: ray_state.x,
                    elev: ray_state.h,
                    path_length,
                };
                old_state = ray_state;
                let (lat, lon) = get_coords_at_dist(
                    &self.params.env.shape,
                    (
                        self.params.view.position.latitude,
                        self.params.view.position.longitude,
                    ),
                    dir.to_degrees(),
                    ray_state.x,
                );
                let terrain_data = TerrainData::from_lat_lon(lat, lon, self.params, self.terrain);
                (terrain_data, path_elem)
            })
    }
}
