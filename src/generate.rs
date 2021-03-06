use crate::{
    object::{Color, Object},
    params::Params,
    terrain::Terrain,
    utils::{world_directions, Coords},
};

use atm_refraction::{EarthShape, RayState};
use nalgebra::Vector3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PixelColor {
    Terrain,
    Rgba(Color),
}

impl PixelColor {
    pub fn alpha(&self) -> f64 {
        match self {
            PixelColor::Rgba(color) => color.a,
            PixelColor::Terrain => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResultPixel {
    pub lat: f64,
    pub lon: f64,
    pub distance: f64,
    pub elevation: f64,
    pub path_length: f64,
    pub normal: Vector3<f64>,
    pub color: PixelColor,
}

#[derive(Debug, Clone, Copy)]
pub struct PathElem {
    pub dist: f64,
    pub elev: f64,
    pub path_length: f64,
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

fn calc_dist(params: &Params, old_state: RayState, new_state: RayState) -> f64 {
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

pub fn gen_path_cache(params: &Params, terrain: &Terrain, y: u16) -> Vec<PathElem> {
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
        if ray_state.x > params.view.frame.max_distance {
            break;
        }
        ray_state = new_ray_state;
    }

    path
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainData {
    lat: f64,
    lon: f64,
    elev: f64,
    normal: Vector3<f64>,
    object_close: bool,
}

fn find_normal(shape: &EarthShape, lat: f64, lon: f64, terrain: &Terrain) -> Vector3<f64> {
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

pub fn gen_terrain_cache(params: &Params, terrain: &Terrain, x: u16) -> Vec<TerrainData> {
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
        let normal = find_normal(&params.env.shape, lat, lon, terrain);
        let object_close = params
            .scene
            .objects
            .iter()
            .any(|obj| obj.is_close(&params.env.shape, params.simulation_step, lat, lon));
        result.push(TerrainData {
            lat,
            lon,
            elev: terrain.get_elev(lat, lon).unwrap_or(0.0),
            normal,
            object_close,
        });
    }

    result
}

struct TracingState {
    terrain_data: TerrainData,
    ray_elev: f64,
    dist: f64,
    path_len: f64,
}

impl TracingState {
    fn new(terrain_data: TerrainData, ray_elev: f64, dist: f64, path_len: f64) -> Self {
        Self {
            terrain_data,
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
                object_close: false,
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

pub fn get_single_pixel(
    terrain_cache: &[TerrainData],
    path_cache: &[PathElem],
    objects: &[Object],
    earth_shape: &EarthShape,
) -> Vec<ResultPixel> {
    let mut old_tracing_state = TracingState::new(terrain_cache[0], path_cache[0].elev, 0.0, 0.0);
    let mut result = vec![];

    for (terrain_data, path_elem) in terrain_cache.iter().zip(path_cache).skip(1) {
        let mut finish = false;
        let mut step_result = vec![];
        let new_tracing_state = TracingState::new(
            *terrain_data,
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
        if new_tracing_state.terrain_data.object_close
            || old_tracing_state.terrain_data.object_close
        {
            for object in objects {
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

const DEGREE_DISTANCE: f64 = 111_111.111;

fn get_coords_at_dist(shape: &EarthShape, start: (f64, f64), dir: f64, dist: f64) -> (f64, f64) {
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
