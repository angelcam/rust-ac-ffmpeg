//! Audio resampler.

use std::{
    os::raw::{c_int, c_void},
    ptr,
};

use crate::{
    codec::{
        audio::{AudioFrame, ChannelLayout, SampleFormat},
        CodecError,
    },
    time::TimeBase,
    Error,
};

extern "C" {
    fn ffw_audio_resampler_new(
        target_channel_layout: *const c_void,
        target_sample_format: c_int,
        target_sample_rate: c_int,
        target_frame_samples: c_int,
        source_channel_layout: *const c_void,
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
    #[inline]
    fn new() -> Self {
        Self {
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
    #[inline]
    pub fn source_channel_layout(mut self, channel_layout: ChannelLayout) -> Self {
        self.source_channel_layout = Some(channel_layout);
        self
    }

    /// Set source sample format.
    #[inline]
    pub fn source_sample_format(mut self, sample_format: SampleFormat) -> Self {
        self.source_sample_format = Some(sample_format);
        self
    }

    /// Set source sample rate.
    #[inline]
    pub fn source_sample_rate(mut self, sample_rate: u32) -> Self {
        self.source_sample_rate = Some(sample_rate);
        self
    }

    /// Set target channel layout.
    #[inline]
    pub fn target_channel_layout(mut self, channel_layout: ChannelLayout) -> Self {
        self.target_channel_layout = Some(channel_layout);
        self
    }

    /// Set target sample format.
    #[inline]
    pub fn target_sample_format(mut self, sample_format: SampleFormat) -> Self {
        self.target_sample_format = Some(sample_format);
        self
    }

    /// Set target sample rate.
    #[inline]
    pub fn target_sample_rate(mut self, sample_rate: u32) -> Self {
        self.target_sample_rate = Some(sample_rate);
        self
    }

    /// Set the expected number of samples in target frames (for fixed frame
    /// size codecs).
    #[inline]
    pub fn target_frame_samples(mut self, samples: Option<usize>) -> Self {
        self.target_frame_samples = samples;
        self
    }

    /// Build the resampler.
    pub fn build(self) -> Result<AudioResampler, Error> {
        let source_channel_layout = self
            .source_channel_layout
            .ok_or_else(|| Error::new("source channel layout was not set"))?;
        let source_sample_format = self
            .source_sample_format
            .ok_or_else(|| Error::new("source sample format was not set"))?;
        let source_sample_rate = self
            .source_sample_rate
            .ok_or_else(|| Error::new("source sample rate was not set"))?;

        let target_channel_layout = self
            .target_channel_layout
            .ok_or_else(|| Error::new("target channel layout was not set"))?;
        let target_sample_format = self
            .target_sample_format
            .ok_or_else(|| Error::new("target sample format was not set"))?;
        let target_sample_rate = self
            .target_sample_rate
            .ok_or_else(|| Error::new("target sample rate was not set"))?;

        let target_frame_samples = self.target_frame_samples.unwrap_or(0);

        let ptr = unsafe {
            ffw_audio_resampler_new(
                target_channel_layout.as_ptr(),
                target_sample_format.into_raw(),
                target_sample_rate as _,
                target_frame_samples as _,
                source_channel_layout.as_ptr(),
                source_sample_format.into_raw(),
                source_sample_rate as _,
            )
        };

        if ptr.is_null() {
            return Err(Error::new(
                "unable to create an audio resampler for a given configuration",
            ));
        }

        let res = AudioResampler {
            ptr,

            source_channel_layout,
            source_sample_format,
            source_sample_rate,
            target_sample_rate,
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
/// Timestamps of the output frames will be in 1 / target_sample_rate time
/// base.
pub struct AudioResampler {
    ptr: *mut c_void,

    source_channel_layout: ChannelLayout,
    source_sample_format: SampleFormat,
    source_sample_rate: u32,
    target_sample_rate: u32,
}

impl AudioResampler {
    /// Get a builder for the audio resampler.
    #[inline]
    pub fn builder() -> AudioResamplerBuilder {
        AudioResamplerBuilder::new()
    }

    /// Push a given frame to the resampler.
    ///
    /// # Panics
    /// The method panics if the operation is not expected (i.e. another
    /// operation needs to be done).
    pub fn push(&mut self, frame: AudioFrame) -> Result<(), Error> {
        self.try_push(frame).map_err(|err| err.unwrap_inner())
    }

    /// Push a given frame to the resampler.
    pub fn try_push(&mut self, frame: AudioFrame) -> Result<(), CodecError> {
        if frame.channel_layout() != &self.source_channel_layout {
            return Err(CodecError::error(
                "invalid frame, channel layout does not match",
            ));
        }

        if frame.sample_format() != self.source_sample_format {
            return Err(CodecError::error(
                "invalid frame, sample format does not match",
            ));
        }

        if frame.sample_rate() != self.source_sample_rate {
            return Err(CodecError::error(
                "invalid frame, sample rate does not match",
            ));
        }

        let frame = frame.with_time_base(TimeBase::new(1, self.source_sample_rate));

        unsafe {
            match ffw_audio_resampler_push_frame(self.ptr, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all frames must be consumed before pushing a new frame",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    /// Flush the resampler.
    ///
    /// # Panics
    /// The method panics if the operation is not expected (i.e. another
    /// operation needs to be done).
    pub fn flush(&mut self) -> Result<(), Error> {
        self.try_flush().map_err(|err| err.unwrap_inner())
    }

    /// Flush the resampler.
    pub fn try_flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match ffw_audio_resampler_push_frame(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all frames must be consumed before flushing",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    /// Take a frame from the resampler (if available).
    pub fn take(&mut self) -> Result<Option<AudioFrame>, Error> {
        let mut fptr = ptr::null_mut();

        let tb = TimeBase::new(1, self.target_sample_rate);

        unsafe {
            match ffw_audio_resampler_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("unable to allocate an audio frame")
                    } else {
                        Ok(Some(AudioFrame::from_raw_ptr(fptr, tb)))
                    }
                }
                0 => Ok(None),
                e => Err(Error::from_raw_error_code(e)),
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
