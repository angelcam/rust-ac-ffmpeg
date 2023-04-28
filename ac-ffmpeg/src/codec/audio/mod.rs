//! Audio decoder/encoder.

pub mod frame;
pub mod resampler;
pub mod transcoder;

use std::{ffi::CString, os::raw::c_void, ptr};

use crate::{
    codec::{
        AudioCodecParameters, CodecError, CodecFlag, CodecFlag2, CodecParameters, CodecTag,
        Decoder, Encoder,
    },
    format::stream::Stream,
    packet::Packet,
    time::TimeBase,
    Error,
};

pub use self::{
    frame::{AudioFrame, AudioFrameMut, ChannelLayout, ChannelLayoutRef, SampleFormat},
    resampler::AudioResampler,
    transcoder::AudioTranscoder,
};

/// Builder for the audio decoder.
pub struct AudioDecoderBuilder {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl AudioDecoderBuilder {
    /// Create a new decoder builder from a given raw representation.
    unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        let time_base = TimeBase::MICROSECONDS;

        super::ffw_decoder_set_pkt_timebase(ptr, time_base.num() as _, time_base.den() as _);

        Self { ptr, time_base }
    }

    /// Create a new builder for a given codec.
    fn new(codec: &str) -> Result<Self, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { super::ffw_decoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        unsafe { Ok(Self::from_raw_ptr(ptr)) }
    }

