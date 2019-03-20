use crate::terrain::Terrain;
use atm_refraction::{
    air::{get_atmosphere, us76_atmosphere},
    EarthShape, Environment,
};
use clap::{App, AppSettings, Arg};

#[derive(Clone, Copy)]
pub enum Altitude {
    Absolute(f64),
    Relative(f64),
}

impl Altitude {
    pub fn abs(&self, terrain: &Terrain, lat: f64, lon: f64) -> f64 {
        match *self {
            Altitude::Absolute(x) => x,
            Altitude::Relative(x) => terrain.get_elev(lat, lon).unwrap_or(0.0) + x,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Viewpoint {
    pub lat: f64,
    pub lon: f64,
    pub alt: Altitude,
    pub dir: f64,
    pub fov: f64,
    pub tilt: f64,
}

#[derive(Clone)]
pub struct Params {
    pub terrain_folder: String,
    pub output_file: String,
    pub viewpoint: Viewpoint,
    pub env: Environment,
    pub max_dist: f64,
    pub straight: bool,
    pub step: f64,
    pub pic_width: u16,
    pub pic_height: u16,
}

pub fn parse_params() -> Params {
    let matches = App::new("Atmospheric Panorama Raytracer")
        .version("0.1")
        .setting(AppSettings::AllowLeadingHyphen)
        .arg(
            Arg::with_name("terrain")
                .short("t")
                .long("terrain")
                .value_name("PATH")
                .help("Path to the folder with terrain files (./terrain assumed if none)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("atmosphere")
                .long("atmosphere")
                .value_name("FILE")
                .help("Path to atmosphere config file (US76 atmosphere assumed if none)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("latitude")
                .short("l")
                .long("lat")
                .value_name("DEG")
                .help("Viewpoint latitude in degrees")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("longitude")
                .short("g")
                .long("lon")
                .value_name("DEG")
                .help("Viewpoint longitude in degrees")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("altitude")
                .short("a")
                .long("alt")
                .value_name("ALT")
                .conflicts_with("elevation")
                .required_unless("elevation")
                .help("Viewpoint altitude in meters")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("elevation")
                .short("e")
                .long("elev")
                .value_name("ELEV")
                .conflicts_with("altitude")
                .required_unless("altitude")
                .help("Viewpoint elevation in meters (above the terrain)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("direction")
                .short("d")
                .long("dir")
                .value_name("DEG")
                .help(
                    "Viewing azimuth in degrees (ie. 0 = north, 90 = east, 180 = south, 270 = west)",
                )
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("fov")
                .short("f")
                .long("fov")
                .value_name("DEG")
                .help("Horizontal field of view in degrees (default: 30)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("tilt")
                .short("i")
                .long("tilt")
                .value_name("DEG")
                .help("Observer tilt relative to the horizon in degrees (default: 0)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("max-dist")
                .short("m")
                .long("maxdist")
                .value_name("DIST")
                .help("Cutoff distance in km (default: 150)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("step")
                .long("step")
                .value_name("STEP")
                .help("Light ray propagation step in meters (default: 50)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("radius")
                .short("R")
                .long("radius")
                .value_name("RADIUS")
                .help("Calculate assuming the given value as the Earth's radius, in km (default: 6378) (conflicts with --flat)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("flat")
                .long("flat")
                .help("Simulate a flat Earth (conflicts with --radius)")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("straight")
                .short("s")
                .long("straight")
                .help("Ignore the atmosphere (use straight-line light rays)")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .value_name("FILE")
                .help("File name to save the output image as")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .value_name("PIXELS")
                .help("Output image width in pixels (default: 640)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("PIXELS")
                .help("Output image height in pixels (default: 480)")
                .takes_value(true),
        )
        .get_matches();

    let terrain = matches.value_of("terrain").unwrap_or("./terrain");
    let output = matches.value_of("output").unwrap_or("./output.png");

    let pic_width: u16 = matches
        .value_of("width")
        .unwrap_or("640")
        .parse()
        .ok()
        .expect("Invalid output width");

    let pic_height: u16 = matches
        .value_of("height")
        .unwrap_or("480")
        .parse()
        .ok()
        .expect("Invalid output height");

    let lat: f64 = matches
        .value_of("latitude")
        .expect("Latitude not present")
        .parse()
        .ok()
        .expect("Invalid viewpoint latitude");

    let lon: f64 = matches
        .value_of("longitude")
        .expect("Longitude not present")
        .parse()
        .ok()
        .expect("Invalid viewpoint longitude");

    let alt: Altitude = match (matches.value_of("altitude"), matches.value_of("elevation")) {
        (Some(a), None) => Altitude::Absolute(a.parse().ok().expect("Invalid viewpoint altitude")),
        (None, Some(e)) => Altitude::Relative(e.parse().ok().expect("Invalid viewpoint elevation")),
        _ => unreachable!(),
    };

    let dir: f64 = matches
        .value_of("direction")
        .expect("Direction not present")
        .parse()
        .ok()
        .expect("Invalid viewing azimuth");

    let fov: f64 = matches
        .value_of("fov")
        .unwrap_or("30")
        .parse()
        .ok()
        .expect("Invalid field of view");

    let tilt: f64 = matches
        .value_of("tilt")
        .unwrap_or("0")
        .parse()
        .ok()
        .expect("Invalid view tilt");

    let max_dist: f64 = matches
        .value_of("max-dist")
        .unwrap_or("150")
        .parse()
        .ok()
        .expect("Invalid cutoff distance");
    let max_dist = max_dist * 1e3;

    let step: f64 = matches
        .value_of("step")
        .unwrap_or("50")
        .parse()
        .ok()
        .expect("Invalid step value");

    let viewpoint = Viewpoint {
        lat,
        lon,
        alt,
        dir,
        fov,
        tilt,
    };

    let atmosphere = matches
        .value_of("atmosphere")
        .map(|file| get_atmosphere(&file))
        .unwrap_or_else(us76_atmosphere);

    let shape = match (matches.is_present("flat"), matches.value_of("radius")) {
        (false, None) => EarthShape::Spherical { radius: 6378000.0 },
        (true, None) => EarthShape::Flat,
        (false, Some(radius)) => {
            let r: f64 = radius.parse().ok().expect("Invalid radius passed");
            EarthShape::Spherical { radius: r * 1e3 }
        }
        (true, Some(_)) => panic!("Conflicting Earth shape options chosen!"),
    };

    Params {
        terrain_folder: terrain.to_owned(),
        output_file: output.to_owned(),
        viewpoint,
        env: Environment { shape, atmosphere },
        max_dist,
        step,
        straight: matches.is_present("straight"),
        pic_width,
        pic_height,
    }
}
