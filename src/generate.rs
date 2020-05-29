use crate::params::Params;
use crate::terrain::Terrain;
use atm_refraction::{EarthShape, RayState};

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

pub fn get_single_pixel(
    params: &Params,
    terrain: &Terrain,
    x: u16,
    path_cache: &[RayState],
) -> Option<ResultPixel> {
    let ray_dir = get_ray_dir(params, x);

    let mut elev = terrain
        .get_elev(
            params.view.position.latitude,
            params.view.position.longitude,
        )
        .unwrap_or(0.0);
    let mut dist = 0.0;
    let mut ray_elev = params.view.position.altitude.abs(
        terrain,
        params.view.position.latitude,
        params.view.position.longitude,
    );

    for ray_state in path_cache {
        let (lat, lon) = get_coords_at_dist(params, ray_dir, ray_state.x);
        let new_elev = terrain.get_elev(lat, lon).unwrap_or(0.0);
        if ray_state.h < new_elev {
            let diff1 = ray_elev - elev;
            let diff2 = ray_state.h - new_elev;
            let prop = diff1 / (diff1 - diff2);
            let distance = dist + (ray_state.x - dist) * prop;
            let (lat, lon) = get_coords_at_dist(params, ray_dir, distance);
            return Some(ResultPixel {
                lat,
                lon,
                distance,
                elevation: elev + (new_elev - elev) * prop,
            });
        }
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

            let pos_x = lat_rad.cos() * lon_rad.cos();
            let pos_y = lat_rad.cos() * lon_rad.sin();
            let pos_z = lat_rad.sin();

            // vector tangent to Earth's surface pointing north
            let dirn_x = -lat_rad.sin() * lon_rad.cos();
            let dirn_y = -lat_rad.sin() * lon_rad.sin();
            let dirn_z = lat_rad.cos();

            // vector tangent to Earth's surface pointing east
            let dire_x = -lon_rad.sin();
            let dire_y = lon_rad.cos();
            let dire_z = 0.0f64;

            // vector tangent to Earth's surface in the given direction
            let dir_rad = dir.to_radians();
            let dir_x = dirn_x * dir_rad.cos() + dire_x * dir_rad.sin();
            let dir_y = dirn_y * dir_rad.cos() + dire_y * dir_rad.sin();
            let dir_z = dirn_z * dir_rad.cos() + dire_z * dir_rad.sin();

            // final_pos = pos*cos(ang) + dir*sin(ang)
            let fpos_x = pos_x * ang.cos() + dir_x * ang.sin();
            let fpos_y = pos_y * ang.cos() + dir_y * ang.sin();
            let fpos_z = pos_z * ang.cos() + dir_z * ang.sin();

            let final_lat_rad = fpos_z.asin();
            let final_lon_rad = fpos_y.atan2(fpos_x);

            (final_lat_rad.to_degrees(), final_lon_rad.to_degrees())
        }
    }
}
