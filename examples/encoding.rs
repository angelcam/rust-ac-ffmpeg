use std::{fs::File, time::Duration};

use ac_ffmpeg::{
    codec::{
        video::{self, VideoEncoder, VideoFrameMut},
        CodecParameters, Encoder,
    },
    format::{
        io::IO,
        muxer::{Muxer, OutputFormat},
    },
    time::{TimeBase, Timestamp},
    Error,
};
use clap::{App, Arg};

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

/// Create h264 encoded black video file of a given length and with a given
/// resolution.
fn encode_black_video(
    output: &str,
    width: u32,
    height: u32,
    duration: Duration,
) -> Result<(), Error> {
    let pixel_format = video::frame::get_pixel_format("yuv420p");

    // create a black video frame with a given resolution
    let frame = VideoFrameMut::black(pixel_format, width as _, height as _).freeze();

    // note: it is 1/fps
    let time_base = TimeBase::new(1, 25);

    let mut encoder = VideoEncoder::builder("libx264")?
        .pixel_format(pixel_format)
        .width(width as _)
        .height(height as _)
        .time_base(time_base)
        .build()?;

    let codec_parameters = encoder.codec_parameters().into();

    let mut muxer = open_output(output, &[codec_parameters])?;

    let mut frame_idx = 0;
    let mut frame_timestamp = Timestamp::new(frame_idx, time_base);
    let max_timestamp = Timestamp::from_secs(0) + duration;

    while frame_timestamp < max_timestamp {
        let cloned_frame = frame.clone().with_pts(frame_timestamp);

        encoder.push(cloned_frame)?;

        while let Some(packet) = encoder.take()? {
            muxer.push(packet.with_stream_index(0))?;
        }

        frame_idx += 1;
        frame_timestamp = Timestamp::new(frame_idx, time_base);
    }

    encoder.flush()?;

    while let Some(packet) = encoder.take()? {
        muxer.push(packet.with_stream_index(0))?;
    }

    muxer.flush()
}

fn main() {
    let matches = App::new("encoding")
        .arg(
            Arg::with_name("output")
                .required(true)
                .takes_value(true)
                .value_name("OUTPUT")
                .help("Output file"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .takes_value(true)
                .value_name("WIDTH")
                .help("width")
                .default_value("640"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .takes_value(true)
                .value_name("HEIGHT")
                .help("height")
                .default_value("480"),
        )
        .arg(
            Arg::with_name("duration")
                .short("d")
                .takes_value(true)
                .value_name("DURATION")
                .help("duration in seconds")
                .default_value("10"),
        )
        .get_matches();

    let output_filename = matches.value_of("output").unwrap();
    let width = matches.value_of("width").unwrap().parse().unwrap();
    let height = matches.value_of("height").unwrap().parse().unwrap();
    let duration = matches.value_of("duration").unwrap().parse().unwrap();

    let duration = Duration::from_secs_f32(duration);

    if let Err(err) = encode_black_video(output_filename, width, height, duration) {
        eprintln!("ERROR: {}", err);
    }
}
