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

pub struct CorrectGenerator;

impl Generator for CorrectGenerator {
    fn generate(
        &self,
        params: &Params,
        terrain: &Terrain,
        start: SystemTime,
    ) -> Vec<Vec<Vec<ResultPixel>>> {
        println!(
            "{}: Calculating pixels...",
            start.elapsed().unwrap().as_secs_f64()
        );
        let count_pixels = AtomicUsize::new(0);
        let total_pixels = params.output.width as usize * params.output.height as usize;

        let result = (0..params.output.height)
            .into_par_iter()
            .map(|y| {
                (0..params.output.width)
                    .into_par_iter()
                    .map(|x| {
                        let ray_iterator = create_ray_iterator(x, y, params, terrain);
                        let pixel = get_single_pixel(
                            ray_iterator,
                            &params.scene.objects,
                            &params.env.shape,
                        );
                        let pixels_done = count_pixels.fetch_add(1, Ordering::SeqCst);
                        let prev_percent = pixels_done * 100 / total_pixels;
                        let new_percent = (pixels_done + 1) * 100 / total_pixels;
                        if new_percent > prev_percent {
                            println!(
                                "{}: {}%...",
                                start.elapsed().unwrap().as_secs_f64(),
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
            start.elapsed().unwrap().as_secs_f64()
        );
        result
    }
}

fn create_ray_iterator<'c, 'a: 'c, 'b: 'c>(
    x: u16,
    y: u16,
    params: &'a Params,
    terrain: &'b Terrain,
) -> impl Iterator<Item = (TerrainData, PathElem)> + 'c {
    let width = params.output.width as f64;

    let x = (x as i16 - params.output.width as i16 / 2) as f64;
    let y = (y as i16 - params.output.height as i16 / 2) as f64;
    let z = width / 2.0 / (params.view.frame.fov.to_radians() / 2.0).tan();

    let rot = Matrix::from_euler_angles(
        0.0,
        -params.view.frame.tilt.to_radians(),
        params.view.frame.direction.to_radians(),
    );
    // for Euler angles: [forward, right, up]
    let dir_vec = rot.transform_vector(&Vector3::new(z, x, -y)).normalize();

    let dir = dir_vec.y.atan2(dir_vec.x);
    let elev = dir_vec.z.asin();

    let alt = params.view.position.altitude.abs(
        terrain,
        params.view.position.latitude,
        params.view.position.longitude,
    );
    let mut ray = params.env.cast_ray_stepper(alt, elev, params.straight_rays);
    ray.set_step_size(params.simulation_step);

    let mut path_length = 0.0;
    let mut old_state = ray.next().unwrap();

    iter::once(old_state)
        .chain(ray)
        .take_while(move |ray_state| {
            ray_state.x <= params.view.frame.max_distance && ray_state.h >= -1000.0
        })
        .map(move |ray_state| {
            path_length += calc_dist(params, old_state, ray_state);
            let path_elem = PathElem {
                dist: ray_state.x,
                elev: ray_state.h,
                path_length,
            };
            old_state = ray_state;
            let (lat, lon) = get_coords_at_dist(
                &params.env.shape,
                (
                    params.view.position.latitude,
                    params.view.position.longitude,
                ),
                dir.to_degrees(),
                ray_state.x,
            );
            let terrain_data = TerrainData::from_lat_lon(lat, lon, params, terrain);
            (terrain_data, path_elem)
        })
}
