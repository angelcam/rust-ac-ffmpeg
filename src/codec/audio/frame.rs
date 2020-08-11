//! Audio frame.

use std::{
    ffi::{CStr, CString},
    fmt::{self, Display, Formatter},
    ptr,
    str::FromStr,
};

use libc::{c_char, c_int, c_void};

use crate::time::{TimeBase, Timestamp};

extern "C" {
    fn ffw_get_channel_layout_by_name(name: *const c_char) -> u64;
    fn ffw_get_channel_layout_channels(layout: u64) -> c_int;
    fn ffw_get_default_channel_layout(channels: c_int) -> u64;

    fn ffw_get_sample_format_by_name(name: *const c_char) -> c_int;
    fn ffw_get_sample_format_name(format: c_int) -> *const c_char;
    fn ffw_sample_format_is_none(format: c_int) -> c_int;

    fn ffw_frame_new_silence(
        channel_layout: u64,
        sample_fmt: c_int,
        sample_rate: c_int,
        nb_samples: c_int,
    ) -> *mut c_void;
    fn ffw_frame_get_format(frame: *const c_void) -> c_int;
    fn ffw_frame_get_nb_samples(frame: *const c_void) -> c_int;
    fn ffw_frame_get_sample_rate(frame: *const c_void) -> c_int;
    fn ffw_frame_get_channels(frame: *const c_void) -> c_int;
    fn ffw_frame_get_channel_layout(frame: *const c_void) -> u64;
    fn ffw_frame_get_pts(frame: *const c_void) -> i64;
    fn ffw_frame_set_pts(frame: *mut c_void, pts: i64);
    fn ffw_frame_clone(frame: *const c_void) -> *mut c_void;
    fn ffw_frame_free(frame: *mut c_void);
}

/// An error indicating an unknown channel layout.
#[derive(Debug, Copy, Clone)]
pub struct UnknownChannelLayout;

impl Display for UnknownChannelLayout {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str("unknown channel layout")
    }
}

impl std::error::Error for UnknownChannelLayout {}

/// Channel layout.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ChannelLayout(u64);

impl ChannelLayout {
    /// Create channel layout from its raw representation.
    pub(crate) fn from_raw(v: u64) -> Self {
        Self(v)
    }

    /// Get the raw representation.
    pub(crate) fn into_raw(self) -> u64 {
        let Self(layout) = self;

        layout
    }

    /// Get default channel layout for a given number of channels.
    pub fn from_channels(channels: u32) -> Option<Self> {
        let layout = unsafe { ffw_get_default_channel_layout(channels as _) };

        if layout == 0 {
            None
        } else {
            Some(Self(layout))
        }
    }

    /// Get number of channels.
    pub fn channels(self) -> u32 {
        unsafe { ffw_get_channel_layout_channels(self.into_raw()) as _ }
    }
}

impl FromStr for ChannelLayout {
    type Err = UnknownChannelLayout;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = CString::new(s).expect("invalid channel layout name");

        let layout = unsafe { ffw_get_channel_layout_by_name(name.as_ptr() as _) };

        if layout == 0 {
            Err(UnknownChannelLayout)
        } else {
            Ok(Self(layout))
        }
    }
}

/// Get channel layout with a given name.
pub fn get_channel_layout(name: &str) -> ChannelLayout {
    ChannelLayout::from_str(name).unwrap()
}

/// An error indicating an unknown sample format.
#[derive(Debug, Copy, Clone)]
pub struct UnknownSampleFormat;

/// Audio sample format.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct SampleFormat(c_int);

impl SampleFormat {
    /// Create a sample format value from a given raw representation.
    pub(crate) fn from_raw(v: c_int) -> Self {
        Self(v)
    }

    /// Get the raw value.
    pub(crate) fn into_raw(self) -> c_int {
        let Self(format) = self;

        format
    }

    /// Get name of the sample format.
    pub fn name(self) -> &'static str {
        unsafe {
            let ptr = ffw_get_sample_format_name(self.into_raw());

            if ptr.is_null() {
                panic!("invalid sample format");
            }

            let name = CStr::from_ptr(ptr as _);

            name.to_str().unwrap()
        }
    }
}

impl FromStr for SampleFormat {
    type Err = UnknownSampleFormat;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = CString::new(s).expect("invalid sample format name");

        unsafe {
            let format = ffw_get_sample_format_by_name(name.as_ptr() as _);

            if ffw_sample_format_is_none(format) == 0 {
                Ok(Self(format))
            } else {
                Err(UnknownSampleFormat)
            }
        }
    }
}

/// Get audio sample format with a given name.
pub fn get_sample_format(name: &str) -> SampleFormat {
    SampleFormat::from_str(name).unwrap()
}

