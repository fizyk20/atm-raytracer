use crate::{params::Params, terrain::Terrain, utils::world_directions};

use atm_refraction::{EarthShape, RayState};
use nalgebra::Vector3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResultPixel {
    pub lat: f64,
    pub lon: f64,
    pub distance: f64,
    pub elevation: f64,
    pub normal: Vector3<f64>,
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

pub fn gen_path_cache(params: &Params, terrain: &Terrain, y: u16) -> Vec<RayState> {
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

    let mut path = vec![];

    loop {
        let ray_state = ray.next().unwrap();
        path.push(ray_state);
        if ray_state.x > params.view.frame.max_distance {
            break;
        }
    }

    path
}

#[derive(Debug, Clone, Copy)]
pub struct TerrainData {
    lat: f64,
    lon: f64,
    elev: f64,
    normal: Vector3<f64>,
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
        result.push(TerrainData {
            lat,
            lon,
            elev: terrain.get_elev(lat, lon).unwrap_or(0.0),
            normal,
        });
    }

    result
}

pub fn get_single_pixel(
    terrain_cache: &[TerrainData],
    path_cache: &[RayState],
) -> Option<ResultPixel> {
    let TerrainData {
        mut lat,
        mut lon,
        mut elev,
        mut normal,
    } = terrain_cache[0];
    let mut dist = 0.0;
    let mut ray_elev = path_cache[0].h;

    for (terrain_data, ray_state) in terrain_cache.into_iter().zip(path_cache).skip(1) {
        if ray_state.h < terrain_data.elev {
            let diff1 = ray_elev - elev;
            let diff2 = ray_state.h - terrain_data.elev;
            let prop = diff1 / (diff1 - diff2);
            let distance = dist + (ray_state.x - dist) * prop;
            let lat = lat + (terrain_data.lat - lat) * prop;
            let lon = lon + (terrain_data.lon - lon) * prop;
            let normal = normal + (terrain_data.normal - normal) * prop;
            return Some(ResultPixel {
                lat,
                lon,
                distance,
                elevation: elev + (terrain_data.elev - elev) * prop,
                normal,
            });
        }
        lat = terrain_data.lat;
        lon = terrain_data.lon;
        elev = terrain_data.elev;
        normal = terrain_data.normal;
        dist = ray_state.x;
        ray_elev = ray_state.h;
    }
    None
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
