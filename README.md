# Rust wrapper for FFmpeg libraries

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