/// An audio frame with mutable data.
pub struct AudioFrameMut {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl AudioFrameMut {
    /// Create an audio frame containing silence. The time base of the frame
    /// will be in microseconds.
    pub fn silence(
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        sample_rate: u32,
        samples: usize,
    ) -> Self {
        let ptr = unsafe {
            ffw_frame_new_silence(
                channel_layout.into_raw(),
                sample_format.into_raw(),
                sample_rate as _,
                samples as _,
            )
        };

        if ptr.is_null() {
            panic!("unable to allocate an audio frame");
        }

        Self {
            ptr,
            time_base: TimeBase::MICROSECONDS,
        }
    }

    /// Get frame sample format.
    pub fn sample_format(&self) -> SampleFormat {
        unsafe { SampleFormat::from_raw(ffw_frame_get_format(self.ptr)) }
    }

    /// Get frame sample rate.
    pub fn sample_rate(&self) -> u32 {
        unsafe { ffw_frame_get_sample_rate(self.ptr) as _ }
    }

    /// Get number of samples (per channel) in this frame.
    pub fn samples(&self) -> usize {
        unsafe { ffw_frame_get_nb_samples(self.ptr) as _ }
    }

    /// Get number of channels.
    pub fn channels(&self) -> u32 {
        unsafe { ffw_frame_get_channels(self.ptr) as _ }
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> ChannelLayout {
        unsafe { ChannelLayout::from_raw(ffw_frame_get_channel_layout(self.ptr)) }
    }

    /// Get frame time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Set frame time base. (This will rescale the current timestamp into a
    /// given time base.)
    pub fn with_time_base(mut self, time_base: TimeBase) -> Self {
        let new_pts = self.pts().with_time_base(time_base);

        unsafe {
            ffw_frame_set_pts(self.ptr, new_pts.timestamp());
        }

        self.time_base = time_base;

        self
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> Timestamp {
        let pts = unsafe { ffw_frame_get_pts(self.ptr) };

        Timestamp::new(pts, self.time_base)
    }

    /// Set presentation timestamp.
    pub fn with_pts(self, pts: Timestamp) -> Self {
        let pts = pts.with_time_base(self.time_base);

        unsafe { ffw_frame_set_pts(self.ptr, pts.timestamp()) }

        self
    }

    /// Make the frame immutable.
    pub fn freeze(mut self) -> AudioFrame {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        AudioFrame {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Drop for AudioFrameMut {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for AudioFrameMut {}
unsafe impl Sync for AudioFrameMut {}

/// An audio frame with immutable data.
pub struct AudioFrame {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl AudioFrame {
    /// Create a new audio frame from its raw representation.
    pub(crate) unsafe fn from_raw_ptr(ptr: *mut c_void, time_base: TimeBase) -> Self {
        AudioFrame { ptr, time_base }
    }

    /// Get frame sample format.
    pub fn sample_format(&self) -> SampleFormat {
        unsafe { SampleFormat::from_raw(ffw_frame_get_format(self.ptr)) }
    }

    /// Get frame sample rate.
    pub fn sample_rate(&self) -> u32 {
        unsafe { ffw_frame_get_sample_rate(self.ptr) as _ }
    }

    /// Get number of samples (per channel) in this frame.
    pub fn samples(&self) -> usize {
        unsafe { ffw_frame_get_nb_samples(self.ptr) as _ }
    }

    /// Get number of channels.
    pub fn channels(&self) -> u32 {
        unsafe { ffw_frame_get_channels(self.ptr) as _ }
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> ChannelLayout {
        unsafe { ChannelLayout::from_raw(ffw_frame_get_channel_layout(self.ptr)) }
    }

    /// Get frame time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Set frame time base. (This will rescale the current timestamp into a
    /// given time base.)
    pub fn with_time_base(mut self, time_base: TimeBase) -> Self {
        let new_pts = self.pts().with_time_base(time_base);

        unsafe {
            ffw_frame_set_pts(self.ptr, new_pts.timestamp());
        }

        self.time_base = time_base;

        self
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> Timestamp {
        let pts = unsafe { ffw_frame_get_pts(self.ptr) };

        Timestamp::new(pts, self.time_base)
    }

    /// Set presentation timestamp.
    pub fn with_pts(self, pts: Timestamp) -> Self {
        let pts = pts.with_time_base(self.time_base);

        unsafe { ffw_frame_set_pts(self.ptr, pts.timestamp()) }

        self
    }

    /// Get raw pointer.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.ptr
    }
}

impl Clone for AudioFrame {
    fn clone(&self) -> Self {
        let ptr = unsafe { ffw_frame_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone a frame");
        }

        Self {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Drop for AudioFrame {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for AudioFrame {}
unsafe impl Sync for AudioFrame {}
