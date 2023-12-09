mod directional_calc;

pub use directional_calc::DirectionalCalc;
use directional_calc::{AzEqCalc, EllipsoidCalc, FlDsCalc, SphericalCalc};

use atm_refraction::EarthShape;
use nalgebra::Vector3;
use serde_derive::{Deserialize, Serialize};

use super::Coords;

const DEGREE_DISTANCE: f64 = 10_000_000.0 / 90.0;

const WGS84_A: f64 = 6378137.0;
const WGS84_B: f64 = 6356752.314245;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EarthModel {
    Spherical { radius: f64 },
    Ellipsoid { a: f64, b: f64 },
    Wgs84,
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
            EarthModel::Spherical { .. } | EarthModel::Ellipsoid { .. } | EarthModel::Wgs84 => {
                spherical_directions(lat, lon)
            }
        }
    }

    pub fn as_cartesian(&self, coords: &Coords) -> Vector3<f64> {
        match *self {
            EarthModel::Spherical { radius } => {
                spherical_to_cartesian(radius + coords.elev, coords.lat, coords.lon)
            }
            EarthModel::Wgs84 => EarthModel::Ellipsoid {
                a: WGS84_A,
                b: WGS84_B,
            }
            .as_cartesian(coords),
            EarthModel::Ellipsoid { a, b } => {
                let e2 = 1.0 - (b * b) / (a * a);
                let lat = coords.lat.to_radians();
                let lon = coords.lon.to_radians();
                let n = a / (1.0 - e2 * lat.sin().powi(2)).sqrt();
                let x = (n + coords.elev) * lat.cos() * lon.cos();
                let y = (n + coords.elev) * lat.cos() * lon.sin();
                let z = (n * (1.0 - e2) + coords.elev) * lat.sin();
                Vector3::new(x, y, z)
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
            EarthModel::Wgs84 => EarthModel::Ellipsoid {
                a: WGS84_A,
                b: WGS84_B,
            }
            .to_shape(),
            EarthModel::Ellipsoid { a, b } => EarthShape::Spherical {
                radius: (2.0 * a + b) / 3.0,
            },
            EarthModel::AzimuthalEquidistant
            | EarthModel::FlatDistorted
            | EarthModel::FlatSpherical { .. } => EarthShape::Flat,
        }
    }

    pub fn coords_at_dist_calc(&self, start: (f64, f64), dir: f64) -> Box<dyn DirectionalCalc> {
        match self {
            EarthModel::AzimuthalEquidistant => {
                let pos = self.as_cartesian(&Coords {
                    lat: start.0,
                    lon: start.1,
                    elev: 0.0,
                });
                let (vec_n, vec_e, _) = self.world_directions(start.0, start.1);
                let dir_v = vec_n * dir.to_radians().cos() + vec_e * dir.to_radians().sin();
                Box::new(AzEqCalc::new(dir_v, pos))
            }
            EarthModel::FlatDistorted => Box::new(FlDsCalc::new(start, dir)),
            EarthModel::FlatSpherical { radius } | EarthModel::Spherical { radius } => {
                Box::new(SphericalCalc::new(*radius, start, dir))
            }
            EarthModel::Ellipsoid { a, b } => Box::new(EllipsoidCalc::new(*a, *b, start, dir)),
            EarthModel::Wgs84 => EarthModel::Ellipsoid {
                a: WGS84_A,
                b: WGS84_B,
            }
            .coords_at_dist_calc(start, dir),
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
