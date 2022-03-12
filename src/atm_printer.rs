use atm_refraction::air::Atmosphere;
use clap::{App, Arg, ArgMatches, SubCommand};

pub const SUBCOMMAND: &'static str = "output-atm";

pub fn run(matches: &ArgMatches<'_>) -> Result<(), String> {
    let filename = matches
        .value_of("input")
        .expect("please provide an input file");

    let min_alt = matches
        .value_of("min_altitude")
        .unwrap_or("0.0")
        .parse()
        .expect("please provide a valid minimum altitude");

    let max_alt = matches
        .value_of("max_altitude")
        .unwrap_or("1000.0")
        .parse()
        .expect("please provide a valid maximum altitude");

    let step: f64 = matches
        .value_of("step")
        .unwrap_or("0.2")
        .parse()
        .expect("please provide a valid step size");

    let celsius = matches.is_present("celsius");

    let config = crate::generator::params::parse_config(filename);

    let atmosphere = Atmosphere::from_def(config.atmosphere.clone());

    let mut alt = min_alt;

    while alt <= max_alt {
        println!(
            "{} {} {}",
            alt,
            atmosphere.temperature(alt) - if celsius { 273.15 } else { 0.0 },
            atmosphere.pressure(alt)
        );
        alt += step;
    }

    Ok(())
}

pub fn subcommand_def() -> App<'static, 'static> {
    SubCommand::with_name(SUBCOMMAND)
        .about("Print the atmospheric profile")
        .arg(
            Arg::with_name("input")
                .help("Path to the input file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("min_altitude")
                .short("a")
                .long("min-alt")
                .value_name("ALTITUDE")
                .help(
                    "Lower boundary of the range of altitudes for which the data should be \
                    output, in meters (default: 0)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max_altitude")
                .short("b")
                .long("max-alt")
                .value_name("ALTITUDE")
                .help(
                    "Upper boundary of the range of altitudes for which the data should be \
                    output, in meters (default: 1000)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("step")
                .short("s")
                .long("step")
                .value_name("LENGTH")
                .help(
                    "The altitude difference between two subsequent data points, in meters \
                    (default: 0.2)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("celsius")
                .short("c")
                .long("celsius")
                .help("Use degrees Celsius instead of Kelvins")
                .takes_value(false),
        )
}
