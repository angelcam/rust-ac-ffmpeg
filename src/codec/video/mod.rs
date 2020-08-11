//! Video decoder/encoder.

pub mod frame;
pub mod scaler;

use std::{ffi::CString, ptr};

use libc::c_void;

use crate::{
    codec::{CodecError, CodecParameters, Decoder, Encoder, VideoCodecParameters},
    packet::Packet,
    time::TimeBase,
    Error,
};

pub use self::{
    frame::{PixelFormat, VideoFrame, VideoFrameMut},
    scaler::{VideoFrameScaler, VideoFrameScalerBuilder},
};

/// Builder for the video decoder.
pub struct VideoDecoderBuilder {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl VideoDecoderBuilder {
    /// Create a new builder for a given codec.
    fn new(codec: &str) -> Result<Self, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_decoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let res = Self {
            ptr,
            time_base: TimeBase::MICROSECONDS,
        };

        Ok(res)
    }

    /// Create a new builder from given codec parameters.
    fn from_codec_parameters(codec_parameters: &VideoCodecParameters) -> Result<Self, Error> {
        let ptr = unsafe { super::ffw_decoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create a decoder"));
        }

        let res = Self {
            ptr,
            time_base: TimeBase::MICROSECONDS,
        };

        Ok(res)
    }

    /// Set a decoder option.
    pub fn set_option<V>(self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            super::ffw_decoder_set_initial_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _)
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Set decoder time base (all input packets will be rescaled into this
    /// time base). The default time base is in microseconds.
    pub fn time_base(mut self, time_base: TimeBase) -> Self {
        self.time_base = time_base;
        self
    }

    /// Set codec extradata.
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

        let res = unsafe { super::ffw_decoder_set_extradata(self.ptr, ptr, size as _) };

        if res < 0 {
            panic!("unable to allocate extradata");
        }

        self
    }

    /// Build the decoder.
    pub fn build(mut self) -> Result<VideoDecoder, Error> {
        unsafe {
            if super::ffw_decoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the decoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = VideoDecoder {
            ptr,
            time_base: self.time_base,
        };

        Ok(res)
    }
}

impl Drop for VideoDecoderBuilder {
    fn drop(&mut self) {
        unsafe { super::ffw_decoder_free(self.ptr) }
    }
}

unsafe impl Send for VideoDecoderBuilder {}
unsafe impl Sync for VideoDecoderBuilder {}

/// Video decoder.
pub struct VideoDecoder {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl VideoDecoder {
    /// Create a new video decoder for a given codec.
    pub fn new(codec: &str) -> Result<Self, Error> {
        VideoDecoderBuilder::new(codec).and_then(|builder| builder.build())
    }

    /// Create a new video decoder builder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &VideoCodecParameters,
    ) -> Result<VideoDecoderBuilder, Error> {
        VideoDecoderBuilder::from_codec_parameters(codec_parameters)
    }

    /// Get decoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<VideoDecoderBuilder, Error> {
        VideoDecoderBuilder::new(codec)
    }
}

impl Decoder for VideoDecoder {
    type CodecParameters = VideoCodecParameters;
    type Frame = VideoFrame;

    fn codec_parameters(&self) -> VideoCodecParameters {
        let ptr = unsafe { super::ffw_decoder_get_codec_parameters(self.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        params.into_video_codec_parameters().unwrap()
    }

    fn push(&mut self, packet: Packet) -> Result<(), CodecError> {
        let packet = packet.with_time_base(self.time_base);

        unsafe {
            match super::ffw_decoder_push_packet(self.ptr, packet.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all frames must be consumed before pushing a new packet",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_decoder_push_packet(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all frames must be consumed before flushing",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn take(&mut self) -> Result<Option<VideoFrame>, CodecError> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match super::ffw_decoder_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(VideoFrame::from_raw_ptr(fptr, self.time_base)))
                    }
                }
                0 => Ok(None),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        unsafe { super::ffw_decoder_free(self.ptr) }
    }
}

unsafe impl Send for VideoDecoder {}
unsafe impl Sync for VideoDecoder {}

/// Builder for the video encoder.
pub struct VideoEncoderBuilder {
    ptr: *mut c_void,

    time_base: TimeBase,

    format: Option<PixelFormat>,
    width: Option<usize>,
    height: Option<usize>,
}

impl VideoEncoderBuilder {
    /// Create a new encoder builder for a given codec.
    fn new(codec: &str) -> Result<Self, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_encoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        unsafe {
            super::ffw_encoder_set_bit_rate(ptr, 0);
        }

