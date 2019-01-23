pub mod audio;
pub mod video;

use std::fmt;
use std::ptr;
use std::slice;

use std::ffi::CString;
use std::fmt::{Display, Formatter};

use libc::{c_char, c_int, c_void, int64_t, uint64_t, uint8_t};

use crate::Error;

use crate::codec::audio::{ChannelLayout, SampleFormat};
use crate::codec::video::PixelFormat;

extern "C" {
    fn ffw_audio_codec_parameters_new(codec: *const c_char) -> *mut c_void;
    fn ffw_video_codec_parameters_new(codec: *const c_char) -> *mut c_void;
    fn ffw_codec_parameters_clone(params: *const c_void) -> *mut c_void;
    fn ffw_codec_parameters_is_audio_codec(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_is_video_codec(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_bit_rate(params: *const c_void) -> int64_t;
    fn ffw_codec_parameters_get_format(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_width(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_height(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_sample_rate(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_channel_layout(params: *const c_void) -> uint64_t;
    fn ffw_codec_parameters_get_extradata(params: *mut c_void) -> *mut c_void;
    fn ffw_codec_parameters_get_extradata_size(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_set_bit_rate(params: *mut c_void, bit_rate: int64_t);
    fn ffw_codec_parameters_set_format(params: *mut c_void, format: c_int);
    fn ffw_codec_parameters_set_width(params: *mut c_void, width: c_int);
    fn ffw_codec_parameters_set_height(params: *mut c_void, height: c_int);
    fn ffw_codec_parameters_set_sample_rate(params: *mut c_void, rate: c_int);
    fn ffw_codec_parameters_set_channel_layout(params: *mut c_void, layout: uint64_t);
    fn ffw_codec_parameters_set_extradata(
        params: *mut c_void,
        extradata: *const uint8_t,
        size: c_int,
    ) -> c_int;
    fn ffw_codec_parameters_free(params: *mut c_void);

    fn ffw_decoder_new(codec: *const c_char) -> *mut c_void;
    fn ffw_decoder_from_codec_parameters(params: *const c_void) -> *mut c_void;
    fn ffw_decoder_set_extradata(
        decoder: *mut c_void,
        extradata: *const uint8_t,
        size: c_int,
    ) -> c_int;
    fn ffw_decoder_open(decoder: *mut c_void) -> c_int;
    fn ffw_decoder_push_packet(decoder: *mut c_void, packet: *const c_void) -> c_int;
    fn ffw_decoder_take_frame(decoder: *mut c_void, frame: *mut *mut c_void) -> c_int;
    fn ffw_decoder_get_codec_parameters(decoder: *const c_void) -> *mut c_void;
    fn ffw_decoder_free(decoder: *mut c_void);

    fn ffw_encoder_new(codec: *const c_char) -> *mut c_void;
    fn ffw_encoder_from_codec_parameters(params: *const c_void) -> *mut c_void;
    fn ffw_encoder_get_codec_parameters(encoder: *const c_void) -> *mut c_void;
    fn ffw_encoder_get_pixel_format(encoder: *const c_void) -> c_int;
    fn ffw_encoder_get_width(encoder: *const c_void) -> c_int;
    fn ffw_encoder_get_height(encoder: *const c_void) -> c_int;
    fn ffw_encoder_get_sample_format(encoder: *const c_void) -> c_int;
    fn ffw_encoder_get_sample_rate(encoder: *const c_void) -> c_int;
    fn ffw_encoder_get_channel_layout(encoder: *const c_void) -> uint64_t;
    fn ffw_encoder_get_frame_size(encoder: *const c_void) -> c_int;
    fn ffw_encoder_set_time_base(encoder: *mut c_void, num: c_int, den: c_int);
    fn ffw_encoder_set_bit_rate(encoder: *mut c_void, bit_rate: int64_t);
    fn ffw_encoder_set_pixel_format(encoder: *mut c_void, format: c_int);
    fn ffw_encoder_set_width(encoder: *mut c_void, width: c_int);
    fn ffw_encoder_set_height(encoder: *mut c_void, height: c_int);
    fn ffw_encoder_set_sample_format(encoder: *mut c_void, format: c_int);
    fn ffw_encoder_set_sample_rate(encoder: *mut c_void, sample_rate: c_int);
    fn ffw_encoder_set_channel_layout(encoder: *mut c_void, channel_layout: uint64_t);
    fn ffw_encoder_open(encoder: *mut c_void) -> c_int;
    fn ffw_encoder_push_frame(encoder: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_encoder_take_packet(encoder: *mut c_void, packet: *mut *mut c_void) -> c_int;
    fn ffw_encoder_free(encoder: *mut c_void);
}

/// A type of a decoding or an encoding error.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorKind {
    /// An error.
    Error,
    /// An error indicating that another operation needs to be done before
    /// continuing with the current operation.
    Again,
}

/// A decoding or encoding error.
#[derive(Debug, Clone)]
pub struct CodecError {
    kind: ErrorKind,
    msg: String,
}

impl CodecError {
    /// Create a new error.
    pub fn new<T>(kind: ErrorKind, msg: T) -> CodecError
    where
        T: ToString,
    {
        CodecError {
            kind: kind,
            msg: msg.to_string(),
        }
    }

    /// Get error kind.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl Display for CodecError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for CodecError {
    fn description(&self) -> &str {
        &self.msg
    }
}

/// Codec parameters.
pub struct CodecParameters {
    ptr: *mut c_void,
}

impl CodecParameters {
    /// Create codec parameters from a given raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> CodecParameters {
        CodecParameters { ptr: ptr }
    }

    /// Get raw pointer to the underlying object.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Check if these codec parameters are for an audio codec.
    pub fn is_audio_codec(&self) -> bool {
        unsafe { ffw_codec_parameters_is_audio_codec(self.ptr) != 0 }
    }

    /// Check if these codec parameters are for a video codec.
    pub fn is_video_codec(&self) -> bool {
        unsafe { ffw_codec_parameters_is_video_codec(self.ptr) != 0 }
    }

    /// Convert this object into audio codec parameters (if possible).
    pub fn into_audio_codec_parameters(self) -> Option<AudioCodecParameters> {
        if self.is_audio_codec() {
            let res = AudioCodecParameters { inner: self };

            Some(res)
        } else {
            None
        }
    }

    /// Convert this object into video codec parameters (if possible).
    pub fn into_video_codec_parameters(self) -> Option<VideoCodecParameters> {
        if self.is_video_codec() {
            let res = VideoCodecParameters { inner: self };

            Some(res)
        } else {
            None
        }
    }
}

impl Drop for CodecParameters {
    fn drop(&mut self) {
        unsafe { ffw_codec_parameters_free(self.ptr) }
    }
}

impl Clone for CodecParameters {
    fn clone(&self) -> CodecParameters {
        let ptr = unsafe { ffw_codec_parameters_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone codec parameters");
        }

        CodecParameters { ptr: ptr }
    }
}

unsafe impl Send for CodecParameters {}
unsafe impl Sync for CodecParameters {}

/// Builder for audio codec parameters.
pub struct AudioCodecParametersBuilder {
    inner: CodecParameters,
}

impl AudioCodecParametersBuilder {
    /// Create a new builder for a given audio codec.
    fn new(codec: &str) -> Result<AudioCodecParametersBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_audio_codec_parameters_new(codec.as_ptr() as *const _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        let res = AudioCodecParametersBuilder { inner: params };

        Ok(res)
    }

    /// Set bit rate.
    pub fn bit_rate(self, bit_rate: u64) -> AudioCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_bit_rate(self.inner.ptr, bit_rate as _);
        }

        self
    }

    /// Set frame sample format.
    pub fn sample_format(self, format: SampleFormat) -> AudioCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_format(self.inner.ptr, format as _);
        }

        self
    }

    /// Set sampling rate.
    pub fn sample_rate(self, rate: u32) -> AudioCodecParametersBuilder {
        assert!(rate > 0);

        unsafe {
            ffw_codec_parameters_set_sample_rate(self.inner.ptr, rate as _);
        }

        self
    }

    /// Set channel layout.
    pub fn channel_layout(self, layout: ChannelLayout) -> AudioCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_channel_layout(self.inner.ptr, layout as _);
        }

        self
    }

    /// Set extradata.
    pub fn extradata(self, data: Option<&[u8]>) -> AudioCodecParametersBuilder {
        let ptr;
        let size;

        if let Some(data) = data {
            ptr = data.as_ptr();
            size = data.len();
        } else {
            ptr = ptr::null();
            size = 0;
        }

        let res = unsafe { ffw_codec_parameters_set_extradata(self.inner.ptr, ptr, size as _) };

        if res < 0 {
            panic!("unable to allocate extradata");
        }

        self
    }

    /// Build the codec parameters.
    pub fn build(self) -> AudioCodecParameters {
        AudioCodecParameters { inner: self.inner }
    }
}

