use crate::params::Params;
use crate::terrain::Terrain;
use atm_refraction::EarthShape;

#[derive(Debug, Clone, Copy)]
pub struct ResultPixel {
    pub lat: f64,
    pub lon: f64,
    pub distance: f64,
    pub elevation: f64,
}

pub fn get_single_pixel(params: &Params, terrain: &Terrain, x: u16, y: u16) -> Option<ResultPixel> {
    let (ray_elev, ray_dir) = get_ray_dir(params, x, y);
    let mut ray =
        params
            .env
            .cast_ray_stepper(params.viewpoint.alt, deg2rad(ray_elev), params.straight);
    ray.set_step_size(params.step);

    let mut elev = -10.0;
    let mut dist = 0.0;
    let mut ray_elev = params.viewpoint.alt;

    loop {
        let ray_state = ray.next().unwrap();
        if ray_state.x > params.max_dist {
            return None;
        }
        let (lat, lon) = get_coords_at_dist(params, ray_dir, ray_state.x);
        if let Some(new_elev) = terrain.get_elev(lat, lon) {
            if ray_state.x > 1e3 && ray_state.h < new_elev {
                let diff1 = ray_elev - elev;
                let diff2 = ray_state.h - new_elev;
                let diff_dist = ray_state.x - dist;
                let prop = diff1 / (diff1 - diff2);
                return Some(ResultPixel {
                    lat,
                    lon,
                    distance: dist + diff_dist * prop,
                    elevation: elev + (new_elev - elev) * prop,
                });
            }
            elev = new_elev;
            dist = ray_state.x;
            ray_elev = ray_state.h;
        } else {
            return None;
        }
    }
}

fn deg2rad(x: f64) -> f64 {
    x * 3.1415926535897 / 180.0
}

fn rad2deg(x: f64) -> f64 {
    x * 180.0 / 3.1415926535897
}

fn get_ray_dir(params: &Params, x: u16, y: u16) -> (f64, f64) {
    let width = params.pic_width as f64;
    let height = params.pic_height as f64;

    let aspect = width / height;
    let x = (x as i16 - params.pic_width as i16 / 2) as f64 / width;
    let y = (y as i16 - params.pic_height as i16 / 2) as f64 / height;

    let ray_dir = params.viewpoint.dir + x * params.viewpoint.fov;
    let ray_elev = params.viewpoint.elev_bias - y * params.viewpoint.fov / aspect;

    (ray_elev, ray_dir)
}

fn get_coords_at_dist(params: &Params, dir: f64, dist: f64) -> (f64, f64) {
    match params.env.shape {
        EarthShape::Flat => {
            let d_lat = deg2rad(dir).cos() * dist / 111111.111;
            let d_lon =
                deg2rad(dir).sin() * dist / 111111.111 / deg2rad(params.viewpoint.lat).cos();
            (params.viewpoint.lat + d_lat, params.viewpoint.lon + d_lon)
        }
        EarthShape::Spherical { radius } => {
            let ang = dist / radius;

            let lat_rad = deg2rad(params.viewpoint.lat);
            let lon_rad = deg2rad(params.viewpoint.lon);

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
            let dir_rad = deg2rad(dir);
            let dir_x = dirn_x * dir_rad.cos() + dire_x * dir_rad.sin();
            let dir_y = dirn_y * dir_rad.cos() + dire_y * dir_rad.sin();
            let dir_z = dirn_z * dir_rad.cos() + dire_z * dir_rad.sin();

            // final_pos = pos*cos(ang) + dir*sin(ang)
            let fpos_x = pos_x * ang.cos() + dir_x * ang.sin();
            let fpos_y = pos_y * ang.cos() + dir_y * ang.sin();
            let fpos_z = pos_z * ang.cos() + dir_z * ang.sin();

            let final_lat_rad = fpos_z.asin();
            let final_lon_rad = fpos_y.atan2(fpos_x);

            (rad2deg(final_lat_rad), rad2deg(final_lon_rad))
        }
    }
}
