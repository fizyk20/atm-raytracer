use dted::{read_dted, DtedData};
use std::{collections::HashMap, fs, path::Path};

pub struct Terrain {
    data: HashMap<(i16, i16), DtedData>,
}

impl Terrain {
    pub fn new() -> Self {
        Terrain {
            data: HashMap::new(),
        }
    }

    pub fn from_folder<P: AsRef<Path>>(terrain_folder: P) -> Self {
        let mut terrain = Self::new();

        for dir_entry in
            fs::read_dir(terrain_folder).expect("Error opening the terrain data directory")
        {
            let file_path = dir_entry
                .expect("Error reading an entry in the terrain directory")
                .path();
            println!("Loading terrain file: {:?}", file_path);
            terrain.load_dted(&file_path);
        }

        terrain
    }

    pub fn load_dted<P: AsRef<Path>>(&mut self, path: P) {
        let data = read_dted(path).expect("Couldn't read a DTED file");
        let lat = data.min_lat() as i16;
        let lon = data.min_lon() as i16;
        let _ = self.data.insert((lat, lon), data);
    }

    pub fn get_elev(&self, latitude: f64, longitude: f64) -> Option<f64> {
        let lat = latitude.floor() as i16;
        let lon = longitude.floor() as i16;
        self.data
            .get(&(lat, lon))
            .and_then(|data| data.get_elev(latitude, longitude))
    }
}
