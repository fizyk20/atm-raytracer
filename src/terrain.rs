use dted::{read_dted, read_dted_header, DtedData};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

enum TerrainData {
    Loaded(DtedData),
    Pending(PathBuf),
}

impl TerrainData {
    fn get_elev(&mut self, latitude: f64, longitude: f64) -> Option<f64> {
        match self {
            TerrainData::Loaded(data) => data.get_elev(latitude, longitude),
            TerrainData::Pending(path) => {
                println!("Lazy loading terrain file: {:?}", path);
                let data = read_dted(path).expect("Couldn't read a DTED file");
                let result = data.get_elev(latitude, longitude);
                *self = TerrainData::Loaded(data);
                result
            }
        }
    }
}

pub struct Terrain {
    data: HashMap<(i16, i16), RwLock<TerrainData>>,
}

impl Terrain {
    pub fn new() -> Self {
        Terrain {
            data: HashMap::new(),
        }
    }

    pub fn from_folder<P: AsRef<Path>>(terrain_folder: P) -> Self {
        let mut terrain = Self::new();
        let mut files = 0;

        for dir_entry in
            fs::read_dir(terrain_folder).expect("Error opening the terrain data directory")
        {
            let file_path = dir_entry
                .expect("Error reading an entry in the terrain directory")
                .path();
            files += 1;
            terrain.buffer_dted(file_path);
        }

        println!("Detected {} terrain files", files);

        terrain
    }

    pub fn buffer_dted(&mut self, path: PathBuf) {
        let header = read_dted_header(&path).expect("Couldn't read a DTED file");
        let lat = f64::from(header.origin_lat) as i16;
        let lon = f64::from(header.origin_lon) as i16;
        let _ = self
            .data
            .insert((lat, lon), RwLock::new(TerrainData::Pending(path)));
    }

    pub fn get_elev(&self, latitude: f64, longitude: f64) -> Option<f64> {
        let lat = latitude.floor() as i16;
        let lon = longitude.floor() as i16;
        self.data
            .get(&(lat, lon))
            .and_then(|data| data.write().unwrap().get_elev(latitude, longitude))
    }
}
