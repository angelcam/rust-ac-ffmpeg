mod frame;

pub mod scaler;

use std::ptr;

use std::ffi::CString;

use libc::c_void;

use crate::Error;

use crate::codec::{CodecError, CodecParameters, ErrorKind};
use crate::packet::Packet;

pub use self::frame::{PixelFormat, VideoFrame, VideoFrameMut};
pub use self::scaler::{VideoFrameScaler, VideoFrameScalerBuilder};

/// Builder for the video decoder.
pub struct VideoDecoderBuilder {
    ptr: *mut c_void,
}

impl VideoDecoderBuilder {
    /// Create a new builder for a given codec.
    fn new(codec: &str) -> Result<VideoDecoderBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_decoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let res = VideoDecoderBuilder { ptr: ptr };

        Ok(res)
    }

    /// Set codec extradata.
    pub fn extradata(self, data: Option<&[u8]>) -> VideoDecoderBuilder {
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

        let res = VideoDecoder { ptr: ptr };

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
///
/// # Decoder operation
/// 1. Push a packet to the decoder.
/// 2. Take all frames from the decoder until you get None.
/// 3. If there are more packets to be decoded, continue with 1.
/// 4. Flush the decoder.
/// 5. Take all frames from the decoder until you get None.
pub struct VideoDecoder {
    ptr: *mut c_void,
}

impl VideoDecoder {
    /// Create a new video decoder for a given codec.
    pub fn new(codec: &str) -> Result<VideoDecoder, Error> {
        VideoDecoderBuilder::new(codec).and_then(|builder| builder.build())
    }

    /// Create a new decoder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &CodecParameters,
    ) -> Result<VideoDecoder, Error> {
        assert!(codec_parameters.is_video_codec());

        let ptr = unsafe { super::ffw_decoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create a decoder"));
        }

        let res = VideoDecoder { ptr: ptr };

        Ok(res)
    }

    /// Get decoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<VideoDecoderBuilder, Error> {
        VideoDecoderBuilder::new(codec)
    }

    /// Push a given packet to the decoder.
    pub fn push(&mut self, packet: &Packet) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_decoder_push_packet(self.ptr, packet.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::new(
                    ErrorKind::Again,
                    "all frames must be consumed before pushing a new packet",
                )),
                _ => Err(CodecError::new(ErrorKind::Error, "decoding error")),
            }
        }
    }

    /// Flush the decoder.
    pub fn flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_decoder_push_packet(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::new(
                    ErrorKind::Again,
                    "all frames must be consumed before flushing",
                )),
                _ => Err(CodecError::new(ErrorKind::Error, "decoding error")),
            }
        }
    }

    /// Take the next decoded frame from the decoder.
    pub fn take(&mut self) -> Result<Option<VideoFrame>, CodecError> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match super::ffw_decoder_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(VideoFrame::from_raw_ptr(fptr)))
                    }
                }
                0 => Ok(None),
                _ => Err(CodecError::new(ErrorKind::Error, "decoding error")),
            }
        }
    }

    /// Get codec parameters.
    pub fn codec_parameters(&self) -> CodecParameters {
        let ptr = unsafe { super::ffw_decoder_get_codec_parameters(self.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        unsafe { CodecParameters::from_raw_ptr(ptr) }
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

    format: Option<PixelFormat>,
    width: Option<usize>,
    height: Option<usize>,
}

impl VideoEncoderBuilder {
    /// Create a new encoder builder for a given codec.
    fn new(codec: &str) -> Result<VideoEncoderBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_encoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        unsafe {
            super::ffw_encoder_set_bit_rate(ptr, 0);
            super::ffw_encoder_set_time_base(ptr, 1, 1000);
        }

        let res = VideoEncoderBuilder {
            ptr: ptr,

            format: None,
            width: None,
            height: None,
        };

        Ok(res)
    }

    /// Create a new encoder builder from given codec parameters.
    fn from_codec_parameters(
        codec_parameters: &CodecParameters,
    ) -> Result<VideoEncoderBuilder, Error> {
        assert!(codec_parameters.is_video_codec());

        let ptr = unsafe { super::ffw_encoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create an encoder"));
        }

        let pixel_format;
        let width;
        let height;

        unsafe {
            pixel_format = super::ffw_encoder_get_pixel_format(ptr) as _;
            width = super::ffw_encoder_get_width(ptr) as _;
            height = super::ffw_encoder_get_height(ptr) as _;
        }

        let res = VideoEncoderBuilder {
            ptr: ptr,

            format: Some(pixel_format),
            width: Some(width),
            height: Some(height),
        };

        Ok(res)
    }

    /// Set encoder bit rate. The default is 0 (i.e. automatic).
    pub fn bit_rate(self, bit_rate: u64) -> VideoEncoderBuilder {
        unsafe {
            super::ffw_encoder_set_bit_rate(self.ptr, bit_rate as _);
        }

        self
    }

    /// Set encoder time base as a rational number. The default is 1/1000.
    pub fn time_base(self, num: u32, den: u32) -> VideoEncoderBuilder {
        unsafe {
            super::ffw_encoder_set_time_base(self.ptr, num as _, den as _);
        }

        self
    }

    /// Set encoder pixel format.
    pub fn pixel_format(mut self, format: PixelFormat) -> VideoEncoderBuilder {
        self.format = Some(format);
        self
    }

    /// Set frame width.
    pub fn width(mut self, width: usize) -> VideoEncoderBuilder {
        self.width = Some(width);
        self
    }

    /// Set frame height.
    pub fn height(mut self, height: usize) -> VideoEncoderBuilder {
        self.height = Some(height);
        self
    }

    /// Build the encoder.
    pub fn build(mut self) -> Result<VideoEncoder, Error> {
        let format = self.format.ok_or(Error::new("pixel format not set"))?;
        let width = self.width.ok_or(Error::new("width not set"))?;
        let height = self.height.ok_or(Error::new("height not set"))?;

        unsafe {
            super::ffw_encoder_set_pixel_format(self.ptr, format);
            super::ffw_encoder_set_width(self.ptr, width as _);
            super::ffw_encoder_set_height(self.ptr, height as _);

            if super::ffw_encoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the encoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = VideoEncoder { ptr: ptr };

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
///
/// # Encoder operation
/// 1. Push a frame to the encoder.
/// 2. Take all packets from the encoder until you get None.
/// 3. If there are more frames to be encoded, continue with 1.
/// 4. Flush the encoder.
/// 5. Take all packets from the encoder until you get None.
pub struct VideoEncoder {
    ptr: *mut c_void,
}

impl VideoEncoder {
    /// Get encoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<VideoEncoderBuilder, Error> {
        VideoEncoderBuilder::new(codec)
    }

    /// Create a new encoder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &CodecParameters,
    ) -> Result<VideoEncoderBuilder, Error> {
        VideoEncoderBuilder::from_codec_parameters(codec_parameters)
    }

    /// Push a given frame to the encoder.
    pub fn push(&mut self, frame: &VideoFrame) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_encoder_push_frame(self.ptr, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::new(
                    ErrorKind::Again,
                    "all packets must be consumed before pushing a new frame",
                )),
                _ => Err(CodecError::new(ErrorKind::Error, "encoding error")),
            }
        }
    }

    /// Flush the encoder.
    pub fn flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_encoder_push_frame(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::new(
                    ErrorKind::Again,
                    "all packets must be consumed before flushing",
                )),
                _ => Err(CodecError::new(ErrorKind::Error, "encoding error")),
            }
        }
    }

    /// Take the next packet from the encoder.
    pub fn take(&mut self) -> Result<Option<Packet>, CodecError> {
        let mut pptr = ptr::null_mut();

        unsafe {
            match super::ffw_encoder_take_packet(self.ptr, &mut pptr) {
                1 => {
                    if pptr.is_null() {
                        panic!("no packet received")
                    } else {
                        Ok(Some(Packet::from_raw_ptr(pptr)))
                    }
                }
                0 => Ok(None),
                _ => Err(CodecError::new(ErrorKind::Error, "encoding error")),
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
