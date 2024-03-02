use atm_refraction::{air::Atmosphere, Environment};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

pub const SUBCOMMAND: &str = "output-ray-paths";

pub fn run(matches: &ArgMatches<'_>) -> Result<(), String> {
    let filename = matches
        .value_of("input")
        .expect("please provide an input file");

    let height = matches
        .value_of("height")
        .unwrap_or("2.0")
        .parse()
        .expect("please provide a valid observer height");

    let min_ang = matches
        .value_of("min_angle")
        .unwrap_or("-1.0")
        .parse()
        .expect("please provide a valid minimum altitude");

    let max_ang = matches
        .value_of("max_angle")
        .unwrap_or("1.0")
        .parse()
        .expect("please provide a valid maximum altitude");

    let step: f64 = matches
        .value_of("angle_step")
        .unwrap_or("0.1")
        .parse()
        .expect("please provide a valid step size");

    let ray_step: f64 = matches
        .value_of("ray_step")
        .unwrap_or("50.0")
        .parse()
        .expect("please provide a valid ray step size");

    let cutoff: f64 = matches
        .value_of("cutoff_dist")
        .unwrap_or("10000.0")
        .parse()
        .expect("please provide a valid cutoff distance");

    let output_step: f64 = matches
        .value_of("output_step")
        .unwrap_or("50.0")
        .parse()
        .expect("please provide a valid output step");

    assert!(step > 0.0, "step must be positive");

    let config = crate::generator::params::parse_config(filename);

    let atmosphere = Atmosphere::from_def(config.atmosphere.clone());

    let env = Environment {
        shape: config.earth_shape.to_shape(),
        atmosphere,
        wavelength: config.wavelength,
    };

    let mut ang: f64 = min_ang;
    let mut rays = vec![];
    let mut xs = vec![0.0];

    while ang <= max_ang {
        eprintln!("Elevation angle {} (min={}, max={})", ang, min_ang, max_ang);
        let mut stepper = env.cast_ray_stepper(height, ang.to_radians(), false);
        stepper.set_step_size(ray_step);

        let mut ray = vec![height];

        loop {
            let ray_state = stepper.next().unwrap();
            let x = ray_state.x;
            let y = ray_state.h;
            if ((x - ray_step / 2.0) / output_step).floor()
                != ((x + ray_step / 2.0) / output_step).floor()
            {
                if ang == min_ang {
                    xs.push(x);
                }
                ray.push(y);
            }
            if x >= cutoff {
                break;
            }
        }

        rays.push(ray);
        ang += step;
    }

    for i in 0..xs.len() {
        print!("{}\t", xs[i]);
        for ray in &rays {
            print!("{}\t", ray[i]);
        }
        println!();
    }

    Ok(())
}

pub fn subcommand_def() -> App<'static, 'static> {
    SubCommand::with_name(SUBCOMMAND)
        .about("Output ray paths")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::with_name("input")
                .help("Path to the input file")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("METERS")
                .help(
                    "Observer height, in meters (default: 2.0)"
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("min_angle")
                .short("a")
                .long("min-ang")
                .value_name("DEGREES")
                .help(
                    "Lower boundary of the range of elevation angles for which the data should be \
                    output, in degrees (default: -1.0)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max_angle")
                .short("b")
                .long("max-ang")
                .value_name("DEGREES")
                .help(
                    "Upper boundary of the range of elevation angles for which the data should be \
                    output, in degrees (default: 1.0)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("angle_step")
                .short("s")
                .long("angle-step")
                .value_name("DEGREES")
                .help(
                    "The elevation angle difference between two subsequent data points, in degrees \
                    (default: 0.1)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ray_step")
                .short("r")
                .long("ray-step")
                .value_name("METERS")
                .help(
                    "The distance between subsequent points along the ray path in meters \
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
                    "The length of the simulated ray path (max distance from the observer) \
                    (default: 10000)",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output_step")
                .short("o")
                .long("output-step")
                .value_name("METERS")
                .help(
                    "The interval between points in the output \
                    (default: 50.0)",
                )
                .takes_value(true),
        )
}
