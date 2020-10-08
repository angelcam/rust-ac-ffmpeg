# Rust wrapper for FFmpeg libraries

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][license-badge]][license-url]
[![Build Status][build-badge]][build-url]

[crates-badge]: https://img.shields.io/crates/v/ac-ffmpeg
[crates-url]: https://crates.io/crates/ac-ffmpeg
[license-badge]: https://img.shields.io/crates/l/ac-ffmpeg
[license-url]: https://github.com/angelcam/rust-ac-ffmpeg/blob/master/LICENSE
[build-badge]: https://travis-ci.org/angelcam/rust-ac-ffmpeg.svg?branch=master
[build-url]: https://travis-ci.org/angelcam/rust-ac-ffmpeg

This library provides a Rust interface for FFmpeg libraries. Rather than
supporting all FFmpeg features, we focus on safety and simplicity of the
interface.

## Supported features

* Demuxing any self-contained media container
* Muxing any self-contained media container
* Decoding audio and video
* Encoding audio and video
* Video frame scaling and pixel format transformations
* Audio resampling
* Bitstream filters

## Requirements

* FFmpeg v4.x libraries, the following libraries are required:
    * libavutil
    * libavcodec
    * libavformat
    * libswresample
    * libswscale

## Compilation

The following env. variables can be used to set correct paths to FFmpeg header
files and libraries:

* `FFMPEG_INCLUDE_DIR` - path to the FFmpeg header files
* `FFMPEG_LIB_DIR` - path to the FFmpeg libs

If you prefer static linking, you can force it using:

* `FFMPEG_STATIC=1`

## License

Even though this library is distributed under the MIT license, the FFmpeg
project has its own license policies that need to be respected. See
https://ffmpeg.org/legal.html for more details.
