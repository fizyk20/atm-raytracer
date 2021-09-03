mod generators;
pub mod params;

use std::{env, fs, io::Write, time::SystemTime};

use clap::ArgMatches;
use libflate::gzip::Encoder;

use crate::terrain::Terrain;

pub use generators::{
    FastGenerator, Generator, PixelColor, RectilinearGenerator, ResultPixel, TracePoint,
};
pub use params::subcommand_def;
use params::{GeneratorDef, Params};

pub const SUBCOMMAND: &'static str = "gen";

#[derive(Clone, Serialize, Deserialize)]
pub struct AllData {
    pub params: Params,
    pub result: Vec<Vec<ResultPixel>>,
}

fn output_metadata(filename: &str, pixels: Vec<Vec<ResultPixel>>, params: Params) {
    let mut file = fs::File::create(filename).expect("failed to create a metadata file");
    let all_data = AllData {
        params,
        result: pixels,
    };

    let all_data_bytes = bincode::serialize(&all_data).expect("failed to serialize metadata");
    let mut gzip_encoder = Encoder::new(Vec::new()).expect("failed to create a GZip encoder");
    gzip_encoder
        .write_all(&all_data_bytes)
        .expect("failed to deflate metadata");
    let zipped_data = gzip_encoder
        .finish()
        .into_result()
        .expect("failed to finish deflating metadata");

    file.write_all(&zipped_data)
        .expect("failed to write metadata to the file");
}

pub fn generate(matches: &ArgMatches<'_>) -> Result<(), String> {
    let config = match params::parse_config(matches) {
        Ok(config) => config,
        Err(()) => {
            // this indicates that 'output-atm-data' was chosen and data was printed, nothing more
            // to do
            return Ok(());
        }
    };

    let mut terrain_folder = env::current_dir().unwrap();
    terrain_folder.push(config.terrain_folder());

    let start = SystemTime::now();

    println!(
        "{}: Using terrain data directory: {:?}",
        start.elapsed().unwrap().as_secs_f64(),
        terrain_folder
    );

    let terrain = Terrain::from_folder(terrain_folder);

    let params = config.into_params(&terrain);

    let generator: Box<dyn Generator> = match params.output.generator {
        GeneratorDef::Fast => Box::new(FastGenerator::new(&params, &terrain, start)),
        GeneratorDef::Rectilinear => Box::new(RectilinearGenerator::new(&params, &terrain, start)),
    };

    let result_pixels = generator.generate();

    println!(
        "{}: Outputting image...",
        start.elapsed().unwrap().as_secs_f64()
    );
    crate::renderer::output_image(&result_pixels, &params);

    if let Some(ref filename) = params.output.file_metadata {
        println!(
            "{}: Outputting metadata...",
            start.elapsed().unwrap().as_secs_f64()
        );
        output_metadata(filename, result_pixels, params.clone());
    }

    Ok(())
}
