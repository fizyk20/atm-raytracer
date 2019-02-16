use dted::{read_dted, DtedData};
use std::path::Path;

pub struct Terrain {
    data: Vec<DtedData>,
}

impl Terrain {
    pub fn new() -> Self {
        Terrain { data: vec![] }
    }

    pub fn load_dted<P: AsRef<Path>>(&mut self, path: P) {
        let data = read_dted(path).expect("Couldn't read a DTED file");
        self.data.push(data);
    }

    pub fn get_elev(&self, latitude: f64, longitude: f64) -> Option<f64> {
        self.data
            .iter()
            .filter_map(|data| data.get_elev(latitude, longitude))
            .next()
    }
}