    /// Create a new builder from given codec parameters.
    fn from_codec_parameters(codec_parameters: &AudioCodecParameters) -> Result<Self, Error> {
        let ptr = unsafe { super::ffw_decoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create a decoder"));
        }

        unsafe { Ok(Self::from_raw_ptr(ptr)) }
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

        unsafe {
            super::ffw_decoder_set_pkt_timebase(
                self.ptr,
                time_base.num() as _,
                time_base.den() as _,
            );
        }

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
    pub fn build(mut self) -> Result<AudioDecoder, Error> {
        unsafe {
            if super::ffw_decoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the decoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = AudioDecoder {
            ptr,
            time_base: self.time_base,
        };

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
pub struct AudioDecoder {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl AudioDecoder {
    /// Create a new audio decoder for a given codec.
    pub fn new(codec: &str) -> Result<Self, Error> {
        AudioDecoderBuilder::new(codec).and_then(|builder| builder.build())
    }

    /// Create a new decoder from given codec parameters.
    pub fn from_codec_parameters(
        codec_parameters: &AudioCodecParameters,
    ) -> Result<AudioDecoderBuilder, Error> {
        AudioDecoderBuilder::from_codec_parameters(codec_parameters)
    }

    /// Create a new decoder for a given stream.
    ///
    /// # Panics
    /// The method panics if the stream is not an audio stream.
    pub fn from_stream(stream: &Stream) -> Result<AudioDecoderBuilder, Error> {
        let codec_parameters = stream
            .codec_parameters()
            .into_audio_codec_parameters()
            .unwrap();

        let builder = AudioDecoderBuilder::from_codec_parameters(&codec_parameters)?
            .time_base(stream.time_base());

        Ok(builder)
    }

    /// Get decoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<AudioDecoderBuilder, Error> {
        AudioDecoderBuilder::new(codec)
    }
}

impl Decoder for AudioDecoder {
    type CodecParameters = AudioCodecParameters;
    type Frame = AudioFrame;

    fn codec_parameters(&self) -> Self::CodecParameters {
        let ptr = unsafe { super::ffw_decoder_get_codec_parameters(self.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        params.into_audio_codec_parameters().unwrap()
    }

    fn try_push(&mut self, packet: Packet) -> Result<(), CodecError> {
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

    fn try_flush(&mut self) -> Result<(), CodecError> {
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

    fn take(&mut self) -> Result<Option<AudioFrame>, Error> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match super::ffw_decoder_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(AudioFrame::from_raw_ptr(fptr, self.time_base)))
                    }
                }
                0 => Ok(None),
                e => Err(Error::from_raw_error_code(e)),
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

/// Wrapper for the raw audio encoder.
struct RawAudioEncoder {
    ptr: *mut c_void,
}

impl RawAudioEncoder {
    /// Create a new encoder wrapper.
    fn from_raw_ptr(ptr: *mut c_void) -> Self {
        Self { ptr }
    }
}

impl Drop for RawAudioEncoder {
    fn drop(&mut self) {
        unsafe { super::ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for RawAudioEncoder {}
unsafe impl Sync for RawAudioEncoder {}

/// Builder for the audio encoder.
pub struct AudioEncoderBuilder {
    raw: RawAudioEncoder,

    time_base: TimeBase,

    sample_format: Option<SampleFormat>,
    sample_rate: Option<u32>,
    channel_layout: Option<ChannelLayout>,
}

impl AudioEncoderBuilder {
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
            raw: RawAudioEncoder::from_raw_ptr(ptr),

            time_base: TimeBase::MICROSECONDS,

            sample_format: None,
            sample_rate: None,
            channel_layout: None,
        };

        Ok(res)
    }

    /// Create a new encoder builder from given codec parameters.
    fn from_codec_parameters(codec_parameters: &AudioCodecParameters) -> Result<Self, Error> {
        let ptr = unsafe { super::ffw_encoder_from_codec_parameters(codec_parameters.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::new("unable to create an encoder"));
        }

        let sample_format;
        let sample_rate;
        let channel_layout;

        unsafe {
            sample_format = SampleFormat::from_raw(super::ffw_encoder_get_sample_format(ptr));
            sample_rate = super::ffw_encoder_get_sample_rate(ptr) as _;
            channel_layout =
                ChannelLayoutRef::from_raw_ptr(super::ffw_encoder_get_channel_layout(ptr));
        }

        let channel_layout = channel_layout.to_owned();

        let res = Self {
            raw: RawAudioEncoder::from_raw_ptr(ptr),

            time_base: TimeBase::MICROSECONDS,

            sample_format: Some(sample_format),
            sample_rate: Some(sample_rate),
            channel_layout: Some(channel_layout),
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
            super::ffw_encoder_set_initial_option(
                self.raw.ptr,
                name.as_ptr() as _,
                value.as_ptr() as _,
            )
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Set encoder bit rate. The default is 0 (i.e. automatic).
    pub fn bit_rate(self, bit_rate: u64) -> Self {
        unsafe {
            super::ffw_encoder_set_bit_rate(self.raw.ptr, bit_rate as _);
        }

        self
    }

    /// Set encoder time base. The default time base is in microseconds.
    pub fn time_base(mut self, time_base: TimeBase) -> Self {
        self.time_base = time_base;
        self
    }

    /// Set audio sample format.
    pub fn sample_format(mut self, format: SampleFormat) -> Self {
        self.sample_format = Some(format);
        self
    }

    /// Set sampling rate.
    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.sample_rate = Some(rate);
        self
    }

    /// Set channel layout.
    pub fn channel_layout(mut self, layout: ChannelLayout) -> Self {
        self.channel_layout = Some(layout);
        self
    }

    /// Set codec tag.
    pub fn codec_tag(self, codec_tag: impl Into<CodecTag>) -> Self {
        unsafe {
            super::ffw_encoder_set_codec_tag(self.raw.ptr, codec_tag.into().into());
        }

        self
    }

    /// Set a codec flag.
    pub fn set_flag(self, flag: CodecFlag) -> Self {
        unsafe {
            super::ffw_encoder_set_flag(self.raw.ptr, flag.into_raw());
        }

        self
    }

    /// Set a codec flag using the second set of flags.
    pub fn set_flag2(self, flag2: CodecFlag2) -> Self {
        unsafe {
            super::ffw_encoder_set_flag2(self.raw.ptr, flag2.into_raw());
        }

        self
    }

    /// Build the encoder.
    pub fn build(self) -> Result<AudioEncoder, Error> {
        let sample_format = self
            .sample_format
            .ok_or_else(|| Error::new("sample format not set"))?;

        let sample_rate = self
            .sample_rate
            .ok_or_else(|| Error::new("sample rate not set"))?;

        let channel_layout = self
            .channel_layout
            .ok_or_else(|| Error::new("channel layout not set"))?;

        let tb = self.time_base;

        unsafe {
            super::ffw_encoder_set_time_base(self.raw.ptr, tb.num() as _, tb.den() as _);
            super::ffw_encoder_set_sample_format(self.raw.ptr, sample_format.into_raw());
            super::ffw_encoder_set_sample_rate(self.raw.ptr, sample_rate as _);

            if super::ffw_encoder_set_channel_layout(self.raw.ptr, channel_layout.as_ptr()) != 0 {
                panic!("unable to copy channel layout");
            }

            if super::ffw_encoder_open(self.raw.ptr) != 0 {
                return Err(Error::new("unable to build the encoder"));
            }
        }

        let res = AudioEncoder {
            raw: self.raw,
            time_base: tb,
        };

        Ok(res)
    }
}

/// Audio encoder.
pub struct AudioEncoder {
    raw: RawAudioEncoder,
    time_base: TimeBase,
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

    /// Number of samples per audio channel in an audio frame. Each encoded
    /// frame except the last one must contain exactly this number of samples.
    /// The method returns None if the number of samples per frame is not
    /// restricted.
    pub fn samples_per_frame(&self) -> Option<usize> {
        let res = unsafe { super::ffw_encoder_get_frame_size(self.raw.ptr) as _ };

        if res == 0 {
            None
        } else {
            Some(res)
        }
    }
}

impl Encoder for AudioEncoder {
    type CodecParameters = AudioCodecParameters;
    type Frame = AudioFrame;

    fn codec_parameters(&self) -> Self::CodecParameters {
        let ptr = unsafe { super::ffw_encoder_get_codec_parameters(self.raw.ptr) };

        if ptr.is_null() {
            panic!("unable to allocate codec parameters");
        }

        let params = unsafe { CodecParameters::from_raw_ptr(ptr) };

        params.into_audio_codec_parameters().unwrap()
    }

    fn try_push(&mut self, frame: AudioFrame) -> Result<(), CodecError> {
        let frame = frame.with_time_base(self.time_base);

        unsafe {
            match super::ffw_encoder_push_frame(self.raw.ptr, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all packets must be consumed before pushing a new frame",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn try_flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match super::ffw_encoder_push_frame(self.raw.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all packets must be consumed before flushing",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn take(&mut self) -> Result<Option<Packet>, Error> {
        let mut pptr = ptr::null_mut();

        unsafe {
            match super::ffw_encoder_take_packet(self.raw.ptr, &mut pptr) {
                1 => {
                    if pptr.is_null() {
                        panic!("no packet received")
                    } else {
                        Ok(Some(Packet::from_raw_ptr(pptr, self.time_base)))
                    }
                }
                0 => Ok(None),
                e => Err(Error::from_raw_error_code(e)),
            }
        }
    }
}