        let res = Self {
            ptr,

            time_base: TimeBase::MICROSECONDS,

            format: None,
            width: None,
            height: None,
        };

        Ok(res)
    }

    /// Create a new encoder builder from given codec parameters.
    fn from_codec_parameters(codec_parameters: &VideoCodecParameters) -> Result<Self, Error> {
        let ptr = unsafe { super::ffw_encoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create an encoder"));
        }

        let pixel_format;
        let width;
        let height;

        unsafe {
            pixel_format = PixelFormat::from_raw(super::ffw_encoder_get_pixel_format(ptr));
            width = super::ffw_encoder_get_width(ptr) as _;
            height = super::ffw_encoder_get_height(ptr) as _;
        }

        let res = Self {
            ptr,

            time_base: TimeBase::MICROSECONDS,

            format: Some(pixel_format),
            width: Some(width),
            height: Some(height),
        };

        Ok(res)
    }

    /// Set an encoder option.
    pub fn set_option<V>(self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            super::ffw_encoder_set_initial_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _)
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Set encoder bit rate. The default is 0 (i.e. automatic).
    pub fn bit_rate(self, bit_rate: u64) -> Self {
        unsafe {
            super::ffw_encoder_set_bit_rate(self.ptr, bit_rate as _);
        }

        self
    }

    /// Set encoder time base. The default time base is in microseconds.
    pub fn time_base(mut self, time_base: TimeBase) -> Self {
        self.time_base = time_base;
        self
    }

    /// Set encoder pixel format.
    pub fn pixel_format(mut self, format: PixelFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set frame width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set frame height.
    pub fn height(mut self, height: usize) -> Self {
        self.height = Some(height);
        self
    }

    /// Build the encoder.
    pub fn build(mut self) -> Result<VideoEncoder, Error> {
        let format = self
            .format
            .ok_or_else(|| Error::new("pixel format not set"))?;

        let width = self.width.ok_or_else(|| Error::new("width not set"))?;
        let height = self.height.ok_or_else(|| Error::new("height not set"))?;

        let tb = self.time_base;

        unsafe {
            super::ffw_encoder_set_time_base(self.ptr, tb.num() as _, tb.den() as _);
            super::ffw_encoder_set_pixel_format(self.ptr, format.into_raw());
            super::ffw_encoder_set_width(self.ptr, width as _);
            super::ffw_encoder_set_height(self.ptr, height as _);

            if super::ffw_encoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the encoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = VideoEncoder { ptr, time_base: tb };

        Ok(res)
    }
}

impl Drop for VideoEncoderBuilder {
    fn drop(&mut self) {
        unsafe { super::ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for VideoEncoderBuilder {}
unsafe impl Sync for VideoEncoderBuilder {}

/// Video encoder.
pub struct VideoEncoder {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl VideoEncoder {
    /// Create a new encoder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &VideoCodecParameters,
    ) -> Result<VideoEncoderBuilder, Error> {
        VideoEncoderBuilder::from_codec_parameters(codec_parameters)
    }

    /// Get encoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<VideoEncoderBuilder, Error> {
        VideoEncoderBuilder::new(codec)
    }
}

impl Encoder for VideoEncoder {
    type CodecParameters = VideoCodecParameters;
    type Frame = VideoFrame;

    fn codec_parameters(&self) -> VideoCodecParameters {
        let ptr = unsafe { super::ffw_encoder_get_codec_parameters(self.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        params.into_video_codec_parameters().unwrap()
    }

    fn push(&mut self, frame: VideoFrame) -> Result<(), CodecError> {
        let frame = frame.with_time_base(self.time_base);

        unsafe {
            match super::ffw_encoder_push_frame(self.ptr, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all packets must be consumed before pushing a new frame",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_encoder_push_frame(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all packets must be consumed before flushing",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn take(&mut self) -> Result<Option<Packet>, CodecError> {
        let mut pptr = ptr::null_mut();

        unsafe {
            match super::ffw_encoder_take_packet(self.ptr, &mut pptr) {
                1 => {
                    if pptr.is_null() {
                        panic!("no packet received")
                    } else {
                        Ok(Some(Packet::from_raw_ptr(pptr, self.time_base)))
                    }
                }
                0 => Ok(None),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }
}

impl Drop for VideoEncoder {
    fn drop(&mut self) {
        unsafe { super::ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for VideoEncoder {}
unsafe impl Sync for VideoEncoder {}
