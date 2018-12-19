use std::ptr;

use std::ffi::CString;

use libc::{c_char, c_int, c_void, uint8_t};

use Error;

extern "C" {
    fn ffw_video_codec_parameters_new(codec: *const c_char) -> *mut c_void;
    fn ffw_codec_parameters_clone(params: *const c_void) -> *mut c_void;
    fn ffw_codec_parameters_set_width(params: *mut c_void, width: c_int);
    fn ffw_codec_parameters_set_height(params: *mut c_void, height: c_int);
    fn ffw_codec_parameters_set_extradata(
        params: *mut c_void,
        extradata: *const uint8_t,
        size: c_int,
    ) -> c_int;
    fn ffw_codec_parameters_free(params: *mut c_void);
}


/// Builder for video codec parameters.
pub struct VideoCodecParametersBuilder {
    ptr: *mut c_void,
}

impl VideoCodecParametersBuilder {
    /// Create a new builder for a given video codec.
    fn new(codec: &str) -> Result<VideoCodecParametersBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_video_codec_parameters_new(codec.as_ptr() as *const _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let res = VideoCodecParametersBuilder {
            ptr: ptr,
        };

        Ok(res)
    }

    /// Set frame width.
    pub fn width(self, width: usize) -> VideoCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_width(self.ptr, width as _);
        }

        self
    }

    /// Set frame height.
    pub fn height(self, height: usize) -> VideoCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_height(self.ptr, height as _);
        }

        self
    }

    /// Set extradata.
    pub fn extradata(self, data: Option<&[u8]>) -> VideoCodecParametersBuilder {
        let ptr;
        let size;

        if let Some(data) = data {
            ptr = data.as_ptr();
            size = data.len();
        } else {
            ptr = ptr::null();
            size = 0;
        }

        let res = unsafe { ffw_codec_parameters_set_extradata(self.ptr, ptr, size as _) };

        if res < 0 {
            panic!("unable to allocate extradata");
        }

        self
    }

    /// Build the codec parameters.
    pub fn build(mut self) -> CodecParameters {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        CodecParameters { ptr: ptr }
    }
}

impl Drop for VideoCodecParametersBuilder {
    fn drop(&mut self) {
        unsafe { ffw_codec_parameters_free(self.ptr) }
    }
}

unsafe impl Send for VideoCodecParametersBuilder {}
unsafe impl Sync for VideoCodecParametersBuilder {}

/// Codec parameters.
pub struct CodecParameters {
    ptr: *mut c_void,
}

impl CodecParameters {
    /// Get a builder for video codec parameters for a given codec.
    pub fn video(codec: &str) -> Result<VideoCodecParametersBuilder, Error> {
        VideoCodecParametersBuilder::new(codec)
    }

    /// Create codec parameters from a given raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> CodecParameters {
        CodecParameters { ptr: ptr }
    }

    /// Get raw pointer to the underlying object.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }
}

impl Drop for CodecParameters {
    fn drop(&mut self) {
        unsafe { ffw_codec_parameters_free(self.ptr) }
    }
}

impl Clone for CodecParameters {
    fn clone(&self) -> CodecParameters {
        let ptr = unsafe {
            ffw_codec_parameters_clone(self.ptr)
        };

        if ptr.is_null() {
            panic!("unable to clone codec parameters");
        }

        CodecParameters { ptr: ptr }
    }
}

unsafe impl Send for CodecParameters {}
unsafe impl Sync for CodecParameters {}
