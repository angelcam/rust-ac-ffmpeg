use std::fs::File;

use ac_ffmpeg::format::demuxer::Demuxer;
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::format::muxer::{Muxer, OutputFormat};
use ac_ffmpeg::Error;

fn main() -> Result<(), Error> {
    let input_filename = "foo.mp4";
    let output_filename = "bar.mp4";

    let output_format = OutputFormat::guess_from_file_name(output_filename).ok_or_else(|| {
        Error::new(format!(
            "unable to get output format for file: {}",
            output_filename
        ))
    })?;

    // open files
    let input = File::open(input_filename).map_err(|err| {
        Error::new(format!(
            "unable to open input file {}: {}",
            input_filename, err
        ))
    })?;

    let output = File::create(output_filename).map_err(|err| {
        Error::new(format!(
            "unable to create output file {}: {}",
            output_filename, err
        ))
    })?;

    // create FFmpeg IOs
    let input_io = IO::from_seekable_read_stream(input);
    let output_io = IO::from_seekable_write_stream(output);

    // create demuxer and get codec parameters
    let mut demuxer = Demuxer::builder()
        .build(input_io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)?;

    // create muxer
    let mut muxer_builder = Muxer::builder();

    for codec_parameters in demuxer.codec_parameters() {
        muxer_builder.add_stream(codec_parameters)?;
    }

    let mut muxer = muxer_builder.build(output_io, output_format)?;

    // process data
    while let Some(packet) = demuxer.take()? {
        muxer.push(packet)?;
    }

    // flush the muxer
    muxer.flush()
}
