use std::{env, fs::File, io::Read};

use crate::{
    coloring::{ColoringMethod, Shading, SimpleColors},
    object::Object,
    terrain::Terrain,
    utils::world_directions,
};

use atm_refraction::{
    air::{atm_from_str, get_atmosphere, us76_atmosphere},
    EarthShape, Environment,
};
use clap::{App, AppSettings, Arg};
use nalgebra::Vector3;

#[derive(Clone, Copy, Serialize, Deserialize)]
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

    pub fn unwrap(&self) -> f64 {
        match *self {
            Altitude::Absolute(x) => x,
            Altitude::Relative(_) => panic!("unwrapping relative altitude"),
        }
    }

    pub fn convert_into_absolute(&mut self, terrain: &Terrain, lat: f64, lon: f64) {
        match *self {
            Altitude::Absolute(_) => (),
            Altitude::Relative(x) => {
                *self = Altitude::Absolute(terrain.get_elev(lat, lon).unwrap_or(0.0) + x);
            }
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ConfPosition {
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<Altitude>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Altitude,
}

impl ConfPosition {
    pub fn into_position(self) -> Position {
        Position {
            latitude: self.latitude.unwrap_or(0.0),
            longitude: self.longitude.unwrap_or(0.0),
            altitude: self.altitude.unwrap_or(Altitude::Relative(1.0)),
        }
    }
}

impl Default for Position {
    fn default() -> Position {
        Position {
            latitude: 0.0,
            longitude: 0.0,
            altitude: Altitude::Relative(1.0),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Scene {
    pub terrain_folder: String,
    pub objects: Vec<Object>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ConfFrame {
    direction: Option<f64>,
    tilt: Option<f64>,
    fov: Option<f64>,
    max_distance: Option<f64>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Frame {
    pub direction: f64,
    pub tilt: f64,
    pub fov: f64,
    pub max_distance: f64,
}

impl ConfFrame {
    pub fn into_frame(self) -> Frame {
        Frame {
            direction: self.direction.unwrap_or(0.0),
            tilt: self.tilt.unwrap_or(0.0),
            fov: self.fov.unwrap_or(30.0),
            max_distance: self.max_distance.unwrap_or(150_000.0),
        }
    }
}

impl Default for Frame {
    fn default() -> Frame {
        Frame {
            direction: 0.0,
            tilt: 0.0,
            fov: 30.0,
            max_distance: 150_000.0,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ConfColoring {
    Simple {
        water_level: Option<f64>,
    },
    Shading {
        water_level: Option<f64>,
        ambient_light: Option<f64>,
        light_zenith_angle: Option<f64>,
        light_dir: Option<f64>,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Coloring {
    Simple {
        water_level: f64,
        max_distance: f64,
    },
    Shading {
        water_level: f64,
        ambient_light: f64,
        light_dir: Vector3<f64>,
    },
}

impl ConfColoring {
    pub fn into_coloring(
        self,
        frame: &Frame,
        position: &Position,
        earth_shape: &EarthShape,
    ) -> Coloring {
        match self {
            ConfColoring::Simple { water_level } => Coloring::Simple {
                water_level: water_level.unwrap_or(0.0),
                max_distance: frame.max_distance,
            },
            ConfColoring::Shading {
                water_level,
                ambient_light,
                light_zenith_angle,
                light_dir,
            } => {
                let water_level = water_level.unwrap_or(0.0);
                let ambient_light = ambient_light.unwrap_or(0.4);
                let light_zenith_angle = light_zenith_angle.unwrap_or(45.0).to_radians();
                let light_dir = light_dir.unwrap_or(0.0).to_radians();
                let (dir_north, dir_east, dir_up) =
                    world_directions(earth_shape, position.latitude, position.longitude);
                let front_azimuth = frame.direction.to_radians();
                let dir_front = dir_north * front_azimuth.cos() + dir_east * front_azimuth.sin();
                let dir_right = dir_east * front_azimuth.cos() - dir_north * front_azimuth.sin();
                let light_dir = (-dir_front * light_zenith_angle.sin() * light_dir.cos()
                    + dir_right * light_zenith_angle.sin() * light_dir.sin()
                    + dir_up * light_zenith_angle.cos())
                .normalize();
                Coloring::Shading {
                    water_level,
                    ambient_light,
                    light_dir,
                }
            }
        }
    }
}

impl Coloring {
    fn default_coloring(frame: &Frame) -> Coloring {
        Coloring::Simple {
            water_level: 0.0,
            max_distance: frame.max_distance,
        }
    }

    pub fn coloring_method(&self) -> Box<dyn ColoringMethod> {
        match *self {
            Coloring::Simple {
                water_level,
                max_distance,
            } => Box::new(SimpleColors::new(max_distance, water_level)),
            Coloring::Shading {
                water_level,
                ambient_light,
                light_dir,
            } => Box::new(Shading::new(water_level, ambient_light, light_dir)),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ConfView {
    position: Option<ConfPosition>,
    frame: Option<ConfFrame>,
    coloring: Option<ConfColoring>,
    fog_distance: Option<f64>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct View {
    pub position: Position,
    pub frame: Frame,
    pub coloring: Coloring,
    pub fog_distance: Option<f64>,
}

impl ConfView {
    pub fn into_view(self, earth_shape: &EarthShape) -> View {
        let frame = self
            .frame
            .map(ConfFrame::into_frame)
            .unwrap_or_else(Default::default);
        let position = self
            .position
            .map(ConfPosition::into_position)
            .unwrap_or_else(Default::default);
        let coloring = self
            .coloring
            .map(|conf_coloring| conf_coloring.into_coloring(&frame, &position, earth_shape))
            .unwrap_or_else(|| Coloring::default_coloring(&frame));
        View {
            position,
            frame,
            coloring,
            fog_distance: self.fog_distance,
        }
    }
}

impl Default for View {
    fn default() -> Self {
        let frame = Frame::default();
        let position = Position::default();
        let coloring = Coloring::default_coloring(&frame);
        View {
            frame,
            position,
            coloring,
            fog_distance: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Tick {
    Single {
        azimuth: f64,
        size: u32,
        labelled: bool,
    },
    Multiple {
        bias: f64,
        step: f64,
        size: u32,
        labelled: bool,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ConfOutput {
    file: Option<String>,
    file_metadata: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
    ticks: Option<Vec<Tick>>,
    show_eye_level: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Output {
    pub file: String,
    pub file_metadata: Option<String>,
    pub width: u16,
    pub height: u16,
    pub ticks: Vec<Tick>,
    pub show_eye_level: bool,
}

impl ConfOutput {
    fn into_output(self) -> Output {
        Output {
            file: self.file.unwrap_or_else(|| "./output.png".to_owned()),
            file_metadata: self.file_metadata,
            width: self.width.unwrap_or(640),
            height: self.height.unwrap_or(480),
            ticks: self.ticks.unwrap_or_else(Vec::new),
            show_eye_level: self.show_eye_level.unwrap_or(false),
        }
    }
}

impl Default for Output {
    fn default() -> Output {
        Output {
            file: "./output.png".to_owned(),
            file_metadata: None,
            width: 640,
            height: 480,
            ticks: Vec::new(),
            show_eye_level: false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum AtmosphereDef {
    Path(String),
    Definition(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    scene: Scene,
    view: Option<ConfView>,
    atmosphere: Option<AtmosphereDef>,
    earth_shape: Option<EarthShape>,
    straight_rays: Option<bool>,
    simulation_step: Option<f64>,
    output: Option<ConfOutput>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Params {
    pub scene: Scene,
    pub view: View,
    pub env: Environment,
    pub straight_rays: bool,
    pub simulation_step: f64,
    pub output: Output,
}

impl Params {
    pub fn azimuth_to_x(&self, azimuth: f64) -> u32 {
        let x01 = (azimuth - self.view.frame.direction) / self.view.frame.fov + 0.5;
        ((self.output.width as f64) * x01) as u32
    }

    pub fn eye_level_to_y(&self) -> u32 {
        let width = self.output.width as f64;
        let height = self.output.height as f64;
        let aspect = width / height;
        let yf = self.view.frame.tilt * aspect / self.view.frame.fov;
        ((yf + 0.5) * height) as u32
    }
}

impl Config {
    fn into_params(self) -> Params {
        let atmosphere = if let Some(atm_def) = self.atmosphere {
            match atm_def {
                AtmosphereDef::Path(path) => {
                    let mut atm_abs_path = env::current_dir().unwrap();
                    atm_abs_path.push(&path);
                    get_atmosphere(&atm_abs_path)
                }
                AtmosphereDef::Definition(def) => atm_from_str(&def).unwrap(),
            }
        } else {
            us76_atmosphere()
        };
        let earth_shape = self.earth_shape.unwrap_or(EarthShape::Spherical {
            radius: 6_371_000.0,
        });
        Params {
            scene: self.scene,
            view: self
                .view
                .map(|conf_view| conf_view.into_view(&earth_shape))
                .unwrap_or_else(Default::default),
            env: Environment {
                shape: earth_shape,
                atmosphere,
            },
            straight_rays: self.straight_rays.unwrap_or(false),
            simulation_step: self.simulation_step.unwrap_or(50.0),
            output: self
                .output
                .map(ConfOutput::into_output)
                .unwrap_or_else(Default::default),
        }
    }
}

impl Default for Params {
    fn default() -> Params {
        Params {
            scene: Scene {
                terrain_folder: "./terrain".to_owned(),
                objects: vec![],
            },
            view: Default::default(),
            env: Environment {
                shape: EarthShape::Spherical {
                    radius: 6_371_000.0,
                },
                atmosphere: us76_atmosphere(),
            },
            straight_rays: false,
            simulation_step: 50.0,
            output: Default::default(),
        }
    }
}

pub fn parse_params() -> Params {
    let matches = App::new("Atmospheric Panorama Raytracer")
        .version("0.4")
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
                .help("Viewpoint latitude in degrees (default: 0)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("longitude")
                .short("g")
                .long("lon")
                .value_name("DEG")
                .help("Viewpoint longitude in degrees (default: 0)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("altitude")
                .short("a")
                .long("alt")
                .value_name("ALT")
                .conflicts_with("elevation")
                .help("Viewpoint altitude in meters")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("elevation")
                .short("e")
                .long("elev")
                .value_name("ELEV")
                .conflicts_with("altitude")
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
                .help("File name to save the output image as (default: output.png)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output-meta")
                .long("output-meta")
                .value_name("FILE")
                .help("File name to save the output metadata as")
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
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Path to a config file with alternative defaults")
                .takes_value(true),
        )
        .get_matches();

    let mut params = if let Some(config_path) = matches.value_of("config") {
        let mut config_abs_path = env::current_dir().unwrap();
        config_abs_path.push(&config_path);
        let mut config_file = File::open(&config_abs_path).unwrap();
        let mut contents = String::new();
        config_file.read_to_string(&mut contents).unwrap();
        serde_yaml::from_str::<Config>(&contents)
            .unwrap()
            .into_params()
    } else {
        Default::default()
    };

    if let Some(terrain) = matches.value_of("terrain") {
        params.scene.terrain_folder = terrain.to_owned();
    }
    if let Some(output) = matches.value_of("output") {
        params.output.file = output.to_owned();
    }
    if let Some(output_metadata) = matches.value_of("output-meta") {
        params.output.file_metadata = Some(output_metadata.to_owned());
    }

    if let Some(pic_width) = matches.value_of("width") {
        params.output.width = pic_width.parse().expect("Invalid output width");
    }

    if let Some(pic_height) = matches.value_of("height") {
        params.output.height = pic_height.parse().expect("Invalid output height");
    }

    if let Some(lat) = matches.value_of("latitude") {
        params.view.position.latitude = lat.parse().expect("Invalid viewpoint latitude");
    }

    if let Some(lon) = matches.value_of("longitude") {
        params.view.position.longitude = lon.parse().expect("Invalid viewpoint longitude");
    }

    match (matches.value_of("altitude"), matches.value_of("elevation")) {
        (Some(a), None) => {
            params.view.position.altitude =
                Altitude::Absolute(a.parse().expect("Invalid viewpoint altitude"));
        }
        (None, Some(e)) => {
            params.view.position.altitude =
                Altitude::Relative(e.parse().expect("Invalid viewpoint elevation"));
        }
        _ => (),
    };

    if let Some(dir) = matches.value_of("direction") {
        params.view.frame.direction = dir.parse().expect("Invalid viewing azimuth");
    }

    if let Some(fov) = matches.value_of("fov") {
        params.view.frame.fov = fov.parse().expect("Invalid field of view");
    }

    if let Some(tilt) = matches.value_of("tilt") {
        params.view.frame.tilt = tilt.parse().expect("Invalid view tilt");
    }

    if let Some(max_dist) = matches.value_of("max-dist") {
        params.view.frame.max_distance =
            max_dist.parse::<f64>().expect("Invalid cutoff distance") * 1e3;
    }

    if let Some(step) = matches.value_of("step") {
        params.simulation_step = step.parse().expect("Invalid step value");
    }

    if let Some(atmosphere) = matches.value_of("atmosphere") {
        let atmosphere = get_atmosphere(&atmosphere);
        params.env.atmosphere = atmosphere;
    }

    match (matches.is_present("flat"), matches.value_of("radius")) {
        (true, None) => {
            params.env.shape = EarthShape::Flat;
        }
        (false, Some(radius)) => {
            let r: f64 = radius.parse().expect("Invalid radius passed");
            params.env.shape = EarthShape::Spherical { radius: r * 1e3 };
        }
        (true, Some(_)) => panic!("Conflicting Earth shape options chosen!"),
        _ => (),
    };

    params
}
