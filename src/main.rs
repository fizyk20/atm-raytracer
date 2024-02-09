mod atm_printer;
mod coloring;
mod elev_profile;
mod generator;
mod object;
mod ray_path;
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
        .subcommand(atm_printer::subcommand_def())
        .subcommand(ray_path::subcommand_def())
        .subcommand(elev_profile::subcommand_def())
        .get_matches();

    let result = match matches.subcommand() {
        (generator::SUBCOMMAND, Some(matches)) => generator::generate(matches),
        (viewer::SUBCOMMAND, Some(matches)) => viewer::run(matches),
        (atm_printer::SUBCOMMAND, Some(matches)) => atm_printer::run(matches),
        (ray_path::SUBCOMMAND, Some(matches)) => ray_path::run(matches),
        (elev_profile::SUBCOMMAND, Some(matches)) => elev_profile::run(matches),
        _ => panic!("Unknown subcommand!"),
    };

    if let Err(err) = result {
        println!("ERROR: {}", err);
    }
}
