use std::{env, path::PathBuf};

use cc::Build;
use pkg_config::Config;

fn main() {
    let mut build = Build::new();

    for dir in ffmpeg_include_dirs() {
        build.include(dir);
    }

    build
        .file("src/error.c")
        .file("src/logger.c")
        .file("src/packet.c")
        .file("src/stream.c")
        .file("src/time.c")
        .file("src/format/demuxer.c")
        .file("src/format/io.c")
        .file("src/format/muxer.c")
        .file("src/codec/bsf.c")
        .file("src/codec/mod.c")
        .file("src/codec/frame.c")
        .file("src/codec/audio/resampler.c")
        .file("src/codec/video/scaler.c")
        .compile("ffwrapper");

    link_static("ffwrapper");

    for dir in ffmpeg_lib_dirs() {
        println!("cargo:rustc-link-search=native={}", dir.to_str().unwrap());
    }

    let ffmpeg_link_mode = lib_mode("ffmpeg");

    link("avcodec", ffmpeg_link_mode);
    link("avformat", ffmpeg_link_mode);
    link("avutil", ffmpeg_link_mode);
    link("swresample", ffmpeg_link_mode);
    link("swscale", ffmpeg_link_mode);
}

fn ffmpeg_include_dirs() -> Vec<PathBuf> {
    if let Ok(dir) = env::var("FFMPEG_INCLUDE_DIR") {
        let dir = PathBuf::from(dir);

        if dir.is_dir() {
            return vec![dir];
        }
    }

    let lib = Config::new()
        .cargo_metadata(false)
        .env_metadata(false)
        .print_system_libs(false)
        .print_system_cflags(false)
        .probe("libavcodec")
        .expect("Unable to find FFmpeg include dir. You can specify it explicitly by setting the FFMPEG_INCLUDE_DIR environment variable.");

    lib.include_paths
}

fn ffmpeg_lib_dirs() -> Vec<PathBuf> {
    if let Ok(dir) = env::var("FFMPEG_LIB_DIR") {
        let dir = PathBuf::from(dir);

        if dir.is_dir() {
            return vec![dir];
        }
    }

    let lib = Config::new()
        .cargo_metadata(false)
        .env_metadata(false)
        .print_system_libs(false)
        .print_system_cflags(false)
        .probe("libavcodec")
        .expect("Unable to find FFmpeg lib dir. You can specify it explicitly by setting the FFMPEG_LIB_DIR environment variable.");

    lib.link_paths
}

fn link_static(lib: &str) {
    link(lib, "static")
}

fn link(lib: &str, mode: &str) {
    println!("cargo:rustc-link-lib={}={}", mode, lib);
}

fn lib_mode(lib: &str) -> &'static str {
    let kind = env::var(&format!("{}_STATIC", lib.to_uppercase()));

    match kind.ok().as_deref() {
        Some("0") => "dylib",
        Some(_) => "static",
        None => "dylib",
    }
}
