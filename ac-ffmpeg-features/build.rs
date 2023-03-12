use std::path::Path;

use cc::Build;

fn main() {
    let src = Path::new("src");

    let mut build = Build::new();

    for dir in ac_ffmpeg_build::ffmpeg_include_dirs(true) {
        build.include(dir);
    }

    println!("cargo:rerun-if-changed={}", src.display());

    build
        .file(src.join("features.c"))
        .compile("ac-ffmpeg-features-cc");
}