impl From<AudioCodecParameters> for AudioCodecParametersBuilder {
    fn from(params: AudioCodecParameters) -> AudioCodecParametersBuilder {
        AudioCodecParametersBuilder {
            inner: params.inner,
        }
    }
}

/// Audio codec parameters.
#[derive(Clone)]
pub struct AudioCodecParameters {
    inner: CodecParameters,
}

impl AudioCodecParameters {
    /// Get builder for audio codec parameters.
    pub fn builder(codec: &str) -> Result<AudioCodecParametersBuilder, Error> {
        AudioCodecParametersBuilder::new(codec)
    }

    /// Get raw pointer to the underlying object.
    pub fn as_ptr(&self) -> *const c_void {
        self.inner.ptr
    }

    /// Get bit rate.
    pub fn bit_rate(&self) -> u64 {
        unsafe { ffw_codec_parameters_get_bit_rate(self.inner.ptr) as _ }
    }

    /// Get frame sample format.
    pub fn sample_format(&self) -> SampleFormat {
        unsafe { ffw_codec_parameters_get_format(self.inner.ptr) as _ }
    }

    /// Get sampling rate.
    pub fn sample_rate(&self) -> u32 {
        unsafe { ffw_codec_parameters_get_sample_rate(self.inner.ptr) as _ }
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> ChannelLayout {
        unsafe { ffw_codec_parameters_get_channel_layout(self.inner.ptr) as _ }
    }

    /// Get extradata.
    pub fn extradata(&self) -> Option<&[u8]> {
        unsafe {
            let data = ffw_codec_parameters_get_extradata(self.inner.ptr) as *const u8;
            let size = ffw_codec_parameters_get_extradata_size(self.inner.ptr) as usize;

            if data.is_null() {
                None
            } else {
                Some(slice::from_raw_parts(data, size))
            }
        }
    }

    /// Convert this object into general codec parameters.
    pub fn into_codec_parameters(self) -> CodecParameters {
        self.inner
    }
}

impl AsRef<CodecParameters> for AudioCodecParameters {
    fn as_ref(&self) -> &CodecParameters {
        &self.inner
    }
}

/// Builder for video codec parameters.
pub struct VideoCodecParametersBuilder {
    inner: CodecParameters,
}

impl VideoCodecParametersBuilder {
    /// Create a new builder for a given video codec.
    fn new(codec: &str) -> Result<VideoCodecParametersBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_video_codec_parameters_new(codec.as_ptr() as *const _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        let res = VideoCodecParametersBuilder { inner: params };

        Ok(res)
    }

