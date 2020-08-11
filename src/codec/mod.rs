//! A/V codecs.

pub mod audio;
pub mod bsf;
pub mod video;

use std::{
    ffi::{CStr, CString},
    fmt::{self, Display, Formatter},
    ptr, slice,
};

use libc::{c_char, c_int, c_void};

use crate::{
    codec::{
        audio::{ChannelLayout, SampleFormat},
        video::PixelFormat,
    },
    packet::Packet,
    Error,
};

extern "C" {
    fn ffw_audio_codec_parameters_new(codec: *const c_char) -> *mut c_void;
    fn ffw_video_codec_parameters_new(codec: *const c_char) -> *mut c_void;
    fn ffw_codec_parameters_clone(params: *const c_void) -> *mut c_void;
    fn ffw_codec_parameters_is_audio_codec(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_is_video_codec(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_decoder_name(params: *const c_void) -> *const c_char;
    fn ffw_codec_parameters_get_encoder_name(params: *const c_void) -> *const c_char;
    fn ffw_codec_parameters_get_bit_rate(params: *const c_void) -> i64;
    fn ffw_codec_parameters_get_format(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_width(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_height(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_sample_rate(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_get_channel_layout(params: *const c_void) -> u64;
    fn ffw_codec_parameters_get_extradata(params: *mut c_void) -> *mut c_void;
    fn ffw_codec_parameters_get_extradata_size(params: *const c_void) -> c_int;
    fn ffw_codec_parameters_set_bit_rate(params: *mut c_void, bit_rate: i64);
    fn ffw_codec_parameters_set_format(params: *mut c_void, format: c_int);
    fn ffw_codec_parameters_set_width(params: *mut c_void, width: c_int);
    fn ffw_codec_parameters_set_height(params: *mut c_void, height: c_int);
    fn ffw_codec_parameters_set_sample_rate(params: *mut c_void, rate: c_int);
    fn ffw_codec_parameters_set_channel_layout(params: *mut c_void, layout: u64);
    fn ffw_codec_parameters_set_extradata(
        params: *mut c_void,
        extradata: *const u8,
        size: c_int,
    ) -> c_int;
    fn ffw_codec_parameters_free(params: *mut c_void);

    fn ffw_decoder_new(codec: *const c_char) -> *mut c_void;
    fn ffw_decoder_from_codec_parameters(params: *const c_void) -> *mut c_void;
    fn ffw_decoder_set_extradata(decoder: *mut c_void, extradata: *const u8, size: c_int) -> c_int;
    fn ffw_decoder_set_initial_option(
        decoder: *mut c_void,
        key: *const c_char,
        value: *const c_char,
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
    fn ffw_encoder_get_channel_layout(encoder: *const c_void) -> u64;
    fn ffw_encoder_get_frame_size(encoder: *const c_void) -> c_int;
    fn ffw_encoder_set_time_base(encoder: *mut c_void, num: c_int, den: c_int);
    fn ffw_encoder_set_bit_rate(encoder: *mut c_void, bit_rate: i64);
    fn ffw_encoder_set_pixel_format(encoder: *mut c_void, format: c_int);
    fn ffw_encoder_set_width(encoder: *mut c_void, width: c_int);
    fn ffw_encoder_set_height(encoder: *mut c_void, height: c_int);
    fn ffw_encoder_set_sample_format(encoder: *mut c_void, format: c_int);
    fn ffw_encoder_set_sample_rate(encoder: *mut c_void, sample_rate: c_int);
    fn ffw_encoder_set_channel_layout(encoder: *mut c_void, channel_layout: u64);
    fn ffw_encoder_set_initial_option(
        encoder: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_encoder_open(encoder: *mut c_void) -> c_int;
    fn ffw_encoder_push_frame(encoder: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_encoder_take_packet(encoder: *mut c_void, packet: *mut *mut c_void) -> c_int;
    fn ffw_encoder_free(encoder: *mut c_void);
}

/// Error variants.
#[derive(Debug, Clone)]
enum CodecErrorVariant {
    /// An error.
    Error(Error),
    /// An error indicating that another operation needs to be done before
    /// continuing with the current operation.
    Again(&'static str),
}

/// A decoding or encoding error.
#[derive(Debug, Clone)]
pub struct CodecError {
    variant: CodecErrorVariant,
}

impl CodecError {
    /// Create a new error.
    fn error<T>(msg: T) -> Self
    where
        T: ToString,
    {
        Self {
            variant: CodecErrorVariant::Error(Error::new(msg)),
        }
    }

    /// Create a new FFmpeg error from a given FFmpeg error code.
    fn from_raw_error_code(code: c_int) -> Self {
        Self {
            variant: CodecErrorVariant::Error(Error::from_raw_error_code(code)),
        }
    }

    /// Create a new error indicating that another operation needs to be done.
    fn again(msg: &'static str) -> Self {
        Self {
            variant: CodecErrorVariant::Again(msg),
        }
    }

    /// Check if another operation needs to be done.
    pub fn is_again(&self) -> bool {
        if let CodecErrorVariant::Again(_) = &self.variant {
            true
        } else {
            false
        }
    }

    /// Get the inner error (if any).
    pub fn into_inner(self) -> Option<Error> {
        if let CodecErrorVariant::Error(err) = self.variant {
            Some(err)
        } else {
            None
        }
    }
}

impl Display for CodecError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match &self.variant {
            CodecErrorVariant::Again(msg) => write!(f, "{}", msg),
            CodecErrorVariant::Error(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for CodecError {}

/// Inner struct holding the pointer to the codec parameters.
struct InnerCodecParameters {
    ptr: *mut c_void,
}

impl InnerCodecParameters {
    /// Create codec parameters from a given raw representation.
    unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        Self { ptr }
    }

    /// Get raw pointer to the underlying object.
    fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Check if these codec parameters are for an audio codec.
    fn is_audio_codec(&self) -> bool {
        unsafe { ffw_codec_parameters_is_audio_codec(self.ptr) != 0 }
    }

    /// Check if these codec parameters are for a video codec.
    fn is_video_codec(&self) -> bool {
        unsafe { ffw_codec_parameters_is_video_codec(self.ptr) != 0 }
    }

    /// Get name of the decoder that is able to decode this codec or None
    /// if the decoder is not available.
    fn decoder_name(&self) -> Option<&'static str> {
        unsafe {
            let ptr = ffw_codec_parameters_get_decoder_name(self.ptr);

            if ptr.is_null() {
                None
            } else {
                let name = CStr::from_ptr(ptr as _);

                Some(name.to_str().unwrap())
            }
        }
    }

    /// Get name of the encoder that is able to produce encoding of this codec
    /// or None if the encoder is not available.
    fn encoder_name(&self) -> Option<&'static str> {
        unsafe {
            let ptr = ffw_codec_parameters_get_encoder_name(self.ptr);

            if ptr.is_null() {
                None
            } else {
                let name = CStr::from_ptr(ptr as _);

                Some(name.to_str().unwrap())
            }
        }
    }
}

impl Drop for InnerCodecParameters {
    fn drop(&mut self) {
        unsafe { ffw_codec_parameters_free(self.ptr) }
    }
}

impl Clone for InnerCodecParameters {
    fn clone(&self) -> Self {
        let ptr = unsafe { ffw_codec_parameters_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone codec parameters");
        }

        Self { ptr }
    }
}

unsafe impl Send for InnerCodecParameters {}
unsafe impl Sync for InnerCodecParameters {}

/// Variants of codec parameters.
#[derive(Clone)]
enum CodecParametersVariant {
    Audio(AudioCodecParameters),
    Video(VideoCodecParameters),
    Other(OtherCodecParameters),
}

impl CodecParametersVariant {
    /// Create codec parameters from a given raw representation.
    unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        let inner = InnerCodecParameters::from_raw_ptr(ptr);

        if inner.is_audio_codec() {
            Self::Audio(AudioCodecParameters::from(inner))
        } else if inner.is_video_codec() {
            Self::Video(VideoCodecParameters::from(inner))
        } else {
            Self::Other(OtherCodecParameters::from(inner))
        }
    }
}

impl AsRef<InnerCodecParameters> for CodecParametersVariant {
    fn as_ref(&self) -> &InnerCodecParameters {
        match self {
            Self::Audio(audio) => audio.as_ref(),
            Self::Video(video) => video.as_ref(),
            Self::Other(other) => other.as_ref(),
        }
    }
}

/// Codec parameters.
#[derive(Clone)]
pub struct CodecParameters {
    inner: CodecParametersVariant,
}

impl CodecParameters {
    /// Create codec parameters from a given raw representation.
    pub(crate) unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        Self {
            inner: CodecParametersVariant::from_raw_ptr(ptr),
        }
    }

    /// Get raw pointer to the underlying object.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.inner.as_ref().as_ptr()
    }

    /// Check if these codec parameters are for an audio codec.
    pub fn is_audio_codec(&self) -> bool {
        self.inner.as_ref().is_audio_codec()
    }

    /// Check if these codec parameters are for a video codec.
    pub fn is_video_codec(&self) -> bool {
        self.inner.as_ref().is_video_codec()
    }

    /// Get name of the decoder that is able to decode this codec or None
    /// if the decoder is not available.
    pub fn decoder_name(&self) -> Option<&'static str> {
        self.inner.as_ref().decoder_name()
    }

    /// Get name of the encoder that is able to produce encoding of this codec
    /// or None if the encoder is not available.
    pub fn encoder_name(&self) -> Option<&'static str> {
        self.inner.as_ref().encoder_name()
    }

    /// Get reference to audio codec parameters (if possible).
    pub fn as_audio_codec_parameters(&self) -> Option<&AudioCodecParameters> {
        if let CodecParametersVariant::Audio(params) = &self.inner {
            Some(params)
        } else {
            None
        }
    }

    /// Get reference to video codec parameters (if possible).
    pub fn as_video_codec_parameters(&self) -> Option<&VideoCodecParameters> {
        if let CodecParametersVariant::Video(params) = &self.inner {
            Some(params)
        } else {
            None
        }
    }

    /// Convert this object into audio codec parameters (if possible).
    pub fn into_audio_codec_parameters(self) -> Option<AudioCodecParameters> {
        if let CodecParametersVariant::Audio(params) = self.inner {
            Some(params)
        } else {
            None
        }
    }

    /// Convert this object into video codec parameters (if possible).
    pub fn into_video_codec_parameters(self) -> Option<VideoCodecParameters> {
        if let CodecParametersVariant::Video(params) = self.inner {
            Some(params)
        } else {
            None
        }
    }
}

impl From<AudioCodecParameters> for CodecParameters {
    fn from(params: AudioCodecParameters) -> Self {
        Self {
            inner: CodecParametersVariant::Audio(params),
        }
    }
}

impl From<VideoCodecParameters> for CodecParameters {
    fn from(params: VideoCodecParameters) -> Self {
        Self {
            inner: CodecParametersVariant::Video(params),
        }
    }
}

/// Builder for audio codec parameters.
pub struct AudioCodecParametersBuilder {
    inner: InnerCodecParameters,
}

impl AudioCodecParametersBuilder {
    /// Create a new builder for a given audio codec.
    fn new(codec: &str) -> Result<Self, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_audio_codec_parameters_new(codec.as_ptr() as *const _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let params = unsafe { InnerCodecParameters::from_raw_ptr(ptr) };

        let res = AudioCodecParametersBuilder { inner: params };

        Ok(res)
    }

    /// Set bit rate.
    pub fn bit_rate(self, bit_rate: u64) -> Self {
        unsafe {
            ffw_codec_parameters_set_bit_rate(self.inner.ptr, bit_rate as _);
        }

        self
    }

    /// Set frame sample format.
    pub fn sample_format(self, format: SampleFormat) -> Self {
        unsafe {
            ffw_codec_parameters_set_format(self.inner.ptr, format.into_raw());
        }

        self
    }

    /// Set sampling rate.
    pub fn sample_rate(self, rate: u32) -> Self {
        assert!(rate > 0);

        unsafe {
            ffw_codec_parameters_set_sample_rate(self.inner.ptr, rate as _);
        }

        self
    }

    /// Set channel layout.
    pub fn channel_layout(self, layout: ChannelLayout) -> Self {
        unsafe {
            ffw_codec_parameters_set_channel_layout(self.inner.ptr, layout.into_raw());
        }

        self
    }

    /// Set extradata.
    pub fn extradata<T>(self, data: Option<T>) -> Self
    where
        T: AsRef<[u8]>,
    {
        let data = data.as_ref().map(|d| d.as_ref());

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
    fn from(params: AudioCodecParameters) -> Self {
        Self {
            inner: params.inner,
        }
    }
}

/// Audio codec parameters.
#[derive(Clone)]
pub struct AudioCodecParameters {
    inner: InnerCodecParameters,
}

impl AudioCodecParameters {
    /// Get builder for audio codec parameters.
    pub fn builder(codec: &str) -> Result<AudioCodecParametersBuilder, Error> {
        AudioCodecParametersBuilder::new(codec)
    }

    /// Get raw pointer to the underlying object.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.inner.ptr
    }

    /// Get name of the decoder that is able to decode this codec or None
    /// if the decoder is not available.
    pub fn decoder_name(&self) -> Option<&'static str> {
        self.inner.decoder_name()
    }

    /// Get name of the encoder that is able to produce encoding of this codec
    /// or None if the encoder is not available.
    pub fn encoder_name(&self) -> Option<&'static str> {
        self.inner.encoder_name()
    }

    /// Get bit rate.
    pub fn bit_rate(&self) -> u64 {
        unsafe { ffw_codec_parameters_get_bit_rate(self.inner.ptr) as _ }
    }

    /// Get frame sample format.
    pub fn sample_format(&self) -> SampleFormat {
        unsafe { SampleFormat::from_raw(ffw_codec_parameters_get_format(self.inner.ptr)) }
    }

    /// Get sampling rate.
    pub fn sample_rate(&self) -> u32 {
        unsafe { ffw_codec_parameters_get_sample_rate(self.inner.ptr) as _ }
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> ChannelLayout {
        unsafe { ChannelLayout::from_raw(ffw_codec_parameters_get_channel_layout(self.inner.ptr)) }
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
}

impl AsRef<InnerCodecParameters> for AudioCodecParameters {
    fn as_ref(&self) -> &InnerCodecParameters {
        &self.inner
    }
}

impl From<InnerCodecParameters> for AudioCodecParameters {
    fn from(params: InnerCodecParameters) -> Self {
        Self { inner: params }
    }
}

/// Builder for video codec parameters.
pub struct VideoCodecParametersBuilder {
    inner: InnerCodecParameters,
}

impl VideoCodecParametersBuilder {
    /// Create a new builder for a given video codec.
    fn new(codec: &str) -> Result<Self, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_video_codec_parameters_new(codec.as_ptr() as *const _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let params = unsafe { InnerCodecParameters::from_raw_ptr(ptr) };

        let res = VideoCodecParametersBuilder { inner: params };

        Ok(res)
    }

    /// Set bit rate.
    pub fn bit_rate(self, bit_rate: u64) -> Self {
        unsafe {
            ffw_codec_parameters_set_bit_rate(self.inner.ptr, bit_rate as _);
        }

        self
    }

    /// Set frame pixel format.
    pub fn pixel_format(self, format: PixelFormat) -> Self {
        unsafe {
            ffw_codec_parameters_set_format(self.inner.ptr, format.into_raw());
        }

        self
    }

    /// Set frame width.
    pub fn width(self, width: usize) -> Self {
        unsafe {
            ffw_codec_parameters_set_width(self.inner.ptr, width as _);
        }

        self
    }

    /// Set frame height.
    pub fn height(self, height: usize) -> Self {
        unsafe {
            ffw_codec_parameters_set_height(self.inner.ptr, height as _);
        }

        self
    }

    /// Set extradata.
    pub fn extradata<T>(self, data: Option<T>) -> Self
    where
        T: AsRef<[u8]>,
    {
        let data = data.as_ref().map(|d| d.as_ref());

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
    inner: InnerCodecParameters,
}

impl VideoCodecParameters {
    /// Get builder for video codec parameters.
    pub fn builder(codec: &str) -> Result<VideoCodecParametersBuilder, Error> {
        VideoCodecParametersBuilder::new(codec)
    }

    /// Get raw pointer to the underlying object.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.inner.ptr
    }

    /// Get name of the decoder that is able to decode this codec or None
    /// if the decoder is not available.
    pub fn decoder_name(&self) -> Option<&'static str> {
        self.inner.decoder_name()
    }

    /// Get name of the encoder that is able to produce encoding of this codec
    /// or None if the encoder is not available.
    pub fn encoder_name(&self) -> Option<&'static str> {
        self.inner.encoder_name()
    }

    /// Get bit rate.
    pub fn bit_rate(&self) -> u64 {
        unsafe { ffw_codec_parameters_get_bit_rate(self.inner.ptr) as _ }
    }

    /// Get frame pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { PixelFormat::from_raw(ffw_codec_parameters_get_format(self.inner.ptr)) }
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
}

impl AsRef<InnerCodecParameters> for VideoCodecParameters {
    fn as_ref(&self) -> &InnerCodecParameters {
        &self.inner
    }
}

impl From<InnerCodecParameters> for VideoCodecParameters {
    fn from(params: InnerCodecParameters) -> Self {
        Self { inner: params }
    }
}

/// Other codec parameters.
#[derive(Clone)]
struct OtherCodecParameters {
    inner: InnerCodecParameters,
}

impl AsRef<InnerCodecParameters> for OtherCodecParameters {
    fn as_ref(&self) -> &InnerCodecParameters {
        &self.inner
    }
}

impl From<InnerCodecParameters> for OtherCodecParameters {
    fn from(params: InnerCodecParameters) -> Self {
        Self { inner: params }
    }
}

/// A media decoder.
///
/// # Common decoder operation
/// 1. Push a packet to the decoder.
/// 2. Take all frames from the decoder until you get None.
/// 3. If there are more packets to be decoded, continue with 1.
/// 4. Flush the decoder.
/// 5. Take all frames from the decoder until you get None.
pub trait Decoder {
    type CodecParameters;
    type Frame;

    /// Get codec parameters.
    fn codec_parameters(&self) -> Self::CodecParameters;

    /// Push a given packet to the decoder.
    fn push(&mut self, packet: Packet) -> Result<(), CodecError>;

    /// Flush the decoder.
    fn flush(&mut self) -> Result<(), CodecError>;

    /// Take the next frame from the decoder.
    fn take(&mut self) -> Result<Option<Self::Frame>, CodecError>;
}

/// A media encoder.
///
/// # Common encoder operation
/// 1. Push a frame to the encoder.
/// 2. Take all packets from the encoder until you get None.
/// 3. If there are more frames to be encoded, continue with 1.
/// 4. Flush the encoder.
/// 5. Take all packets from the encoder until you get None.
pub trait Encoder {
    type CodecParameters;
    type Frame;

    /// Get codec parameters.
    fn codec_parameters(&self) -> Self::CodecParameters;

    /// Push a given frame to the encoder.
    fn push(&mut self, frame: Self::Frame) -> Result<(), CodecError>;

    /// Flush the encoder.
    fn flush(&mut self) -> Result<(), CodecError>;

    /// Take the next packet from the encoder.
    fn take(&mut self) -> Result<Option<Packet>, CodecError>;
}
