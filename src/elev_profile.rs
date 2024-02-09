use std::env;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

use crate::terrain::Terrain;

pub const SUBCOMMAND: &str = "output-elev-profile";

pub fn run(matches: &ArgMatches<'_>) -> Result<(), String> {
    let filename = matches
        .value_of("input")
        .expect("please provide an input file");

    let azim: f64 = matches
        .value_of("azim")
        .unwrap_or("0.0")
        .parse()
        .expect("please provide a valid azimuth");

    let step: f64 = matches
        .value_of("step")
        .unwrap_or("50.0")
        .parse()
        .expect("please provide a valid step size");

    let cutoff: f64 = matches
        .value_of("cutoff_dist")
        .unwrap_or("10000.0")
        .parse()
        .expect("please provide a valid cutoff distance");

    assert!(step > 0.0, "step must be positive");

    let config = crate::generator::params::parse_config(filename);

    let mut terrain_folder = env::current_dir().unwrap();
    terrain_folder.push(config.terrain_folder());

    let terrain = Terrain::from_folder(terrain_folder);

    let params = config.into_params(&terrain);

    let dist_calc = params.model.coords_at_dist_calc(
        (
            params.view.position.latitude,
            params.view.position.longitude,
        ),
        azim,
    );

    let mut points = vec![];

    let mut x = 0.0;

    while x <= cutoff {
        let coords = dist_calc.coords_at_dist(x);
        let elev = terrain.get_elev(coords.0, coords.1).unwrap_or(0.0);
        points.push((x, elev));
        x += step;
    }

    for point in points {
        println!("{}\t{}", point.0, point.1);
    }

    Ok(())
}

pub fn subcommand_def() -> App<'static, 'static> {
    SubCommand::with_name(SUBCOMMAND)
        .about("Output elevation profile")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::with_name("input")
                .help("Path to the input file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("azim")
                .short("a")
                .long("azim")
                .value_name("DEGREES")
                .help("Azimuth along which the profile will be generated")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("step")
                .short("s")
                .long("step")
                .value_name("METERS")
                .help(
                    "The interval between points in the output \
                    (default: 50.0)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cutoff_dist")
                .short("c")
                .long("cutoff-dist")
                .value_name("METERS")
                .help(
                    "The length of the elevation profile path (max distance from the observer) \
                    (default: 10000)",
                )
                .takes_value(true),
        )
}