    /// Set bit rate.
    pub fn bit_rate(self, bit_rate: u64) -> VideoCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_bit_rate(self.inner.ptr, bit_rate as _);
        }

        self
    }

    /// Set frame pixel format.
    pub fn pixel_format(self, format: PixelFormat) -> VideoCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_format(self.inner.ptr, format as _);
        }

        self
    }

    /// Set frame width.
    pub fn width(self, width: usize) -> VideoCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_width(self.inner.ptr, width as _);
        }

        self
    }

    /// Set frame height.
    pub fn height(self, height: usize) -> VideoCodecParametersBuilder {
        unsafe {
            ffw_codec_parameters_set_height(self.inner.ptr, height as _);
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

        let res = unsafe { ffw_codec_parameters_set_extradata(self.inner.ptr, ptr, size as _) };

        if res < 0 {
            panic!("unable to allocate extradata");
        }

        self
    }

    /// Build the codec parameters.
    pub fn build(self) -> VideoCodecParameters {
        VideoCodecParameters { inner: self.inner }
    }
}

impl From<VideoCodecParameters> for VideoCodecParametersBuilder {
    fn from(params: VideoCodecParameters) -> VideoCodecParametersBuilder {
        VideoCodecParametersBuilder {
            inner: params.inner,
        }
    }
}

/// Video codec parameters.
#[derive(Clone)]
pub struct VideoCodecParameters {
    inner: CodecParameters,
}

impl VideoCodecParameters {
    /// Get builder for video codec parameters.
    pub fn builder(codec: &str) -> Result<VideoCodecParametersBuilder, Error> {
        VideoCodecParametersBuilder::new(codec)
    }

    /// Get raw pointer to the underlying object.
    pub fn as_ptr(&self) -> *const c_void {
        self.inner.ptr
    }

    /// Get bit rate.
    pub fn bit_rate(&self) -> u64 {
        unsafe { ffw_codec_parameters_get_bit_rate(self.inner.ptr) as _ }
    }

    /// Get frame pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { ffw_codec_parameters_get_format(self.inner.ptr) as _ }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_codec_parameters_get_width(self.inner.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_codec_parameters_get_height(self.inner.ptr) as _ }
    }

    /// Get extradata.
    pub fn extradata(&self) -> Option<&[u8]> {
        unsafe {
            let data = ffw_codec_parameters_get_extradata(self.inner.ptr) as *const u8;
            let size = ffw_codec_parameters_get_extradata_size(self.inner.ptr) as usize;

            if data.is_null() {
                None
            } else {
                Some(slice::from_raw_parts(data, size))
            }
        }
    }

    /// Convert this object into general codec parameters.
    pub fn into_codec_parameters(self) -> CodecParameters {
        self.inner
    }
}

impl AsRef<CodecParameters> for VideoCodecParameters {
    fn as_ref(&self) -> &CodecParameters {
        &self.inner
    }
}
