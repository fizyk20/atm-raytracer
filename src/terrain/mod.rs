mod geotiff;
mod tile;

use dted::{read_dted, read_dted_header};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

use self::geotiff::GeoTiffWrapper;
pub use self::tile::Tile;

type TileObj = Box<dyn Tile + Send + Sync>;

enum TerrainDataInner {
    Loaded(TileObj),
    Pending(PathBuf),
}

impl TerrainDataInner {
    fn read_tile(path: &PathBuf) -> Option<TileObj> {
        if let Ok(dted_obj) = read_dted(path) {
            return Some(Box::new(dted_obj));
        } else if let Some(geotiff_obj) = GeoTiffWrapper::from_path(path) {
            return Some(Box::new(geotiff_obj));
        }
        None
    }
}

struct TerrainData(RwLock<TerrainDataInner>);

impl TerrainData {
    fn get_elev(&self, latitude: f64, longitude: f64) -> Option<f64> {
        if let TerrainDataInner::Loaded(data) = &*self.0.read().unwrap() {
            return data.get_elev(latitude, longitude);
        }

        let mut inner = self.0.write().unwrap();
        if let TerrainDataInner::Pending(path) = &*inner {
            println!("Lazy loading terrain file: {:?}", path);
            let data = TerrainDataInner::read_tile(path)
                .unwrap_or_else(|| panic!("Couldn't read a terrain file {:?}", path));
            let result = data.get_elev(latitude, longitude);
            *inner = TerrainDataInner::Loaded(data);
            return result;
        }

        unreachable!()
    }
}

pub struct Terrain {
    data: HashMap<(i16, i16), TerrainData>,
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
            terrain.buffer_file(file_path);
        }

        println!("Detected {} terrain files", files);

        terrain
    }

    fn buffer_dted(&mut self, path: PathBuf) -> bool {
        let header = if let Ok(hdr) = read_dted_header(&path) {
            hdr
        } else {
            return false;
        };
        let lat = f64::from(header.origin_lat) as i16;
        let lon = f64::from(header.origin_lon) as i16;
        let _ = self.data.insert(
            (lat, lon),
            TerrainData(RwLock::new(TerrainDataInner::Pending(path))),
        );
        true
    }

    fn buffer_geotiff(&mut self, path: PathBuf) -> bool {
        let (lat, lon) = if let Some(coords) = GeoTiffWrapper::coords_from_name(&path) {
            coords
        } else {
            return false;
        };
        let _ = self.data.insert(
            (lat, lon),
            TerrainData(RwLock::new(TerrainDataInner::Pending(path))),
        );
        true
    }

    pub fn buffer_file(&mut self, path: PathBuf) {
        if self.buffer_dted(path.clone()) || self.buffer_geotiff(path.clone()) {
            return;
        }
        panic!("Could not buffer terrain file {:?}", path);
    }

    pub fn get_elev(&self, latitude: f64, longitude: f64) -> Option<f64> {
        let lat = latitude.floor() as i16;
        let lon = longitude.floor() as i16;
        self.data
            .get(&(lat, lon))
            .and_then(|data| data.get_elev(latitude, longitude))
    }
}
