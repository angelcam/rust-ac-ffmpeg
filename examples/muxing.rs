use std::fs::File;

use ac_ffmpeg::{
    codec::CodecParameters,
    format::{
        demuxer::{Demuxer, DemuxerWithCodecParameters},
        io::IO,
        muxer::{Muxer, OutputFormat},
    },
    Error,
};
use clap::{App, Arg};

/// Open a given input file.
fn open_input(path: &str) -> Result<DemuxerWithCodecParameters<File>, Error> {
    let input = File::open(path)
        .map_err(|err| Error::new(format!("unable to open input file {}: {}", path, err)))?;

    let io = IO::from_seekable_read_stream(input);

    Demuxer::builder()
        .build(io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)
}

/// Open a given output file.
fn open_output(path: &str, elementary_streams: &[CodecParameters]) -> Result<Muxer<File>, Error> {
    let output_format = OutputFormat::guess_from_file_name(path)
        .ok_or_else(|| Error::new(format!("unable to guess output format for file: {}", path)))?;

    let output = File::create(path)
        .map_err(|err| Error::new(format!("unable to create output file {}: {}", path, err)))?;

    let io = IO::from_seekable_write_stream(output);

    let mut muxer_builder = Muxer::builder();

    for codec_parameters in elementary_streams {
        muxer_builder.add_stream(codec_parameters)?;
    }

    muxer_builder.build(io, output_format)
}

/// Convert a given input file into a given output file.
fn convert(input: &str, output: &str) -> Result<(), Error> {
    let mut demuxer = open_input(input)?;
    let mut muxer = open_output(output, demuxer.codec_parameters())?;

    // process data
    while let Some(packet) = demuxer.take()? {
        muxer.push(packet)?;
    }

    // flush the muxer
    muxer.flush()
}

fn main() {
    let matches = App::new("muxing")
        .arg(
            Arg::with_name("input")
                .required(true)
                .takes_value(true)
                .value_name("INPUT")
                .help("Input file"),
        )
        .arg(
            Arg::with_name("output")
                .required(true)
                .takes_value(true)
                .value_name("OUTPUT")
                .help("Output file"),
        )
        .get_matches();

    let input_filename = matches.value_of("input").unwrap();
    let output_filename = matches.value_of("output").unwrap();

    if let Err(err) = convert(input_filename, output_filename) {
        eprintln!("ERROR: {}", err);
    }
}
