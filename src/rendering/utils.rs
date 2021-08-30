use std::collections::HashSet;

use atm_refraction::{EarthShape, RayState};
use nalgebra::Vector3;

use crate::{
    object::Object, params::Params, terrain::Terrain, utils::world_directions, utils::Coords,
};

use super::{PixelColor, ResultPixel};

pub fn find_normal(shape: &EarthShape, lat: f64, lon: f64, terrain: &Terrain) -> Vector3<f64> {
    const DIFF: f64 = 15.0;

    let p_north = get_coords_at_dist(shape, (lat, lon), 0.0, DIFF);
    let p_south = get_coords_at_dist(shape, (lat, lon), 180.0, DIFF);
    let p_east = get_coords_at_dist(shape, (lat, lon), 90.0, DIFF);
    let p_west = get_coords_at_dist(shape, (lat, lon), 270.0, DIFF);

    let (dir_north, dir_east, dir_up) = world_directions(shape, lat, lon);

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

const DEGREE_DISTANCE: f64 = 111_111.111;

pub fn get_coords_at_dist(
    shape: &EarthShape,
    start: (f64, f64),
    dir: f64,
    dist: f64,
) -> (f64, f64) {
    match shape {
        EarthShape::Flat => {
            let d_lat = dir.to_radians().cos() * dist / DEGREE_DISTANCE;
            let d_lon =
                dir.to_radians().sin() * dist / DEGREE_DISTANCE / start.0.to_radians().cos();
            (start.0 + d_lat, start.1 + d_lon)
        }
        EarthShape::Spherical { radius } => {
            let ang = dist / radius;

            let (dirn, dire, pos) = world_directions(shape, start.0, start.1);

            // vector tangent to Earth's surface in the given direction
            let dir_rad = dir.to_radians();
            let sindir = dir_rad.sin();
            let cosdir = dir_rad.cos();

            let dir = dirn * cosdir + dire * sindir;

            // final_pos = pos*cos(ang) + dir*sin(ang)
            let sinang = ang.sin();
            let cosang = ang.cos();

            let fpos = pos * cosang + dir * sinang;

            let final_lat_rad = fpos[2].asin();
            let final_lon_rad = fpos[1].atan2(fpos[0]);

            (final_lat_rad.to_degrees(), final_lon_rad.to_degrees())
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
        let normal = find_normal(&params.env.shape, lat, lon, terrain);
        let objects_close = params
            .scene
            .objects
            .iter()
            .enumerate()
            .filter(|(_, obj)| obj.is_close(&params.env.shape, params.simulation_step, lat, lon))
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

pub fn get_single_pixel<I: Iterator<Item = (TerrainData, PathElem)>>(
    mut terrain_and_path: I,
    objects: &[Object],
    earth_shape: &EarthShape,
) -> Vec<ResultPixel> {
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
        if path_elem.elev < terrain_data.elev {
            let diff1 = old_tracing_state.ray_elev - old_tracing_state.terrain_data.elev;
            let diff2 = new_tracing_state.ray_elev - new_tracing_state.terrain_data.elev;
            let prop = diff1 / (diff1 - diff2);
            let interpolated = old_tracing_state.interpolate(&new_tracing_state, prop);
            step_result.push((
                prop,
                ResultPixel {
                    lat: interpolated.terrain_data.lat,
                    lon: interpolated.terrain_data.lon,
                    distance: interpolated.dist,
                    elevation: interpolated.terrain_data.elev,
                    path_length: interpolated.path_len,
                    normal: interpolated.terrain_data.normal,
                    color: PixelColor::Terrain,
                },
            ));
            finish = true;
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
                    earth_shape,
                    old_tracing_state.ray_coords(),
                    new_tracing_state.ray_coords(),
                ) {
                    let interpolated = old_tracing_state.interpolate(&new_tracing_state, prop);
                    step_result.push((
                        prop,
                        ResultPixel {
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