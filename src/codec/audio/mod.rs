pub mod frame;
pub mod resampler;

use std::ptr;

use std::ffi::CString;

use libc::c_void;

use crate::Error;

use crate::codec::{AudioCodecParameters, CodecError, CodecParameters, ErrorKind};
use crate::packet::Packet;

pub use self::frame::{AudioFrame, AudioFrameMut, ChannelLayout, SampleFormat};
pub use self::resampler::AudioResampler;

/// Builder for the audio decoder.
pub struct AudioDecoderBuilder {
    ptr: *mut c_void,
}

impl AudioDecoderBuilder {
    /// Create a new builder for a given codec.
    fn new(codec: &str) -> Result<AudioDecoderBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_decoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let res = AudioDecoderBuilder { ptr: ptr };

        Ok(res)
    }

    /// Set codec extradata.
    pub fn extradata(self, data: Option<&[u8]>) -> AudioDecoderBuilder {
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
    pub fn build(mut self) -> Result<AudioDecoder, Error> {
        unsafe {
            if super::ffw_decoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the decoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = AudioDecoder { ptr: ptr };

        Ok(res)
    }
}

impl Drop for AudioDecoderBuilder {
    fn drop(&mut self) {
        unsafe { super::ffw_decoder_free(self.ptr) }
    }
}

unsafe impl Send for AudioDecoderBuilder {}
unsafe impl Sync for AudioDecoderBuilder {}

/// Audio decoder.
///
/// # Decoder operation
/// 1. Push a packet to the decoder.
/// 2. Take all frames from the decoder until you get None.
/// 3. If there are more packets to be decoded, continue with 1.
/// 4. Flush the decoder.
/// 5. Take all frames from the decoder until you get None.
pub struct AudioDecoder {
    ptr: *mut c_void,
}

impl AudioDecoder {
    /// Create a new audio decoder for a given codec.
    pub fn new(codec: &str) -> Result<AudioDecoder, Error> {
        AudioDecoderBuilder::new(codec).and_then(|builder| builder.build())
    }

    /// Create a new decoder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &AudioCodecParameters,
    ) -> Result<AudioDecoder, Error> {
        let ptr = unsafe { super::ffw_decoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create a decoder"));
        }

        let res = AudioDecoder { ptr: ptr };

        Ok(res)
    }

    /// Get decoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<AudioDecoderBuilder, Error> {
        AudioDecoderBuilder::new(codec)
    }

    /// Get codec parameters.
    pub fn codec_parameters(&self) -> AudioCodecParameters {
        let ptr = unsafe { super::ffw_decoder_get_codec_parameters(self.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        params.into_audio_codec_parameters().unwrap()
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
    pub fn take(&mut self) -> Result<Option<AudioFrame>, CodecError> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match super::ffw_decoder_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(AudioFrame::from_raw_ptr(fptr)))
                    }
                }
                0 => Ok(None),
                _ => Err(CodecError::new(ErrorKind::Error, "decoding error")),
            }
        }
    }
}

impl Drop for AudioDecoder {
    fn drop(&mut self) {
        unsafe { super::ffw_decoder_free(self.ptr) }
    }
}

unsafe impl Send for AudioDecoder {}
unsafe impl Sync for AudioDecoder {}

/// Builder for the audio encoder.
pub struct AudioEncoderBuilder {
    ptr: *mut c_void,

    sample_format: Option<SampleFormat>,
    sample_rate: Option<u32>,
    channel_layout: Option<ChannelLayout>,
}

impl AudioEncoderBuilder {
    /// Create a new encoder builder for a given codec.
    fn new(codec: &str) -> Result<AudioEncoderBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_encoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        unsafe {
            super::ffw_encoder_set_bit_rate(ptr, 0);
            super::ffw_encoder_set_time_base(ptr, 1, 1000);
        }

        let res = AudioEncoderBuilder {
            ptr: ptr,

            sample_format: None,
            sample_rate: None,
            channel_layout: None,
        };

