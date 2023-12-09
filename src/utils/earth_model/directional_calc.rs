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

pub struct EllipsoidCalc {
    b: f64,
    f: f64,
    red_lat: f64,
    lon: f64,
    az1: f64,
    alfa: f64,
    sig1: f64,
    cap_a: f64,
    cap_b: f64,
    cap_c: f64,
}

// Implementation according to https://www.ngs.noaa.gov/PUBS_LIB/inverse.pdf

impl EllipsoidCalc {
    pub fn new(a: f64, b: f64, start: (f64, f64), dir: f64) -> Self {
        let lat = start.0.to_radians();
        let lon = start.1.to_radians();
        let az1 = dir.to_radians();

        let f = (a - b) / a;

        let red_lat = ((1.0 - f) * lat.tan()).atan();
        let sig1 = (red_lat.tan() / az1.cos()).atan();
        let alfa = (red_lat.cos() * az1.sin()).asin();

        let u2 = alfa.cos().powi(2) * (a * a - b * b) / (b * b);

        let cap_a = 1.0 + u2 / 256.0 * (64.0 + u2 * (-12.0 + 5.0 * u2));
        let cap_b = u2 / 512.0 * (128.0 + u2 * (-64.0 + 37.0 * u2));
        let cap_c = f / 16.0 * alfa.cos().powi(2) * (4.0 + f * (4.0 - 3.0 * alfa.cos().powi(2)));

        Self {
            b,
            f,
            red_lat,
            lon,
            az1,
            alfa,
            sig1,
            cap_a,
            cap_b,
            cap_c,
        }
    }
}

const EPSILON: f64 = 1e-10; // corresponds to an accuracy of ~0.1 cm

impl DirectionalCalc for EllipsoidCalc {
    fn coords_at_dist(&self, dist: f64) -> (f64, f64) {
        let mut sig = dist / self.b / self.cap_a;

        loop {
            let sigm = 2.0 * self.sig1 + sig;
            let dsig = self.cap_b
                * sig.sin()
                * (sigm.cos() + self.cap_b / 4.0 * sig.cos() * (-1.0 + 2.0 * sigm.cos().powi(2)));
            let new_sig = dist / self.b / self.cap_a + dsig;
            let dsig = (new_sig - sig).abs();
            sig = new_sig;
            if dsig < EPSILON {
                break;
            }
        }

        let sigm = 2.0 * self.sig1 + sig;

        let lat2 = ((self.red_lat.sin() * sig.cos()
            + self.red_lat.cos() * sig.sin() * self.az1.cos())
            / ((1.0 - self.f)
                * (self.alfa.sin().powi(2)
                    + (self.red_lat.sin() * sig.sin()
                        - self.red_lat.cos() * sig.cos() * self.az1.cos())
                    .powi(2))
                .sqrt()))
        .atan();

        let lambda = (sig.sin() * self.az1.sin()
            / (self.red_lat.cos() * sig.cos() - self.red_lat.sin() * sig.sin() * self.az1.cos()))
        .atan();

        let dl = lambda
            - (1.0 - self.cap_c)
                * self.f
                * self.alfa.sin()
                * (sig
                    + self.cap_c
                        * sig.sin()
                        * (sigm.cos()
                            + self.cap_c * sig.cos() * (-1.0 + 2.0 * sigm.cos().powi(2))));

        let lon2 = self.lon + dl;

        (lat2.to_degrees(), lon2.to_degrees())
    }
}
