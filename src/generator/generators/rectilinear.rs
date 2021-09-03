use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
    time::SystemTime,
};

use nalgebra::{Matrix, Vector3};
use rayon::prelude::*;

use super::{
    utils::{gen_path_cache, gen_terrain_cache, get_single_pixel, PathElem, TerrainData},
    Generator, ResultPixel, TracePoint,
};

use crate::{generator::params::Params, terrain::Terrain};

pub struct RectilinearGenerator<'a, 'b> {
    params: &'a Params,
    terrain: &'b Terrain,
    start: SystemTime,
}

struct Cache {
    min_elev_step: f64,
    min_dir_step: f64,
    terrain: RwLock<HashMap<i32, Vec<TerrainData>>>,
    paths: RwLock<HashMap<i32, Vec<PathElem>>>,
    pixels: RwLock<HashMap<CacheCoords, ResultPixel>>,
}

impl Cache {
    fn new(min_elev_step: f64, min_dir_step: f64) -> Self {
        Self {
            min_elev_step,
            min_dir_step,
            terrain: RwLock::new(Default::default()),
            paths: RwLock::new(Default::default()),
            pixels: RwLock::new(Default::default()),
        }
    }

    fn get_path_cache(&self, params: &Params, terrain: &Terrain, elev_index: i32) -> Vec<PathElem> {
        let maybe_result = self.paths.read().unwrap().get(&elev_index).cloned();
        if let Some(result) = maybe_result {
            result
        } else {
            let elev = elev_index as f64 * self.min_elev_step;
            let path_cache = gen_path_cache(params, terrain, elev.to_degrees());
            self.paths
                .write()
                .unwrap()
                .insert(elev_index, path_cache.clone());
            path_cache
        }
    }

    fn get_terrain_cache(
        &self,
        params: &Params,
        terrain: &Terrain,
        dir_index: i32,
    ) -> Vec<TerrainData> {
        let maybe_result = self.terrain.read().unwrap().get(&dir_index).cloned();
        if let Some(result) = maybe_result {
            result
        } else {
            let dir = dir_index as f64 * self.min_dir_step;
            let terrain_cache = gen_terrain_cache(params, terrain, dir.to_degrees());
            self.terrain
                .write()
                .unwrap()
                .insert(dir_index, terrain_cache.clone());
            terrain_cache
        }
    }

    fn get_pixel(&self, params: &Params, terrain: &Terrain, point: CacheCoords) -> ResultPixel {
        let maybe_result = self.pixels.read().unwrap().get(&point).cloned();
        if let Some(result) = maybe_result {
            result
        } else {
            let path_cache = self.get_path_cache(params, terrain, point.elev_index);
            let terrain_cache = self.get_terrain_cache(params, terrain, point.dir_index);
            let trace_points = get_single_pixel(
                terrain_cache.into_iter().zip(path_cache.into_iter()),
                &params.scene.objects,
                &params.env.shape,
            );
            let result = ResultPixel {
                elevation_angle: (point.elev_index as f64 * self.min_elev_step).to_degrees(),
                azimuth: (point.dir_index as f64 * self.min_dir_step).to_degrees(),
                trace_points,
            };
            self.pixels.write().unwrap().insert(point, result.clone());
            result
        }
    }
}

