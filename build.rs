extern crate cc;

use std::env;

use std::path::Path;

use cc::Build;

fn main() {
    let ffmpeg_include_dir = ffmpeg_include_dir();

    Build::new()
        .include(&ffmpeg_include_dir)
        .file("src/error.c")
        .file("src/logger.c")
        .file("src/packet.c")
        .file("src/format/io.c")
        .file("src/format/muxer.c")
        .file("src/codec/mod.c")
        .file("src/codec/frame.c")
        .file("src/codec/audio/resampler.c")
        .file("src/codec/video/scaler.c")
        .compile("ffwrapper");

    link_static("ffwrapper");

    if let Some(dir) = ffmpeg_lib_dir() {
        println!("cargo:rustc-link-search=native={}", dir);
    }

    let ffmpeg_link_mode = lib_mode("ffmpeg");

    link("avcodec", ffmpeg_link_mode);
    link("avformat", ffmpeg_link_mode);
    link("avutil", ffmpeg_link_mode);
    link("swresample", ffmpeg_link_mode);
    link("swscale", ffmpeg_link_mode);
}

fn is_dir(d: &str) -> bool {
    let path = Path::new(d);

    path.is_dir()
}

fn ffmpeg_include_dir() -> String {
    if let Ok(include) = env::var("FFMPEG_INCLUDE_DIR") {
        if is_dir(&include) {
            return include;
        }
    }

    if is_dir("/usr/include/ffmpeg") {
        return String::from("/usr/include/ffmpeg");
    }

    panic!("Unable to find FFmpeg include dir. You can specify it explicitly by setting the FFMPEG_INCLUDE_DIR environment variable.");
}

fn ffmpeg_lib_dir() -> Option<String> {
    if let Ok(dir) = env::var("FFMPEG_LIB_DIR") {
        if is_dir(&dir) {
            return Some(dir);
        }
    }

    None
}

fn link_static(lib: &str) {
    link(lib, "static")
}

fn link(lib: &str, mode: &str) {
    println!("cargo:rustc-link-lib={}={}", mode, lib);
}

fn lib_mode(lib: &str) -> &'static str {
    let kind = env::var(&format!("{}_STATIC", lib.to_uppercase()));

    match kind.ok().as_ref().map(|v| v.as_str()) {
        Some("0") => "dylib",
        Some(_) => "static",
        None => "dylib",
    }
}
