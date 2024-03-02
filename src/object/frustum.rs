use crate::utils::{Coords, EarthModel};

use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

use super::{Color, Object};

#[derive(Clone, Serialize, Deserialize)]
pub struct Frustum {
    pub(super) r1: f64,
    pub(super) r2: f64,
    pub(super) height: f64,
    pub(super) position: Coords,
    pub(super) color: Color,
}

impl Object for Frustum {
    fn check_collision(
        &self,
        earth_model: &EarthModel,
        point1: Coords,
        point2: Coords,
    ) -> Vec<(f64, Vector3<f64>, Color)> {
        let pos1 = earth_model.as_cartesian(&point1);
        let pos2 = earth_model.as_cartesian(&point2);
        let obj_pos = earth_model.as_cartesian(&self.position);

        let p1 = pos1 - obj_pos;
        let p1sq = p1.dot(&p1);

        let v = earth_model
            .world_directions(self.position.lat, self.position.lon)
            .2;

        let w = pos2 - pos1;

        let wsq = w.dot(&w);
        let p1v = p1.dot(&v);
        let p1w = p1.dot(&w);
        let wv = w.dot(&v);
        let aa = (self.r2 - self.r1) / self.height;
        let aa1 = 1.0 + aa * aa;

        let a = wsq - wv * wv * (1.0 + aa * aa);
        let b = 2.0 * (p1w - wv * (p1v * aa1 + aa * self.r1));
        let c = p1sq - p1v * p1v * aa1 - self.r1 * self.r1 - 2.0 * aa * self.r1 * p1v;

        let delta = b * b - 4.0 * a * c;

        let mut results = vec![];

        // Side surface
        if delta >= 0.0 {
            let x1 = (-b - delta.sqrt()) / 2.0 / a;
            let x2 = (-b + delta.sqrt()) / 2.0 / a;
            let (x1, x2) = if a < 0.0 { (x2, x1) } else { (x1, x2) };

            let mut tmp = vec![];

            if (0.0..1.0).contains(&x1) {
                tmp.push(x1);
            }
            if (0.0..1.0).contains(&x2) {
                tmp.push(x2);
            }

            for x in tmp {
                let intersection = p1 + w * x;

                let h = intersection.dot(&v);

                if !(0.0..self.height).contains(&h) {
                    continue;
                }

                let outward = intersection - h * v;

                let o_len = outward.dot(&outward).sqrt();
                let outward = outward / o_len;

                let ang = (self.r1 - self.r2).atan2(self.height);
                let normal = outward * ang.cos() + v * ang.sin();

                results.push((x, normal, self.color));
            }
        }

        // top and bottom
        for (h, r, n) in [(0.0, self.r1, -v), (self.height, self.r2, v)] {
            let x = (h - p1v) / wv;
            let out = p1 + w * x - h * v;
            let d = out.dot(&out);
            if d < r * r && (0.0..1.0).contains(&x) {
                results.push((x, n, self.color));
            }
        }

        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        results
    }

    fn is_close(&self, earth_model: &EarthModel, sim_step: f64, lat: f64, lon: f64) -> bool {
        let obj_pos = earth_model.as_cartesian(&self.position);
        let pos = earth_model.as_cartesian(&Coords {
            lat,
            lon,
            elev: self.position.elev,
        });
        let dist_v = pos - obj_pos;
        let r = self.r1.max(self.r2);

        dist_v.dot(&dist_v) < 2.0 * (r + sim_step) * (r + sim_step)
    }
}
