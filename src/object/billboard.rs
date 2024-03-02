use crate::utils::{Coords, EarthModel};

use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

use super::{Color, Image, Object};

#[derive(Clone, Serialize, Deserialize)]
pub struct Billboard {
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) texture: Image,
    pub(super) position: Coords,
}

impl Object for Billboard {
    fn check_collision(
        &self,
        earth_model: &EarthModel,
        point1: Coords,
        point2: Coords,
    ) -> Vec<(f64, Vector3<f64>, Color)> {
        let pos1 = earth_model.as_cartesian(&point1);
        let pos2 = earth_model.as_cartesian(&point2);
        let obj_pos = earth_model.as_cartesian(&self.position);

        let ray = pos2 - pos1;
        let up = earth_model
            .world_directions(self.position.lat, self.position.lon)
            .2;
        let right = ray.cross(&up);
        let right_len = right.dot(&right).sqrt();
        let right = right / right_len;
        let front = right.cross(&up);

        let p1 = pos1 - obj_pos;

        let prop = -p1.dot(&front) / ray.dot(&front);

        if !(0.0..1.0).contains(&prop) {
            // intersection outside of the current interval
            return vec![];
        }

        let intersection = p1 + ray * prop;
        let y = intersection.dot(&up);
        let x = intersection.dot(&right);

        if !(0.0..self.height).contains(&y) || !(-self.width / 2.0..self.width / 2.0).contains(&x) {
            // intersection outside of the rectangle
            return vec![];
        }

        let x = (x + self.width / 2.0) / self.width;
        let y = y / self.height;
        let pixel = self.texture.get_pixel(x, y);

        let color = Color {
            r: pixel.0[0] as f64 / 255.0,
            g: pixel.0[1] as f64 / 255.0,
            b: pixel.0[2] as f64 / 255.0,
            a: pixel.0[3] as f64 / 255.0,
        };

        vec![(prop, front, color)]
    }

    fn is_close(&self, earth_model: &EarthModel, sim_step: f64, lat: f64, lon: f64) -> bool {
        let obj_pos = earth_model.as_cartesian(&self.position);
        let pos = earth_model.as_cartesian(&Coords {
            lat,
            lon,
            elev: self.position.elev,
        });
        let dist_v = pos - obj_pos;

        dist_v.dot(&dist_v) < 2.0 * (self.width + sim_step) * (self.width + sim_step)
    }
}
