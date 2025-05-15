use std::{fs::File, time::Duration};

use ac_ffmpeg::{
    codec::{
        video::{self, filter::VideoFilter, PixelFormat, VideoEncoder, VideoFrame, VideoFrameMut},
        Encoder, Filter,
    },
    format::{
        io::IO,
        muxer::{Muxer, OutputFormat},
    },
    time::{TimeBase, Timestamp},
    Error,
};
use clap::{Arg, Command};

/// Encoding and muxing output pipeline.
struct OutputPipeline {
    encoder: VideoEncoder,
    muxer: Muxer<File>,
}

impl OutputPipeline {
    /// Create a new output pipeline.
    fn open(
        width: usize,
        height: usize,
        pixel_fmt: PixelFormat,
        time_base: TimeBase,
        file: &str,
    ) -> Result<Self, Error> {
        let encoder = VideoEncoder::builder("libx264")?
            .pixel_format(pixel_fmt)
            .width(width)
            .height(height)
            .time_base(time_base)
            .build()?;

        let codec_parameters = encoder.codec_parameters().into();

        let output_format = OutputFormat::guess_from_file_name(file).ok_or_else(|| {
            Error::new(format!("unable to guess output format for file: {}", file))
        })?;

        let output = File::create(file)
            .map_err(|err| Error::new(format!("unable to create output file {}: {}", file, err)))?;

        let io = IO::from_seekable_write_stream(output);

        let mut muxer_builder = Muxer::builder();

        muxer_builder.add_stream(&codec_parameters)?;

        let muxer = muxer_builder.build(io, output_format)?;

        let res = Self { encoder, muxer };

        Ok(res)
    }

    /// Push a given frame to the pipeline.
    fn push(&mut self, frame: VideoFrame) -> Result<(), Error> {
        self.encoder.push(frame)?;

        while let Some(packet) = self.encoder.take()? {
            self.muxer.push(packet.with_stream_index(0))?;
        }

        Ok(())
    }

    /// Close the pipeline.
    fn close(mut self) -> Result<(), Error> {
        self.encoder.flush()?;

        while let Some(packet) = self.encoder.take()? {
            self.muxer.push(packet.with_stream_index(0))?;
        }

        self.muxer.flush()
    }
}

/// Create h264 encoded black video file of a given length and with a given
/// resolution, with timecode burnt in using the drawtext filter
fn encode_black_video_with_bitc(
    output: &str,
    width: usize,
    height: usize,
    duration: Duration,
) -> Result<(), Error> {
    // note: it is 1/fps
    let time_base = TimeBase::new(1, 25);

    let pixel_format = video::frame::get_pixel_format("yuv420p");

    // create a black video frame with a given resolution
    let frame = VideoFrameMut::black(pixel_format, width, height)
        .with_time_base(time_base)
        .freeze();

    let mut draw_text_filter = VideoFilter::builder(width, height, pixel_format)
        .input_time_base(time_base)
        .build("drawtext=timecode='00\\:00\\:00\\:00':rate=25:fontsize=72:fontcolor=white")?;

    let mut output = OutputPipeline::open(width, height, pixel_format, time_base, output)?;

    let mut current_timestamp = Timestamp::new(0, time_base);

    let max_timestamp = current_timestamp + duration;

    let mut frame_idx = 0;

    while current_timestamp < max_timestamp {
        let cloned_frame = frame.clone().with_pts(current_timestamp);

        draw_text_filter.push(cloned_frame)?;

        while let Some(frame) = draw_text_filter.take()? {
            output.push(frame)?;
        }

        frame_idx += 1;

        current_timestamp = Timestamp::new(frame_idx, time_base);
    }

    draw_text_filter.flush()?;

    while let Some(frame) = draw_text_filter.take()? {
        output.push(frame)?;
    }

    output.close()
}

fn main() {
    let matches = Command::new("filtering")
        .arg(
            Arg::new("output")
                .required(true)
                .value_name("OUTPUT")
                .help("Output file"),
        )
        .arg(
            Arg::new("width")
                .short('W')
                .value_name("WIDTH")
                .value_parser(clap::value_parser!(usize))
                .help("width")
                .default_value("640"),
        )
        .arg(
            Arg::new("height")
                .short('H')
                .value_name("HEIGHT")
                .value_parser(clap::value_parser!(usize))
                .help("height")
                .default_value("480"),
        )
        .arg(
            Arg::new("duration")
                .short('D')
                .value_name("DURATION")
                .value_parser(clap::value_parser!(f32))
                .help("duration in seconds")
                .default_value("10"),
        )
        .get_matches();

    let output_filename = matches.get_one::<String>("output").unwrap();
    let width = matches.get_one::<usize>("width").copied().unwrap();
    let height = matches.get_one::<usize>("height").copied().unwrap();
    let duration = matches.get_one::<f32>("duration").copied().unwrap();

    let duration = Duration::from_secs_f32(duration);

    if let Err(err) = encode_black_video_with_bitc(output_filename, width, height, duration) {
        eprintln!("ERROR: {}", err);
    }
}
