mod coloring;
mod generator;
mod object;
mod renderer;
mod terrain;
mod utils;
mod viewer;

#[macro_use]
extern crate serde_derive;

use clap::{crate_version, App};

fn main() {
    let matches = App::new("Atmospheric Panorama Raytracer")
        .version(crate_version!())
        .subcommand(generator::subcommand_def())
        .subcommand(viewer::subcommand_def())
        .get_matches();

    let result = match matches.subcommand() {
        (generator::SUBCOMMAND, Some(matches)) => generator::generate(matches),
        (viewer::SUBCOMMAND, Some(matches)) => viewer::run(matches),
        _ => panic!("Unknown subcommand!"),
    };

    if let Err(err) = result {
        println!("ERROR: {}", err);
    }
}
