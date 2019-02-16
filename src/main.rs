mod generate;
mod params;
mod terrain;

use crate::generate::get_single_pixel;
use crate::terrain::Terrain;
use std::env;
use std::fs;

fn main() {
    let params = params::parse_params();

    let mut terrain = Terrain::new();
    let mut terrain_folder = env::current_dir().unwrap();
    terrain_folder.push(&params.terrain_folder);

    for dir_entry in fs::read_dir(terrain_folder).expect("Error opening the terrain data directory")
    {
        let file_path = dir_entry
            .expect("Error reading an entry in the terrain directory")
            .path();
        println!("Loading terrain file: {:?}", file_path);
        terrain.load_dted(&file_path);
    }

    println!("Test: {:?}", get_single_pixel(&params, &terrain, 320, 140));
}
