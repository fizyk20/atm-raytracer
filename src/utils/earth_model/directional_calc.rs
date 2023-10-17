use nalgebra::Vector3;

use super::{spherical_directions, DEGREE_DISTANCE};

pub trait DirectionalCalc {
    fn coords_at_dist(&self, dist: f64) -> (f64, f64);
}

pub struct AzEqCalc {
    dir_v: Vector3<f64>,
    pos: Vector3<f64>,
}

impl AzEqCalc {
    pub fn new(dir_v: Vector3<f64>, pos: Vector3<f64>) -> Self {
        Self { dir_v, pos }
    }
}

impl DirectionalCalc for AzEqCalc {
    fn coords_at_dist(&self, dist: f64) -> (f64, f64) {
        let pos2 = self.pos + self.dir_v * dist;
        let lon = pos2.y.atan2(pos2.x).to_degrees();
        let r = (pos2.x * pos2.x + pos2.y * pos2.y).sqrt();
        let lat = 90.0 - r / DEGREE_DISTANCE;
        (lat, lon)
    }
}

pub struct FlDsCalc {
    start: (f64, f64),
    dir: f64,
}

impl FlDsCalc {
    pub fn new(start: (f64, f64), dir: f64) -> Self {
        Self { start, dir }
    }
}

impl DirectionalCalc for FlDsCalc {
    fn coords_at_dist(&self, dist: f64) -> (f64, f64) {
        let d_lat = self.dir.to_radians().cos() * dist / DEGREE_DISTANCE;
        let d_lon =
            self.dir.to_radians().sin() * dist / DEGREE_DISTANCE / self.start.0.to_radians().cos();
        (self.start.0 + d_lat, self.start.1 + d_lon)
    }
}

pub struct SphericalCalc {
    radius: f64,
    pos: Vector3<f64>,
    dir: Vector3<f64>,
}

impl SphericalCalc {
    pub fn new(radius: f64, start: (f64, f64), dir: f64) -> Self {
        let (dirn, dire, pos) = spherical_directions(start.0, start.1);

        // vector tangent to Earth's surface in the given direction
        let dir_rad = dir.to_radians();
        let sindir = dir_rad.sin();
        let cosdir = dir_rad.cos();

        let dir = dirn * cosdir + dire * sindir;

        Self { radius, pos, dir }
    }
}

impl DirectionalCalc for SphericalCalc {
    fn coords_at_dist(&self, dist: f64) -> (f64, f64) {
        let ang = dist / self.radius;

        // final_pos = pos*cos(ang) + dir*sin(ang)
        let sinang = ang.sin();
        let cosang = ang.cos();

        let fpos = self.pos * cosang + self.dir * sinang;

        let final_lat_rad = fpos[2].asin();
        let final_lon_rad = fpos[1].atan2(fpos[0]);

        (final_lat_rad.to_degrees(), final_lon_rad.to_degrees())
    }
}