impl<'a, 'b> Generator for RectilinearGenerator<'a, 'b> {
    fn generate(&self) -> Vec<Vec<ResultPixel>> {
        println!(
            "{}: Generating FoV data...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let fov_data = self.gen_fov_data();

        println!(
            "{}: Calculating pixels...",
            self.start.elapsed().unwrap().as_secs_f64()
        );
        let count_pixels = AtomicUsize::new(0);
        let total_pixels = self.params.output.width as usize * self.params.output.height as usize;

        let cache = Cache::new(fov_data.min_elev_step, fov_data.min_dir_step);

        let result = (0..self.params.output.height)
            .into_par_iter()
            .map(|y| {
                (0..self.params.output.width)
                    .into_par_iter()
                    .map(|x| {
                        let ray_params = fov_data.ray_params_table[y as usize][x as usize];
                        let (points_to_read, rem_elev, rem_dir) = fov_data.cache_coords(ray_params);
                        let pixels: Vec<_> = points_to_read
                            .into_iter()
                            .map(|point| cache.get_pixel(self.params, self.terrain, point))
                            .collect();
                        let pixel =
                            interpolate(pixels, rem_elev, rem_dir, self.params.simulation_step);
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct CacheCoords {
    elev_index: i32,
    dir_index: i32,
}

#[derive(Clone)]
struct FovData {
    min_elev_step: f64,
    min_dir_step: f64,
    ray_params_table: Vec<Vec<RayParams>>,
}

const SEQUENCE: [(i32, i32); 4] = [(0, 0), (0, 1), (1, 0), (1, 1)];

impl FovData {
    fn cache_coords(&self, ray_params: RayParams) -> (Vec<CacheCoords>, f64, f64) {
        let elev_index_f = ray_params.elevation / self.min_elev_step;
        let dir_index_f = ray_params.direction / self.min_dir_step;
        let elev_index = elev_index_f.floor() as i32;
        let dir_index = dir_index_f.floor() as i32;
        let rem_elev = elev_index_f - elev_index as f64;
        let rem_dir = dir_index_f - dir_index as f64;
        (
            SEQUENCE
                .iter()
                .map(|(i, j)| CacheCoords {
                    elev_index: elev_index + i,
                    dir_index: dir_index + j,
                })
                .collect(),
            rem_elev,
            rem_dir,
        )
    }
}

#[derive(Clone, Debug)]
struct Indexed<T> {
    index: (i32, i32),
    value: T,
}

fn collect_trace_points(pixels: &[ResultPixel], step_size: f64) -> Vec<Vec<Indexed<TracePoint>>> {
    let mut result: Vec<Vec<Indexed<TracePoint>>> = vec![];

    for (&index, pixel) in SEQUENCE.iter().zip(pixels.iter()) {
        for trace_point in &pixel.trace_points {
            if let Some(close_trace_point) = result
                .iter()
                .enumerate()
                .find(|(_, points)| {
                    points.iter().any(|point| {
                        (trace_point.distance - point.value.distance).abs() < step_size
                            && trace_point.color.same_class(&point.value.color)
                    })
                })
                .map(|(idx, _)| idx)
            {
                result[close_trace_point].push(Indexed {
                    index,
                    value: *trace_point,
                });
            } else {
                result.push(vec![Indexed {
                    index,
                    value: *trace_point,
                }]);
            }
        }
    }

    result
}

fn match_sequence<T>(vals: Vec<Indexed<T>>) -> [Option<T>; 4] {
    let mut result = [None, None, None, None];
    for val in vals {
        match val.index {
            (0, 0) => {
                result[0] = Some(val.value);
            }
            (0, 1) => {
                result[1] = Some(val.value);
            }
            (1, 0) => {
                result[2] = Some(val.value);
            }
            (1, 1) => {
                result[3] = Some(val.value);
            }
            _ => unreachable!(),
        }
    }
    result
}

fn interpolate_trace_points(
    points: Vec<Indexed<TracePoint>>,
    rem_elev: f64,
    rem_dir: f64,
) -> Option<TracePoint> {
    let present_elems = match_sequence(points);
    match present_elems {
        [None, None, None, None] => None,
        [Some(elem), None, None, None] => {
            if rem_elev < 0.5 && rem_dir < 0.5 {
                Some(elem)
            } else {
                None
            }
        }
        [None, Some(elem), None, None] => {
            if rem_elev < 0.5 && rem_dir >= 0.5 {
                Some(elem)
            } else {
                None
            }
        }
        [None, None, Some(elem), None] => {
            if rem_elev >= 0.5 && rem_dir < 0.5 {
                Some(elem)
            } else {
                None
            }
        }
        [None, None, None, Some(elem)] => {
            if rem_elev >= 0.5 && rem_dir >= 0.5 {
                Some(elem)
            } else {
                None
            }
        }
        [Some(elem0), Some(elem1), None, None] => {
            interpolate_two_adjacent(elem0, elem1, rem_elev, rem_dir)
        }
        [Some(elem0), None, Some(elem1), None] => {
            interpolate_two_adjacent(elem0, elem1, rem_dir, rem_elev)
        }
        [Some(elem0), None, None, Some(elem1)] => {
            interpolate_two_diagonal(elem0, elem1, rem_elev, rem_dir)
        }
        [None, Some(elem0), Some(elem1), None] => {
            interpolate_two_diagonal(elem0, elem1, rem_elev, 1.0 - rem_dir)
        }
        [None, Some(elem0), None, Some(elem1)] => {
            interpolate_two_adjacent(elem0, elem1, 1.0 - rem_dir, rem_elev)
        }
        [None, None, Some(elem0), Some(elem1)] => {
            interpolate_two_adjacent(elem0, elem1, 1.0 - rem_elev, rem_dir)
        }
        [Some(elem0), Some(elem1), Some(elem2), None] => {
            interpolate_three(elem0, elem1, elem2, rem_elev, rem_dir)
        }
        [Some(elem0), Some(elem1), None, Some(elem2)] => {
            interpolate_three(elem1, elem0, elem2, rem_elev, 1.0 - rem_dir)
        }
        [Some(elem0), None, Some(elem1), Some(elem2)] => {
            interpolate_three(elem0, elem2, elem1, 1.0 - rem_elev, rem_dir)
        }
        [None, Some(elem0), Some(elem1), Some(elem2)] => {
            interpolate_three(elem2, elem1, elem0, 1.0 - rem_elev, 1.0 - rem_dir)
        }
        [Some(elem0), Some(elem1), Some(elem2), Some(elem3)] => {
            interpolate_four(elem0, elem1, elem2, elem3, rem_elev, rem_dir)
        }
    }
}

fn interpolate_two_adjacent(
    elem0: TracePoint,
    elem1: TracePoint,
    rem_elev: f64,
    rem_dir: f64,
) -> Option<TracePoint> {
    if rem_elev >= 0.5 {
        None
    } else {
        Some(elem0.interpolate(&elem1, rem_dir))
    }
}

fn interpolate_two_diagonal(
    elem0: TracePoint,
    elem1: TracePoint,
    rem_elev: f64,
    rem_dir: f64,
) -> Option<TracePoint> {
    if (rem_elev >= 0.5 && rem_dir < 0.5) || (rem_elev < 0.5 && rem_dir >= 0.5) {
        None
    } else {
        let coeff = rem_elev * rem_dir / (rem_elev * rem_dir + (1.0 - rem_elev) * (1.0 - rem_dir));
        Some(elem0.interpolate(&elem1, coeff))
    }
}

fn interpolate_three(
    elem0: TracePoint,
    elem1: TracePoint,
    elem2: TracePoint,
    rem_elev: f64,
    rem_dir: f64,
) -> Option<TracePoint> {
    if rem_elev >= 0.5 && rem_dir >= 0.5 {
        None
    } else {
        let sum = 1.0 - rem_elev + rem_elev * (1.0 - rem_dir);
        let interp = elem0.interpolate(&elem1, rem_dir);
        Some(interp.interpolate(&elem2, rem_elev * (1.0 - rem_dir) / sum))
    }
}

fn interpolate_four(
    elem0: TracePoint,
    elem1: TracePoint,
    elem2: TracePoint,
    elem3: TracePoint,
    rem_elev: f64,
    rem_dir: f64,
) -> Option<TracePoint> {
    let interp1 = elem0.interpolate(&elem1, rem_dir);
    let interp2 = elem2.interpolate(&elem3, rem_dir);
    Some(interp1.interpolate(&interp2, rem_elev))
}

fn interpolate(
    pixels: Vec<ResultPixel>,
    rem_elev: f64,
    rem_dir: f64,
    step_size: f64,
) -> ResultPixel {
    assert_eq!(pixels.len(), 4);
    let trace_points = collect_trace_points(&pixels, step_size)
        .into_iter()
        .filter_map(|points| interpolate_trace_points(points, rem_elev, rem_dir))
        .collect();

    ResultPixel {
        elevation_angle: pixels[0].elevation_angle * (1.0 - rem_elev) * (1.0 - rem_dir)
            + pixels[1].elevation_angle * (1.0 - rem_elev) * rem_dir
            + pixels[2].elevation_angle * rem_elev * (1.0 - rem_dir)
            + pixels[3].elevation_angle * rem_elev * rem_dir,
        azimuth: pixels[0].azimuth * (1.0 - rem_elev) * (1.0 - rem_dir)
            + pixels[1].azimuth * (1.0 - rem_elev) * rem_dir
            + pixels[2].azimuth * rem_elev * (1.0 - rem_dir)
            + pixels[3].azimuth * rem_elev * rem_dir,
        trace_points,
    }
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

    fn gen_fov_data(&self) -> FovData {
        const SCALE: f64 = 1.5;

        let ray_params = (0..self.params.output.height)
            .into_par_iter()
            .map(|y| {
                (0..self.params.output.width)
                    .into_par_iter()
                    .map(|x| self.get_ray_params(x, y))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let min_elev_step = (0..self.params.output.width)
            .into_par_iter()
            .map(|x| {
                let mut min = 360.0_f64.to_radians();
                let mut last_elev = ray_params[0][x as usize].elevation;
                for y in 1..self.params.output.height {
                    let next_elev = ray_params[y as usize][x as usize].elevation;
                    let mut diff = (next_elev - last_elev).abs();
                    let min_diff = self.params.view.frame.fov.to_radians()
                        / (self.params.output.width as f64)
                        / 3.0;
                    if diff < min_diff {
                        diff = min_diff;
                    }
                    if diff < min {
                        min = diff;
                    }
                    last_elev = next_elev;
                }
                min
            })
            .reduce(|| f64::INFINITY, |a, b| a.min(b))
            * SCALE;

        let min_dir_step = (0..self.params.output.height)
            .into_par_iter()
            .map(|y| {
                let mut min = 360.0_f64.to_radians();
                let mut last_dir = ray_params[y as usize][0].direction;
                for x in 1..self.params.output.width {
                    let next_dir = ray_params[y as usize][x as usize].direction;
                    let mut diff = (next_dir - last_dir).abs();
                    let min_diff = self.params.view.frame.fov.to_radians()
                        / (self.params.output.width as f64)
                        / 3.0;
                    if diff > 360.0_f64.to_radians() {
                        diff -= 360.0_f64.to_radians();
                    }
                    if diff < min_diff {
                        diff = min_diff;
                    }
                    if diff < min {
                        min = diff;
                    }
                    last_dir = next_dir;
                }
                min
            })
            .reduce(|| f64::INFINITY, |a, b| a.min(b))
            * SCALE;

        FovData {
            min_elev_step,
            min_dir_step,
            ray_params_table: ray_params,
        }
    }
}