        Ok(res)
    }

    /// Create a new encoder builder from given codec parameters.
    fn from_codec_parameters(
        codec_parameters: &AudioCodecParameters,
    ) -> Result<AudioEncoderBuilder, Error> {
        let ptr = unsafe { super::ffw_encoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create an encoder"));
        }

        let sample_format;
        let sample_rate;
        let channel_layout;

        unsafe {
            sample_format = super::ffw_encoder_get_sample_format(ptr) as _;
            sample_rate = super::ffw_encoder_get_sample_rate(ptr) as _;
            channel_layout = super::ffw_encoder_get_channel_layout(ptr) as _;
        }

        let res = AudioEncoderBuilder {
            ptr: ptr,

            sample_format: Some(sample_format),
            sample_rate: Some(sample_rate),
            channel_layout: Some(channel_layout),
        };

        Ok(res)
    }

    /// Set encoder bit rate. The default is 0 (i.e. automatic).
    pub fn bit_rate(self, bit_rate: u64) -> AudioEncoderBuilder {
        unsafe {
            super::ffw_encoder_set_bit_rate(self.ptr, bit_rate as _);
        }

        self
    }

    /// Set encoder time base as a rational number. The default is 1/1000.
    pub fn time_base(self, num: u32, den: u32) -> AudioEncoderBuilder {
        unsafe {
            super::ffw_encoder_set_time_base(self.ptr, num as _, den as _);
        }

        self
    }

    /// Set audio sample format.
    pub fn sample_format(mut self, format: SampleFormat) -> AudioEncoderBuilder {
        self.sample_format = Some(format);
        self
    }

    /// Set sampling rate.
    pub fn sample_rate(mut self, rate: u32) -> AudioEncoderBuilder {
        self.sample_rate = Some(rate);
        self
    }

    /// Set channel layout.
    pub fn channel_layout(mut self, layout: ChannelLayout) -> AudioEncoderBuilder {
        self.channel_layout = Some(layout);
        self
    }

    /// Build the encoder.
    pub fn build(mut self) -> Result<AudioEncoder, Error> {
        let sample_format = self
            .sample_format
            .ok_or(Error::new("sample format not set"))?;

        let sample_rate = self.sample_rate.ok_or(Error::new("sample rate not set"))?;

        let channel_layout = self
            .channel_layout
            .ok_or(Error::new("channel layout not set"))?;

        unsafe {
            super::ffw_encoder_set_sample_format(self.ptr, sample_format as _);
            super::ffw_encoder_set_sample_rate(self.ptr, sample_rate as _);
            super::ffw_encoder_set_channel_layout(self.ptr, channel_layout as _);

            if super::ffw_encoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the encoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = AudioEncoder { ptr: ptr };

        Ok(res)
    }
}

impl Drop for AudioEncoderBuilder {
    fn drop(&mut self) {
        unsafe { super::ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for AudioEncoderBuilder {}
unsafe impl Sync for AudioEncoderBuilder {}

/// Audio encoder.
///
/// # Encoder operation
/// 1. Push a frame to the encoder.
/// 2. Take all packets from the encoder until you get None.
/// 3. If there are more frames to be encoded, continue with 1.
/// 4. Flush the encoder.
/// 5. Take all packets from the encoder until you get None.
pub struct AudioEncoder {
    ptr: *mut c_void,
}

impl AudioEncoder {
    /// Create a new encoder builder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &AudioCodecParameters,
    ) -> Result<AudioEncoderBuilder, Error> {
        AudioEncoderBuilder::from_codec_parameters(codec_parameters)
    }

    /// Get encoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<AudioEncoderBuilder, Error> {
        AudioEncoderBuilder::new(codec)
    }

    /// Get codec parameters.
    pub fn codec_parameters(&self) -> AudioCodecParameters {
        let ptr = unsafe { super::ffw_encoder_get_codec_parameters(self.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        params.into_audio_codec_parameters().unwrap()
    }

    /// Number of samples per audio channel in an audio frame. Each encoded
    /// frame except the last one must contain exactly this number of samples.
    /// The method returns None if the number of samples per frame is not
    /// restricted.
    pub fn samples_per_frame(&self) -> Option<usize> {
        let res = unsafe { super::ffw_encoder_get_frame_size(self.ptr) as _ };

        if res == 0 {
            None
        } else {
            Some(res)
        }
    }

    /// Push a given frame to the encoder.
    pub fn push(&mut self, frame: &AudioFrame) -> Result<(), CodecError> {
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

impl Drop for AudioEncoder {
    fn drop(&mut self) {
        unsafe { super::ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for AudioEncoder {}
unsafe impl Sync for AudioEncoder {}
