use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use cc::Build;
use pkg_config::Config;

fn main() {
    let docs_rs = std::env::var_os("DOCS_RS");

    for feature in get_ffmpeg_features(docs_rs.is_some()) {
        println!("cargo:rustc-cfg={}", feature);
    }

    // skip building native resources during docs.rs builds
    if docs_rs.is_some() {
        return;
    }

    let mut build = Build::new();

    for dir in ffmpeg_include_dirs() {
        build.include(dir);
    }

    build
        .include("src")
        .file("src/error.c")
        .file("src/logger.c")
        .file("src/packet.c")
        .file("src/time.c")
        .file("src/format/demuxer.c")
        .file("src/format/io.c")
        .file("src/format/muxer.c")
        .file("src/format/stream.c")
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
    if let Some(target) = normalized_target() {
        if let Ok(dir) = env::var(&format!("FFMPEG_INCLUDE_DIR_{}", target)) {
            let dir = PathBuf::from(dir);

            if dir.is_dir() {
                return vec![dir];
            }
        }
    } else {
        if let Ok(dir) = env::var("FFMPEG_INCLUDE_DIR") {
            let dir = PathBuf::from(dir);

            if dir.is_dir() {
                return vec![dir];
            }
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
    if let Some(target) = normalized_target() {
        if let Ok(dir) = env::var(&format!("FFMPEG_LIB_DIR_{}", target)) {
            let dir = PathBuf::from(dir);

            if dir.is_dir() {
                return vec![dir];
            }
        }
    } else {
        if let Ok(dir) = env::var("FFMPEG_LIB_DIR") {
            let dir = PathBuf::from(dir);

            if dir.is_dir() {
                return vec![dir];
            }
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

fn normalized_target() -> Option<String> {
    env::var("TARGET")
        .ok()
        .map(|t| t.to_uppercase().replace('-', "_"))
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

fn get_ffmpeg_features(all: bool) -> Vec<String> {
    let out_dir = std::env::var_os("OUT_DIR").expect("output directory is not defined");

    std::fs::create_dir_all(&out_dir).expect("unable to create the output directory");

    let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let src_dir = root_dir.join("src");

    let src_file = root_dir.join("build").join("native-features.c");

    let bin = Path::new(&out_dir).join("native-features");

    let mut cmd = Command::new("cc");

    for dir in ffmpeg_include_dirs() {
        cmd.arg(format!("-I{}", dir.to_string_lossy()));
    }

    if all {
        cmd.arg("-DPRINT_ALL_FEATURES");
    }

    let output = cmd
        .arg(format!("-I{}", src_dir.to_string_lossy()))
        .arg(src_file)
        .arg("-o")
        .arg(&bin)
        .output()
        .unwrap();

    if !output.status.success() {
        panic!(
            "unable to get FFmpeg features: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new(bin).output().unwrap();

    if !output.status.success() {
        panic!("unable to get FFmpeg features");
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.to_string())
        .collect()
}
