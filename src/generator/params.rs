use std::{env, fs::File, io::Read};

use crate::{
    coloring::{ColoringMethod, Shading, SimpleColors},
    object::{ConfObject, Object},
    terrain::Terrain,
    utils::world_directions,
};

use atm_refraction::{
    air::{Atmosphere, AtmosphereDef},
    EarthShape, Environment,
};
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
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
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    #[serde(default)]
    pub latitude: f64,
    #[serde(default)]
    pub longitude: f64,
    #[serde(default = "default_altitude")]
    pub altitude: Altitude,
}

fn default_altitude() -> Altitude {
    Altitude::Relative(1.0)
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
pub struct ConfScene {
    #[serde(default = "default_terrain_folder")]
    pub terrain_folder: String,
    #[serde(default)]
    pub objects: Vec<ConfObject>,
}

fn default_terrain_folder() -> String {
    "./terrain".to_string()
}

impl Default for ConfScene {
    fn default() -> Self {
        Self {
            terrain_folder: default_terrain_folder(),
            objects: vec![],
        }
    }
}

impl ConfScene {
    fn into_scene(self, terrain: &Terrain) -> Scene {
        let objects = self
            .objects
            .into_iter()
            .map(|obj| obj.into_object(terrain))
            .collect();
        Scene {
            terrain_folder: self.terrain_folder,
            objects,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Scene {
    pub terrain_folder: String,
    pub objects: Vec<Object>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Frame {
    #[serde(default)]
    pub direction: f64,
    #[serde(default)]
    pub tilt: f64,
    #[serde(default = "default_fov")]
    pub fov: f64,
    #[serde(default = "default_distance")]
    pub max_distance: f64,
}

fn default_fov() -> f64 {
    30.0
}

fn default_distance() -> f64 {
    150_000.0
}

impl Default for Frame {
    fn default() -> Frame {
        Frame {
            direction: 0.0,
            tilt: 0.0,
            fov: default_fov(),
            max_distance: default_distance(),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ConfColoring {
    Simple {
        #[serde(default)]
        water_level: f64,
    },
    Shading {
        #[serde(default)]
        water_level: f64,
        #[serde(default = "default_ambient_light")]
        ambient_light: f64,
        #[serde(default = "default_zenith_angle")]
        light_zenith_angle: f64,
        #[serde(default)]
        light_dir: f64,
    },
}

fn default_ambient_light() -> f64 {
    0.4
}

fn default_zenith_angle() -> f64 {
    45.0
}

impl Default for ConfColoring {
    fn default() -> Self {
        ConfColoring::Shading {
            water_level: 0.0,
            ambient_light: default_ambient_light(),
            light_zenith_angle: default_zenith_angle(),
            light_dir: 0.0,
        }
    }
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
                water_level,
                max_distance: frame.max_distance,
            },
            ConfColoring::Shading {
                water_level,
                ambient_light,
                light_zenith_angle,
                light_dir,
            } => {
                let light_zenith_angle = light_zenith_angle.to_radians();
                let light_dir = light_dir.to_radians();
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

#[derive(Clone, Copy, Serialize, Deserialize, Default)]
pub struct ConfView {
    #[serde(default)]
    position: Position,
    #[serde(default)]
    frame: Frame,
    #[serde(default)]
    coloring: ConfColoring,
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
        let coloring = self
            .coloring
            .into_coloring(&self.frame, &self.position, earth_shape);
        View {
            position: self.position,
            frame: self.frame,
            coloring,
            fog_distance: self.fog_distance,
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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum GeneratorDef {
    Fast,
    Rectilinear,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Output {
    #[serde(default = "default_file")]
    pub file: String,
    pub file_metadata: Option<String>,
    #[serde(default = "default_width")]
    pub width: u16,
    #[serde(default = "default_height")]
    pub height: u16,
    #[serde(default)]
    pub ticks: Vec<Tick>,
    #[serde(default)]
    pub show_eye_level: bool,
    #[serde(default = "default_generator")]
    pub generator: GeneratorDef,
}

fn default_file() -> String {
    "./output.png".to_owned()
}

fn default_width() -> u16 {
    640
}

fn default_height() -> u16 {
    480
}

fn default_generator() -> GeneratorDef {
    GeneratorDef::Fast
}

impl Default for Output {
    fn default() -> Output {
        Output {
            file: default_file(),
            file_metadata: None,
            width: default_width(),
            height: default_height(),
            ticks: Vec::new(),
            show_eye_level: false,
            generator: default_generator(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    scene: ConfScene,
    #[serde(default)]
    view: ConfView,
    #[serde(default = "AtmosphereDef::us_76")]
    atmosphere: AtmosphereDef,
    #[serde(default = "default_earth_shape")]
    earth_shape: EarthShape,
    #[serde(default)]
    straight_rays: bool,
    #[serde(default = "default_simulation_step")]
    simulation_step: f64,
    #[serde(default)]
    output: Output,
}

fn default_earth_shape() -> EarthShape {
    EarthShape::Spherical {
        radius: 6_371_000.0,
    }
}

fn default_simulation_step() -> f64 {
    50.0
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scene: Default::default(),
            view: Default::default(),
            atmosphere: AtmosphereDef::us_76(),
            earth_shape: default_earth_shape(),
            straight_rays: false,
            simulation_step: default_simulation_step(),
            output: Default::default(),
        }
    }
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

impl Config {
    pub fn terrain_folder(&self) -> &str {
        &self.scene.terrain_folder
    }

    pub fn into_params(self, terrain: &Terrain) -> Params {
        let scene = self.scene.into_scene(terrain);
        let atmosphere = Atmosphere::from_def(self.atmosphere);
        Params {
            scene,
            view: self.view.into_view(&self.earth_shape),
            env: Environment {
                shape: self.earth_shape,
                atmosphere,
            },
            straight_rays: self.straight_rays,
            simulation_step: self.simulation_step,
            output: self.output,
        }
    }
}

pub fn subcommand_def() -> App<'static, 'static> {
    SubCommand::with_name(super::SUBCOMMAND).about("Render a panorama")
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
        .arg(
            Arg::with_name("output-atm-data")
                .long("output-atm-data")
                .help("With this switch, the program only outputs the profile of pressure and\
                    temperature to a file.")
                .takes_value(false),
        )
}

pub fn parse_config(matches: &ArgMatches<'_>) -> Result<Config, ()> {
    let mut config = if let Some(config_path) = matches.value_of("config") {
        let mut config_abs_path = env::current_dir().unwrap();
        config_abs_path.push(&config_path);
        let mut config_file = File::open(&config_abs_path).unwrap();
        let mut contents = String::new();
        config_file.read_to_string(&mut contents).unwrap();
        serde_yaml::from_str::<Config>(&contents).unwrap()
    } else {
        Default::default()
    };

    if let Some(terrain) = matches.value_of("terrain") {
        config.scene.terrain_folder = terrain.to_owned();
    }
    if let Some(output) = matches.value_of("output") {
        config.output.file = output.to_owned();
    }
    if let Some(output_metadata) = matches.value_of("output-meta") {
        config.output.file_metadata = Some(output_metadata.to_owned());
    }

    if let Some(pic_width) = matches.value_of("width") {
        config.output.width = pic_width.parse().expect("Invalid output width");
    }

    if let Some(pic_height) = matches.value_of("height") {
        config.output.height = pic_height.parse().expect("Invalid output height");
    }

    if let Some(lat) = matches.value_of("latitude") {
        config.view.position.latitude = lat.parse().expect("Invalid viewpoint latitude");
    }

    if let Some(lon) = matches.value_of("longitude") {
        config.view.position.longitude = lon.parse().expect("Invalid viewpoint longitude");
    }

    match (matches.value_of("altitude"), matches.value_of("elevation")) {
        (Some(a), None) => {
            config.view.position.altitude =
                Altitude::Absolute(a.parse().expect("Invalid viewpoint altitude"));
        }
        (None, Some(e)) => {
            config.view.position.altitude =
                Altitude::Relative(e.parse().expect("Invalid viewpoint elevation"));
        }
        _ => (),
    };

    if let Some(dir) = matches.value_of("direction") {
        config.view.frame.direction = dir.parse().expect("Invalid viewing azimuth");
    }

    if let Some(fov) = matches.value_of("fov") {
        config.view.frame.fov = fov.parse().expect("Invalid field of view");
    }

    if let Some(tilt) = matches.value_of("tilt") {
        config.view.frame.tilt = tilt.parse().expect("Invalid view tilt");
    }

    if let Some(max_dist) = matches.value_of("max-dist") {
        config.view.frame.max_distance =
            max_dist.parse::<f64>().expect("Invalid cutoff distance") * 1e3;
    }

    if let Some(step) = matches.value_of("step") {
        config.simulation_step = step.parse().expect("Invalid step value");
    }

    match (matches.is_present("flat"), matches.value_of("radius")) {
        (true, None) => {
            config.earth_shape = EarthShape::Flat;
        }
        (false, Some(radius)) => {
            let r: f64 = radius.parse().expect("Invalid radius passed");
            config.earth_shape = EarthShape::Spherical { radius: r * 1e3 };
        }
        (true, Some(_)) => panic!("Conflicting Earth shape options chosen!"),
        _ => (),
    };

    if matches.is_present("straight") {
        config.straight_rays = true;
    }

    if matches.is_present("output-atm-data") {
        output_atm_data(&config);
        Err(())
    } else {
        Ok(config)
    }
}

fn output_atm_data(config: &Config) {
    let atmosphere = Atmosphere::from_def(config.atmosphere.clone());
    for alt in 0..=5000 {
        let alt = alt as f64 * 0.2;
        println!(
            "{} {} {}",
            alt,
            atmosphere.temperature(alt),
            atmosphere.pressure(alt)
        );
    }
}
