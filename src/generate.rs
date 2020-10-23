use crate::{params::Params, terrain::Terrain};

use atm_refraction::{EarthShape, RayState};
use nalgebra::Vector3;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResultPixel {
    pub lat: f64,
    pub lon: f64,
    pub distance: f64,
    pub elevation: f64,
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

pub fn gen_terrain_cache(params: &Params, terrain: &Terrain, x: u16) -> Vec<(f64, f64, f64)> {
    let dir = get_ray_dir(params, x);
    let mut distance = 0.0;

    let mut result = vec![];
    while distance < params.view.frame.max_distance {
        distance += params.simulation_step;
        let (lat, lon) = get_coords_at_dist(params, dir, distance);
        result.push((lat, lon, terrain.get_elev(lat, lon).unwrap_or(0.0)));
    }

    result
}

pub fn get_single_pixel(
    terrain_cache: &[(f64, f64, f64)],
    path_cache: &[RayState],
) -> Option<ResultPixel> {
    let (mut lat, mut lon, mut elev) = terrain_cache[0];
    let mut dist = 0.0;
    let mut ray_elev = path_cache[0].h;

    for (&(new_lat, new_lon, new_elev), ray_state) in
        terrain_cache.into_iter().zip(path_cache).skip(1)
    {
        if ray_state.h < new_elev {
            let diff1 = ray_elev - elev;
            let diff2 = ray_state.h - new_elev;
            let prop = diff1 / (diff1 - diff2);
            let distance = dist + (ray_state.x - dist) * prop;
            let lat = lat + (new_lat - lat) * prop;
            let lon = lon + (new_lon - lon) * prop;
            return Some(ResultPixel {
                lat,
                lon,
                distance,
                elevation: elev + (new_elev - elev) * prop,
            });
        }
        lat = new_lat;
        lon = new_lon;
        elev = new_elev;
        dist = ray_state.x;
        ray_elev = ray_state.h;
    }
    None
}

const DEGREE_DISTANCE: f64 = 111_111.111;

fn get_coords_at_dist(params: &Params, dir: f64, dist: f64) -> (f64, f64) {
    match params.env.shape {
        EarthShape::Flat => {
            let d_lat = dir.to_radians().cos() * dist / DEGREE_DISTANCE;
            let d_lon = dir.to_radians().sin() * dist
                / DEGREE_DISTANCE
                / params.view.position.latitude.to_radians().cos();
            (
                params.view.position.latitude + d_lat,
                params.view.position.longitude + d_lon,
            )
        }
        EarthShape::Spherical { radius } => {
            let ang = dist / radius;

            let lat_rad = params.view.position.latitude.to_radians();
            let lon_rad = params.view.position.longitude.to_radians();

            let sinlon = lon_rad.sin();
            let coslon = lon_rad.cos();
            let sinlat = lat_rad.sin();
            let coslat = lat_rad.cos();

            let pos = Vector3::new(coslat * coslon, coslat * sinlon, sinlat);
            // vector tangent to Earth's surface pointing north
            let dirn = Vector3::new(-sinlat * coslon, -sinlat * sinlon, coslat);
            // vector tangent to Earth's surface pointing east
            let dire = Vector3::new(-sinlon, coslon, 0.0);

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
