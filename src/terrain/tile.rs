use dted::DtedData;

pub trait Tile {
    fn min_latitude(&self) -> f64;
    fn max_latitude(&self) -> f64;
    fn min_longitude(&self) -> f64;
    fn max_longitude(&self) -> f64;
    fn get_elev(&self, lat: f64, lon: f64) -> Option<f64>;
}

impl Tile for DtedData {
    fn min_latitude(&self) -> f64 {
        self.min_lat()
    }

    fn max_latitude(&self) -> f64 {
        self.max_lat()
    }

    fn min_longitude(&self) -> f64 {
        self.min_lon()
    }

    fn max_longitude(&self) -> f64 {
        self.max_lon()
    }

    fn get_elev(&self, lat: f64, lon: f64) -> Option<f64> {
        self.get_elev(lat, lon)
    }
}
