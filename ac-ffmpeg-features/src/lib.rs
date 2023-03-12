use std::{
    ffi::CStr,
    os::raw::{c_char, c_void},
};

type FeatureCallback = unsafe extern "C" fn(ctx: *mut c_void, feature: *const c_char);

extern "C" {
    fn get_ffmpeg_features(ctx: *mut c_void, all: u8, cb: FeatureCallback);
}

/// Get FFmpeg features.
///
/// # Arguments
/// * `all` - if `true` all features will be returned; otherwise, only the
///   features actually provided by the libraries will be returned
pub fn ffmpeg_features(all: bool) -> Vec<String> {
    let mut res = Vec::new();

    unsafe {
        get_ffmpeg_features(&mut res as *mut Vec<String> as _, all as _, push_feature);
    }

    res
}

/// Native feature callback.
unsafe extern "C" fn push_feature(ctx: *mut c_void, feature: *const c_char) {
    let features = &mut *(ctx as *mut Vec<String>);

    let feature = CStr::from_ptr(feature).to_str().map(String::from);

    if let Ok(f) = feature {
        features.push(f);
    }
}
