use std::ptr;

use libc::{c_int, c_void};

use crate::Error;

use crate::codec::audio::{AudioFrame, ChannelLayout, SampleFormat};
use crate::codec::{CodecError, ErrorKind};

extern "C" {
    fn ffw_audio_resampler_new(
        target_channel_layout: u64,
        target_sample_format: c_int,
        target_sample_rate: c_int,
        target_frame_samples: c_int,
        source_channel_layout: u64,
        source_sample_format: c_int,
        source_sample_rate: c_int,
    ) -> *mut c_void;
    fn ffw_audio_resampler_free(resampler: *mut c_void);
    fn ffw_audio_resampler_push_frame(resampler: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_audio_resampler_take_frame(resampler: *mut c_void, frame: *mut *mut c_void) -> c_int;
}

/// Builder for the audio resampler.
pub struct AudioResamplerBuilder {
    source_channel_layout: Option<ChannelLayout>,
    source_sample_format: Option<SampleFormat>,
    source_sample_rate: Option<u32>,

    target_channel_layout: Option<ChannelLayout>,
    target_sample_format: Option<SampleFormat>,
    target_sample_rate: Option<u32>,

    target_frame_samples: Option<usize>,
}

impl AudioResamplerBuilder {
    /// Create a new builder.
    fn new() -> AudioResamplerBuilder {
        AudioResamplerBuilder {
            source_channel_layout: None,
            source_sample_format: None,
            source_sample_rate: None,

            target_channel_layout: None,
            target_sample_format: None,
            target_sample_rate: None,

            target_frame_samples: None,
        }
    }

    /// Set source channel layout.
    pub fn source_channel_layout(mut self, channel_layout: ChannelLayout) -> AudioResamplerBuilder {
        self.source_channel_layout = Some(channel_layout);
        self
    }

    /// Set source sample format.
    pub fn source_sample_format(mut self, sample_format: SampleFormat) -> AudioResamplerBuilder {
        self.source_sample_format = Some(sample_format);
        self
    }

    /// Set source sample rate.
    pub fn source_sample_rate(mut self, sample_rate: u32) -> AudioResamplerBuilder {
        self.source_sample_rate = Some(sample_rate);
        self
    }

    /// Set target channel layout.
    pub fn target_channel_layout(mut self, channel_layout: ChannelLayout) -> AudioResamplerBuilder {
        self.target_channel_layout = Some(channel_layout);
        self
    }

    /// Set target sample format.
    pub fn target_sample_format(mut self, sample_format: SampleFormat) -> AudioResamplerBuilder {
        self.target_sample_format = Some(sample_format);
        self
    }

    /// Set target sample rate.
    pub fn target_sample_rate(mut self, sample_rate: u32) -> AudioResamplerBuilder {
        self.target_sample_rate = Some(sample_rate);
        self
    }

    /// Set the expected number of samples in target frames (for fixed frame
    /// size codecs).
    pub fn target_frame_samples(mut self, samples: Option<usize>) -> AudioResamplerBuilder {
        self.target_frame_samples = samples;
        self
    }

    /// Build the resampler.
    pub fn build(self) -> Result<AudioResampler, Error> {
        let source_channel_layout = self
            .source_channel_layout
            .ok_or(Error::new("source channel layout was not set"))?;
        let source_sample_format = self
            .source_sample_format
            .ok_or(Error::new("source sample format was not set"))?;
        let source_sample_rate = self
            .source_sample_rate
            .ok_or(Error::new("source sample rate was not set"))?;

        let target_channel_layout = self
            .target_channel_layout
            .ok_or(Error::new("target channel layout was not set"))?;
        let target_sample_format = self
            .target_sample_format
            .ok_or(Error::new("target sample format was not set"))?;
        let target_sample_rate = self
            .target_sample_rate
            .ok_or(Error::new("target sample rate was not set"))?;

        let target_frame_samples = self.target_frame_samples.unwrap_or(0);

        let ptr = unsafe {
            ffw_audio_resampler_new(
                target_channel_layout as _,
                target_sample_format as _,
                target_sample_rate as _,
                target_frame_samples as _,
                source_channel_layout as _,
                source_sample_format as _,
                source_sample_rate as _,
            )
        };

        if ptr.is_null() {
            return Err(Error::new(
                "unable to create an audio resampler for a given configuration",
            ));
        }

        let res = AudioResampler {
            ptr: ptr,

            source_channel_layout: source_channel_layout,
            source_sample_format: source_sample_format,
            source_sample_rate: source_sample_rate,
        };

        Ok(res)
    }
}

/// Audio resampler.
///
///  # Resampler operation
/// 1. Push an audio frame to the resampler.
/// 2. Take all frames from the resampler until you get None.
/// 3. If there are more frames to be resampled, continue with 1.
/// 4. Flush the resampler.
/// 5. Take all frames from the resampler until you get None.
///
/// Timestamps of input frames are expected to be in 1 / source_sample_rate
/// timebase. Timestamps of output frames will be in 1 / target_sample_rate
/// timebase.
pub struct AudioResampler {
    ptr: *mut c_void,

    source_channel_layout: ChannelLayout,
    source_sample_format: SampleFormat,
    source_sample_rate: u32,
}

impl AudioResampler {
    /// Get a builder for the audio resampler.
    pub fn builder() -> AudioResamplerBuilder {
        AudioResamplerBuilder::new()
    }

    /// Push a given frame to the resampler.
    pub fn push(&mut self, frame: &AudioFrame) -> Result<(), CodecError> {
        if frame.channel_layout() != self.source_channel_layout {
            return Err(CodecError::new(
                ErrorKind::Error,
                "invalid frame, channel layout does not match",
            ));
        }

        if frame.sample_format() != self.source_sample_format {
            return Err(CodecError::new(
                ErrorKind::Error,
                "invalid frame, sample format does not match",
            ));
        }

        if frame.sample_rate() != self.source_sample_rate {
            return Err(CodecError::new(
                ErrorKind::Error,
                "invalid frame, sample rate does not match",
            ));
        }

        unsafe {
            match ffw_audio_resampler_push_frame(self.ptr, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::new(
                    ErrorKind::Again,
                    "all frames must be consumed before pushing a new frame",
                )),
                _ => Err(CodecError::new(ErrorKind::Error, "audio resampler error")),
            }
        }
    }

    /// Flush the resampler.
    pub fn flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match ffw_audio_resampler_push_frame(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::new(
                    ErrorKind::Again,
                    "all frames must be consumed before flushing",
                )),
                _ => Err(CodecError::new(ErrorKind::Error, "audio resampler error")),
            }
        }
    }

    /// Take a frame from the resampler (if available).
    pub fn take(&mut self) -> Result<Option<AudioFrame>, CodecError> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match ffw_audio_resampler_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("unable to allocate an audio frame")
                    } else {
                        Ok(Some(AudioFrame::from_raw_ptr(fptr)))
                    }
                }
                0 => Ok(None),
                _ => Err(CodecError::new(ErrorKind::Error, "audio resampler error")),
            }
        }
    }
}

impl Drop for AudioResampler {
    fn drop(&mut self) {
        unsafe { ffw_audio_resampler_free(self.ptr) }
    }
}

unsafe impl Send for AudioResampler {}
unsafe impl Sync for AudioResampler {}
