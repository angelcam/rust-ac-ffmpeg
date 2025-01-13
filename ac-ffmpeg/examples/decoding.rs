use std::fs::File;

use ac_ffmpeg::{
    codec::{video::VideoDecoder, Decoder},
    format::{
        demuxer::{Demuxer, DemuxerWithStreamInfo},
        io::IO,
    },
    Error,
};
use clap::{Arg, Command};

/// Open a given input file.
fn open_input(path: &str) -> Result<DemuxerWithStreamInfo<File>, Error> {
    let input = File::open(path)
        .map_err(|err| Error::new(format!("unable to open input file {}: {}", path, err)))?;

    let io = IO::from_seekable_read_stream(input);

    Demuxer::builder()
        .build(io)?
        .find_stream_info(None)
        .map_err(|(_, err)| err)
}

/// Decode all video frames from the first video stream and print their
/// presentation timestamps.
fn print_video_frame_info(input: &str) -> Result<(), Error> {
    let mut demuxer = open_input(input)?;

    let (stream_index, (stream, _)) = demuxer
        .streams()
        .iter()
        .map(|stream| (stream, stream.codec_parameters()))
        .enumerate()
        .find(|(_, (_, params))| params.is_video_codec())
        .ok_or_else(|| Error::new("no video stream"))?;

    let mut decoder = VideoDecoder::from_stream(stream)?.build()?;

    // process data
    while let Some(packet) = demuxer.take()? {
        if packet.stream_index() != stream_index {
            continue;
        }

        decoder.push(packet)?;

        while let Some(frame) = decoder.take()? {
            println!("{}", frame.pts().as_f32().unwrap_or(0f32));
        }
    }

    decoder.flush()?;

    while let Some(frame) = decoder.take()? {
        println!("{}", frame.pts().as_f32().unwrap_or(0f32));
    }

    Ok(())
}

fn main() {
    let matches = Command::new("decoding")
        .arg(
            Arg::new("input")
                .required(true)
                .value_name("INPUT")
                .help("Input file"),
        )
        .get_matches();

    let input_filename = matches.get_one::<String>("input").unwrap();

    if let Err(err) = print_video_frame_info(input_filename) {
        eprintln!("ERROR: {}", err);
    }
}
