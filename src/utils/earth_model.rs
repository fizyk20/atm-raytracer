use atm_refraction::EarthShape;
use nalgebra::Vector3;
use serde_derive::{Deserialize, Serialize};

use super::Coords;

const DEGREE_DISTANCE: f64 = 10_000_000.0 / 90.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EarthModel {
    Spherical { radius: f64 },
    AzimuthalEquidistant,
    FlatDistorted,
    FlatSpherical { radius: f64 },
}

impl EarthModel {
    pub fn world_directions(
        &self,
        lat: f64,
        lon: f64,
    ) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
        match self {
            EarthModel::AzimuthalEquidistant
            | EarthModel::FlatDistorted
            | EarthModel::FlatSpherical { .. } => {
                let lon_rad = lon.to_radians();

                let sinlon = lon_rad.sin();
                let coslon = lon_rad.cos();

                (
                    Vector3::new(-coslon, -sinlon, 0.0),
                    Vector3::new(-sinlon, coslon, 0.0),
                    Vector3::new(0.0, 0.0, 1.0),
                )
            }
            EarthModel::Spherical { .. } => spherical_directions(lat, lon),
        }
    }

    pub fn as_cartesian(&self, coords: &Coords) -> Vector3<f64> {
        match *self {
            EarthModel::Spherical { radius } => {
                spherical_to_cartesian(radius + coords.elev, coords.lat, coords.lon)
            }
            EarthModel::AzimuthalEquidistant
            | EarthModel::FlatDistorted
            | EarthModel::FlatSpherical { .. } => {
                let z = coords.elev;
                let r = (90.0 - coords.lat) * DEGREE_DISTANCE;
                let x = r * coords.lon.to_radians().cos();
                let y = r * coords.lon.to_radians().sin();
                Vector3::new(x, y, z)
            }
        }
    }

    pub fn to_shape(self) -> EarthShape {
        match self {
            EarthModel::Spherical { radius } => EarthShape::Spherical { radius },
            EarthModel::AzimuthalEquidistant
            | EarthModel::FlatDistorted
            | EarthModel::FlatSpherical { .. } => EarthShape::Flat,
        }
    }

    pub fn get_coords_at_dist(&self, start: (f64, f64), dir: f64, dist: f64) -> (f64, f64) {
        match self {
            EarthModel::AzimuthalEquidistant => {
                let pos = self.as_cartesian(&Coords {
                    lat: start.0,
                    lon: start.1,
                    elev: 0.0,
                });
                let (vec_n, vec_e, _) = self.world_directions(start.0, start.1);
                let dir_v = vec_n * dir.to_radians().cos() + vec_e * dir.to_radians().sin();
                let pos2 = pos + dir_v * dist;
                let lon = pos2.y.atan2(pos2.x).to_degrees();
                let r = (pos2.x * pos2.x + pos2.y * pos2.y).sqrt();
                let lat = 90.0 - r / DEGREE_DISTANCE;
                (lat, lon)
            }
            EarthModel::FlatDistorted => {
                let d_lat = dir.to_radians().cos() * dist / DEGREE_DISTANCE;
                let d_lon =
                    dir.to_radians().sin() * dist / DEGREE_DISTANCE / start.0.to_radians().cos();
                (start.0 + d_lat, start.1 + d_lon)
            }
            EarthModel::FlatSpherical { radius } | EarthModel::Spherical { radius } => {
                let ang = dist / radius;

                let (dirn, dire, pos) = spherical_directions(start.0, start.1);

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
}

pub fn spherical_to_cartesian(r: f64, lat: f64, lon: f64) -> Vector3<f64> {
    let x = r * lat.to_radians().cos() * lon.to_radians().cos();
    let y = r * lat.to_radians().cos() * lon.to_radians().sin();
    let z = r * lat.to_radians().sin();
    Vector3::new(x, y, z)
}

fn spherical_directions(lat: f64, lon: f64) -> (Vector3<f64>, Vector3<f64>, Vector3<f64>) {
    let lat_rad = lat.to_radians();
    let lon_rad = lon.to_radians();

    let sinlon = lon_rad.sin();
    let coslon = lon_rad.cos();
    let sinlat = lat_rad.sin();
    let coslat = lat_rad.cos();

    // up direction
    let dirup = Vector3::new(coslat * coslon, coslat * sinlon, sinlat);
    // vector tangent to Earth's surface pointing north
    let dirn = Vector3::new(-sinlat * coslon, -sinlat * sinlon, coslat);
    // vector tangent to Earth's surface pointing east
    let dire = Vector3::new(-sinlon, coslon, 0.0);

    (dirn, dire, dirup)
}
