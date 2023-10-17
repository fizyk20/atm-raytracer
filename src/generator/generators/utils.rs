use std::collections::HashSet;

use atm_refraction::{EarthShape, RayState};
use nalgebra::Vector3;

use crate::{
    generator::params::Params,
    object::Object,
    terrain::Terrain,
    utils::{Coords, EarthModel},
};

use super::{PixelColor, TracePoint};

pub fn find_normal(model: &EarthModel, lat: f64, lon: f64, terrain: &Terrain) -> Vector3<f64> {
    const DIFF: f64 = 15.0;

    let ns_calc = model.coords_at_dist_calc((lat, lon), 0.0);
    let ew_calc = model.coords_at_dist_calc((lat, lon), 90.0);

    let p_north = ns_calc.coords_at_dist(DIFF);
    let p_south = ns_calc.coords_at_dist(-DIFF);
    let p_east = ew_calc.coords_at_dist(DIFF);
    let p_west = ew_calc.coords_at_dist(-DIFF);

    let (dir_north, dir_east, dir_up) = model.world_directions(lat, lon);

    let diff_ew = terrain.get_elev(p_east.0, p_east.1).unwrap_or(0.0)
        - terrain.get_elev(p_west.0, p_west.1).unwrap_or(0.0);
    let diff_ns = terrain.get_elev(p_north.0, p_north.1).unwrap_or(0.0)
        - terrain.get_elev(p_south.0, p_south.1).unwrap_or(0.0);

    let vec_ns = 2.0 * DIFF * dir_north + diff_ns * dir_up;
    let vec_ew = 2.0 * DIFF * dir_east + diff_ew * dir_up;

    let mut normal = vec_ew.cross(&vec_ns);
    normal.normalize_mut();

    normal
}

