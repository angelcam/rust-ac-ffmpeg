//! Audio frame.

#[cfg(not(channel_layout_v2))]
mod channels_v1;

#[cfg(channel_layout_v2)]
mod channels_v2;

use std::{
    ffi::{CStr, CString},
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    os::raw::{c_char, c_int, c_void},
    ptr, slice,
    str::FromStr,
};

use crate::time::{TimeBase, Timestamp};

#[cfg(not(channel_layout_v2))]
pub use channels_v1::{ChannelLayout, ChannelLayoutRef};

#[cfg(channel_layout_v2)]
pub use channels_v2::{ChannelLayout, ChannelLayoutRef};

extern "C" {
    fn ffw_get_sample_format_by_name(name: *const c_char) -> c_int;
    fn ffw_get_sample_format_name(format: c_int) -> *const c_char;
    fn ffw_sample_format_is_planar(format: c_int) -> c_int;
    fn ffw_sample_format_is_none(format: c_int) -> c_int;

    fn ffw_frame_new_silence(
        channel_layout: *const c_void,
        sample_fmt: c_int,
        sample_rate: c_int,
        nb_samples: c_int,
    ) -> *mut c_void;
    fn ffw_frame_get_format(frame: *const c_void) -> c_int;
    fn ffw_frame_get_nb_samples(frame: *const c_void) -> c_int;
    fn ffw_frame_get_sample_rate(frame: *const c_void) -> c_int;
    fn ffw_frame_get_channel_layout(frame: *const c_void) -> *const c_void;
    fn ffw_frame_get_pts(frame: *const c_void) -> i64;
    fn ffw_frame_set_pts(frame: *mut c_void, pts: i64);
    fn ffw_frame_get_plane_data(frame: *mut c_void, index: usize) -> *mut u8;
    fn ffw_frame_get_line_size(frame: *const c_void, plane: usize) -> usize;
    fn ffw_frame_clone(frame: *const c_void) -> *mut c_void;
    fn ffw_frame_free(frame: *mut c_void);
    fn ffw_frame_is_writable(frame: *const c_void) -> c_int;
    fn ffw_frame_make_writable(frame: *mut c_void) -> c_int;
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

    /// Check if the sample format is planar (i.e. each channel has its own
    /// plane).
    pub fn is_planar(self) -> bool {
        unsafe { ffw_sample_format_is_planar(self.into_raw()) != 0 }
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

/// Audio plane. This is an array of audio sample data. Depending on the
/// sample format, this can either be samples for a single channel, or
/// for all channels multiplexed together.
pub struct Plane<'a> {
    frame: *mut c_void,
    index: usize,
    line_size: usize,
    phantom: PhantomData<&'a ()>,
}

impl Plane<'_> {
    /// Create a new plane.
    fn new(frame: *mut c_void, index: usize, line_size: usize) -> Self {
        Self {
            frame,
            index,
            line_size,
            phantom: PhantomData,
        }
    }

    /// Get plane data.
    pub fn data(&self) -> &[u8] {
        unsafe {
            let data = ffw_frame_get_plane_data(self.frame, self.index as _);
            slice::from_raw_parts(data, self.line_size)
        }
    }

    /// Get mutable plane data.
    pub fn data_mut(&mut self) -> &mut [u8] {
        unsafe {
            let data = ffw_frame_get_plane_data(self.frame, self.index as _);
            slice::from_raw_parts_mut(data, self.line_size)
        }
    }
}

/// Get sample data planes from a given audio frame.
fn get_audio_planes<'a>(
    frame: *mut c_void,
    sample_format: SampleFormat,
    channels: usize,
) -> Vec<Plane<'a>> {
    let line_size = unsafe { ffw_frame_get_line_size(frame, 0) as _ };

    let mut inner = Vec::new();

    if sample_format.is_planar() {
        for i in 0..channels {
            inner.push(Plane::new(frame, i, line_size));
        }
    } else {
        inner.push(Plane::new(frame, 0, line_size));
    }

    inner
}

/// A collection of audio planes. This type can be dereferenced into a slice of
///  `Plane`. If the sample data is planar, you will get the same number of
/// `Plane`'s as you have channels. If the sample data is packed (or interleaved),
/// there will be a single plane containing data for all channels.
pub struct Planes<'a> {
    inner: Vec<Plane<'a>>,
}

impl<'a> From<&'a AudioFrame> for Planes<'a> {
    fn from(frame: &'a AudioFrame) -> Self {
        let sample_format = frame.sample_format();
        let channel_layout = frame.channel_layout();

        Self {
            inner: get_audio_planes(frame.ptr, sample_format, channel_layout.channels() as _),
        }
    }
}

impl<'a> From<&'a AudioFrameMut> for Planes<'a> {
    fn from(frame: &'a AudioFrameMut) -> Self {
        let sample_format = frame.sample_format();
        let channel_layout = frame.channel_layout();

        Self {
            inner: get_audio_planes(frame.ptr, sample_format, channel_layout.channels() as _),
        }
    }
}

impl<'a> Deref for Planes<'a> {
    type Target = [Plane<'a>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A collection of mutable audio planes.
pub struct PlanesMut<'a> {
    inner: Vec<Plane<'a>>,
}

impl<'a> From<&'a mut AudioFrameMut> for PlanesMut<'a> {
    fn from(frame: &'a mut AudioFrameMut) -> Self {
        let sample_format = frame.sample_format();
        let channel_layout = frame.channel_layout();

        Self {
            inner: get_audio_planes(frame.ptr, sample_format, channel_layout.channels() as _),
        }
    }
}

impl<'a> Deref for PlanesMut<'a> {
    type Target = [Plane<'a>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for PlanesMut<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
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
        channel_layout: &ChannelLayoutRef,
        sample_format: SampleFormat,
        sample_rate: u32,
        samples: usize,
    ) -> Self {
        let ptr = unsafe {
            ffw_frame_new_silence(
                channel_layout.as_ptr(),
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

    /// Get sample data planes for this frame.
    #[inline]
    pub fn planes(&self) -> Planes {
        Planes::from(self)
    }

    /// Get mutable sample data planes for this frame.
    #[inline]
    pub fn planes_mut(&mut self) -> PlanesMut {
        PlanesMut::from(self)
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> &ChannelLayoutRef {
        unsafe { ChannelLayoutRef::from_raw_ptr(ffw_frame_get_channel_layout(self.ptr)) }
    }

    /// Get frame time base.
    #[inline]
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
    #[inline]
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

    /// Get sample data planes for this frame.
    #[inline]
    pub fn planes(&self) -> Planes {
        Planes::from(self)
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> &ChannelLayoutRef {
        unsafe { ChannelLayoutRef::from_raw_ptr(ffw_frame_get_channel_layout(self.ptr)) }
    }

    /// Get frame time base.
    #[inline]
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

    /// Try to make this frame mutable. Returns AudioFrameMut if it can be made
    /// into mutable without copying the data, otherwise returns AudioFrame.
    pub fn try_into_mut(self) -> Result<AudioFrameMut, AudioFrame> {
        let res = unsafe { ffw_frame_is_writable(self.ptr) };
        if res > 0 {
            Ok(self.into_mut())
        } else {
            Err(self)
        }
    }

    /// Make this frame mutable. This will copy the data if it is not already
    /// mutable.
    pub fn into_mut(mut self) -> AudioFrameMut {
        let res = unsafe { ffw_frame_make_writable(self.ptr) };

        if res < 0 {
            panic!("unable to make the frame mutable");
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        AudioFrameMut {
            ptr,
            time_base: self.time_base,
        }
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
