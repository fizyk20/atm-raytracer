mod app;

use std::{fs::File, io::Read};

use clap::{App, Arg, ArgMatches, SubCommand};
use libflate::gzip::Decoder;

use crate::generator::AllData;

pub const SUBCOMMAND: &'static str = "view";

pub fn run(matches: &ArgMatches<'_>) -> Result<(), String> {
    let filename = matches
        .value_of("input")
        .expect("please provide an input file");

    let mut file = File::open(filename).expect("couldn't open the input file");
    let mut zipped_data = vec![];
    let _ = file
        .read_to_end(&mut zipped_data)
        .expect("couldn't read the file");

    let mut decoder = Decoder::new(&zipped_data[..]).expect("couldn't create the decoder");
    let mut data = vec![];
    let _ = decoder
        .read_to_end(&mut data)
        .expect("couldn't inflate the data");

    let data: AllData = bincode::deserialize(&data[..]).expect("couldn't deserialize the data");

    app::run(data)?;

    Ok(())
}

pub fn subcommand_def() -> App<'static, 'static> {
    SubCommand::with_name(SUBCOMMAND)
        .about("Launch the panorama viewer")
        .arg(
            Arg::with_name("input")
                .help("Path to the input file")
                .required(true)
                .index(1),
        )
}