pub fn calc_dist(params: &Params, old_state: RayState, new_state: RayState) -> f64 {
    let dx = new_state.x - old_state.x;
    let dh = new_state.h - old_state.h;
    match params.env.shape {
        EarthShape::Flat => (dx * dx + dh * dh).sqrt(),
        EarthShape::Spherical { radius } => {
            let avg_h = (new_state.h + old_state.h) / 2.0;
            let dx = dx / radius * (avg_h + radius);
            (dx * dx + dh * dh).sqrt()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PathElem {
    pub dist: f64,
    pub elev: f64,
    pub path_length: f64,
}

#[derive(Debug, Clone)]
pub struct TerrainData {
    pub lat: f64,
    pub lon: f64,
    pub elev: f64,
    pub normal: Vector3<f64>,
    pub objects_close: Vec<usize>,
}

impl TerrainData {
    pub fn from_lat_lon(lat: f64, lon: f64, params: &Params, terrain: &Terrain) -> Self {
        let normal = find_normal(&params.model, lat, lon, terrain);
        let objects_close = params
            .scene
            .objects
            .iter()
            .enumerate()
            .filter(|(_, obj)| obj.is_close(&params.model, params.simulation_step, lat, lon))
            .map(|(index, _)| index)
            .collect();
        TerrainData {
            lat,
            lon,
            elev: terrain.get_elev(lat, lon).unwrap_or(0.0),
            normal,
            objects_close,
        }
    }
}

struct TracingState {
    terrain_data: TerrainData,
    ray_elev: f64,
    dist: f64,
    path_len: f64,
}

impl TracingState {
    fn new(terrain_data: &TerrainData, ray_elev: f64, dist: f64, path_len: f64) -> Self {
        Self {
            terrain_data: terrain_data.clone(),
            ray_elev,
            dist,
            path_len,
        }
    }

    fn interpolate(&self, other: &TracingState, prop: f64) -> TracingState {
        TracingState {
            terrain_data: TerrainData {
                lat: self.terrain_data.lat
                    + (other.terrain_data.lat - self.terrain_data.lat) * prop,
                lon: self.terrain_data.lon
                    + (other.terrain_data.lon - self.terrain_data.lon) * prop,
                elev: self.terrain_data.elev
                    + (other.terrain_data.elev - self.terrain_data.elev) * prop,
                normal: self.terrain_data.normal
                    + (other.terrain_data.normal - self.terrain_data.normal) * prop,
                objects_close: vec![],
            },
            ray_elev: self.ray_elev + (other.ray_elev - self.ray_elev) * prop,
            dist: self.dist + (other.dist - self.dist) * prop,
            path_len: self.path_len + (other.path_len - self.path_len) * prop,
        }
    }

    fn ray_coords(&self) -> Coords {
        Coords {
            lat: self.terrain_data.lat,
            lon: self.terrain_data.lon,
            elev: self.ray_elev,
        }
    }
}

pub fn gen_path_cache(params: &Params, terrain: &Terrain, ray_elev: f64) -> Vec<PathElem> {
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
    let mut ray_state = RayState {
        x: 0.0,
        h: alt,
        dh: 0.0,
    };
    let mut path_length = 0.0;

    loop {
        let new_ray_state = ray.next().unwrap();
        path_length += calc_dist(params, ray_state, new_ray_state);
        path.push(PathElem {
            dist: new_ray_state.x,
            elev: new_ray_state.h,
            path_length,
        });
        if ray_state.x > params.view.frame.max_distance || ray_state.h < -1000.0 {
            break;
        }
        ray_state = new_ray_state;
    }

    path
}

pub fn gen_terrain_cache(params: &Params, terrain: &Terrain, dir: f64) -> Vec<TerrainData> {
    let mut distance = 0.0;

    let mut result = Vec::with_capacity(
        (params.view.frame.max_distance / params.simulation_step).ceil() as usize,
    );

    let distance_calc = params.model.coords_at_dist_calc(
        (
            params.view.position.latitude,
            params.view.position.longitude,
        ),
        dir,
    );

    while distance < params.view.frame.max_distance {
        let (lat, lon) = distance_calc.coords_at_dist(distance);
        let terrain_data = TerrainData::from_lat_lon(lat, lon, params, terrain);
        result.push(terrain_data);
        distance += params.simulation_step;
    }

    result
}

pub fn get_single_pixel<I: Iterator<Item = (TerrainData, PathElem)>>(
    mut terrain_and_path: I,
    objects: &[Object],
    earth_model: &EarthModel,
    terrain_alpha: f64,
) -> Vec<TracePoint> {
    let (first_terrain, first_path) = terrain_and_path.next().unwrap();
    let mut old_tracing_state = TracingState::new(&first_terrain, first_path.elev, 0.0, 0.0);
    let mut result = vec![];

    for (terrain_data, path_elem) in terrain_and_path {
        let mut finish = false;
        let mut step_result = vec![];
        let new_tracing_state = TracingState::new(
            &terrain_data,
            path_elem.elev,
            path_elem.dist,
            path_elem.path_length,
        );
        let diff1 = old_tracing_state.ray_elev - old_tracing_state.terrain_data.elev;
        let diff2 = new_tracing_state.ray_elev - new_tracing_state.terrain_data.elev;
        if diff1 * diff2 < 0.0 {
            let prop = diff1 / (diff1 - diff2);
            let interpolated = old_tracing_state.interpolate(&new_tracing_state, prop);
            step_result.push((
                prop,
                TracePoint {
                    lat: interpolated.terrain_data.lat,
                    lon: interpolated.terrain_data.lon,
                    distance: interpolated.dist,
                    elevation: interpolated.terrain_data.elev,
                    path_length: interpolated.path_len,
                    normal: interpolated.terrain_data.normal,
                    color: PixelColor::Terrain(terrain_alpha),
                },
            ));
            if terrain_alpha == 1.0 {
                finish = true;
            }
        }
        if !new_tracing_state.terrain_data.objects_close.is_empty()
            || !old_tracing_state.terrain_data.objects_close.is_empty()
        {
            let object_indices: HashSet<usize> = old_tracing_state
                .terrain_data
                .objects_close
                .iter()
                .chain(new_tracing_state.terrain_data.objects_close.iter())
                .copied()
                .collect();
            for object_index in object_indices {
                let object = &objects[object_index];
                if let Some((prop, normal, color)) = object.check_collision(
                    earth_model,
                    old_tracing_state.ray_coords(),
                    new_tracing_state.ray_coords(),
                ) {
                    if color.a == 0.0 {
                        continue;
                    }
                    let interpolated = old_tracing_state.interpolate(&new_tracing_state, prop);
                    step_result.push((
                        prop,
                        TracePoint {
                            lat: interpolated.terrain_data.lat,
                            lon: interpolated.terrain_data.lon,
                            distance: interpolated.dist,
                            elevation: interpolated.ray_elev,
                            path_length: interpolated.path_len,
                            normal,
                            color: PixelColor::Rgba(color),
                        },
                    ));
                    if color.a == 1.0 {
                        finish = true;
                    }
                }
            }
        }
        step_result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        result.extend(step_result.into_iter().map(|p| p.1));
        if finish {
            break;
        }
        old_tracing_state = new_tracing_state;
    }
    result
}
