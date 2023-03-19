use std::{env, ffi::OsStr, path::PathBuf};

/// Get FFmpeg include directories.
///
/// # Arguments
/// * `env_metadata` - if `true`, the function will emit Cargo metadata to
///   re-run the build if the corresponding env. variables change
pub fn ffmpeg_include_dirs(env_metadata: bool) -> Vec<PathBuf> {
    if let Some(dir) = path_from_env("FFMPEG_INCLUDE_DIR", env_metadata) {
        if dir.is_dir() {
            return vec![dir];
        }
    }

    let lib = find_ffmpeg_lib()
        .expect("Unable to find FFmpeg include dir. You can specify it explicitly by setting the FFMPEG_INCLUDE_DIR environment variable.");

    lib.include_paths
}

/// Get FFmpeg library directories.
///
/// # Arguments
/// * `env_metadata` - if `true`, the function will emit Cargo metadata to
///   re-run the build if the corresponding env. variables change
pub fn ffmpeg_lib_dirs(env_metadata: bool) -> Vec<PathBuf> {
    if let Some(dir) = path_from_env("FFMPEG_LIB_DIR", env_metadata) {
        if dir.is_dir() {
            return vec![dir];
        }
    }

    let lib = find_ffmpeg_lib()
        .expect("Unable to find FFmpeg lib dir. You can specify it explicitly by setting the FFMPEG_LIB_DIR environment variable.");

    lib.link_paths
}

cfg_if::cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "macos"))] {
        use pkg_config::{Config, Library};

        /// Find a given library using pkg-config.
        fn find_ffmpeg_lib() -> Option<Library> {
            Config::new()
                .cargo_metadata(false)
                .probe("libavcodec")
                .ok()
        }
    } else if #[cfg(target_os = "windows")] {
        use vcpkg::{Config, Library};

        /// Find a given library/package in the vcpkg tree.
        fn find_ffmpeg_lib() -> Option<Library> {
            Config::new()
                .cargo_metadata(false)
                .find_package("ffmpeg")
                .ok()
        }
    } else {
        /// Helper struct.
        struct Library {
            include_paths: Vec<PathBuf>,
            link_paths: Vec<PathBuf>,
        }

        /// Dummy function.
        fn find_ffmpeg_lib() -> Option<Library> {
            None
        }
    }
}

/// Get a given path from the environment.
fn path_from_env(name: &str, env_metadata: bool) -> Option<PathBuf> {
    let target = normalized_target();

    if env_metadata {
        emit_env_metadata(name, target.as_deref());
    }

    if let Some(target) = target {
        if let Some(path) = path_from_var(format!("{name}_{target}")) {
            return Some(path);
        }
    }

    if let Some(path) = path_from_var(name) {
        return Some(path);
    }

    None
}

/// Get path from a given env. variable.
fn path_from_var<K>(key: K) -> Option<PathBuf>
where
    K: AsRef<OsStr>,
{
    Some(PathBuf::from(env::var_os(key)?))
}

/// Emit Cargo metadata that will rerun the build if a given variable or its
/// target-specific variant changes.
fn emit_env_metadata(name: &str, target: Option<&str>) {
    if let Some(target) = target {
        println!("cargo:rerun-if-env-changed={name}_{target}");
    }

    println!("cargo:rerun-if-env-changed={name}");
}

/// Get uppercase target with dashes replaced by underscores.
fn normalized_target() -> Option<String> {
    let target = env::var("TARGET").ok()?.to_uppercase().replace('-', "_");

    Some(target)
}
