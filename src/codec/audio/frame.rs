use std::ptr;

use std::ffi::CString;

use libc::{c_char, c_int, c_void, int64_t, uint64_t};

extern "C" {
    fn ffw_get_channel_layout_by_name(name: *const c_char) -> uint64_t;
    fn ffw_get_default_channel_layour(channels: c_int) -> uint64_t;

    fn ffw_get_sample_format_by_name(name: *const c_char) -> c_int;
    fn ffw_sample_format_is_none(format: c_int) -> c_int;

    fn ffw_frame_new_silence(
        channel_layout: uint64_t,
        sample_fmt: c_int,
        nb_samples: c_int,
    ) -> *mut c_void;
    fn ffw_frame_format(frame: *const c_void) -> c_int;
    fn ffw_frame_nb_samples(frame: *const c_void) -> c_int;
    fn ffw_frame_channels(frame: *const c_void) -> c_int;
    fn ffw_frame_channel_layout(frame: *const c_void) -> uint64_t;
    fn ffw_frame_pts(frame: *const c_void) -> int64_t;
    fn ffw_frame_clone(frame: *const c_void) -> *mut c_void;
    fn ffw_frame_free(frame: *mut c_void);
}

/// Channel layout;
pub type ChannelLayout = uint64_t;

/// Get channel layout with a given name.
pub fn get_channel_layout(name: &str) -> Option<ChannelLayout> {
    let name = CString::new(name).expect("invalid channel layout name");

    let layout = unsafe { ffw_get_channel_layout_by_name(name.as_ptr() as _) };

    if layout == 0 {
        None
    } else {
        Some(layout)
    }
}

/// Get default channel layout for a given number of channels.
pub fn get_default_channel_layout(channels: u32) -> Option<ChannelLayout> {
    let layout = unsafe { ffw_get_default_channel_layour(channels as _) };

    if layout == 0 {
        None
    } else {
        Some(layout)
    }
}

/// Audio sample format.
pub type SampleFormat = c_int;

/// Get audio sample format with a given name.
pub fn get_sample_format(name: &str) -> Option<SampleFormat> {
    let name = CString::new(name).expect("invalid sample format name");

    unsafe {
        let format = ffw_get_sample_format_by_name(name.as_ptr() as _);

        if ffw_sample_format_is_none(format) == 0 {
            Some(format)
        } else {
            None
        }
    }
}

/// Mutable audio frame.
pub struct AudioFrameMut {
    ptr: *mut c_void,
}

impl AudioFrameMut {
    /// Create an audio frame containing silence.
    pub fn silence(
        channel_layout: ChannelLayout,
        sample_format: SampleFormat,
        samples: usize,
    ) -> AudioFrameMut {
        let ptr = unsafe { ffw_frame_new_silence(channel_layout, sample_format, samples as _) };

        if ptr.is_null() {
            panic!("unable to allocate an audio frame");
        }

        AudioFrameMut { ptr: ptr }
    }

    /// Create a new audio frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> AudioFrameMut {
        AudioFrameMut { ptr: ptr }
    }

    /// Get frame sample format.
    pub fn sample_format(&self) -> SampleFormat {
        unsafe { ffw_frame_format(self.ptr) }
    }

    /// Get number of samples (per channel) in this frame.
    pub fn samples(&self) -> usize {
        unsafe { ffw_frame_nb_samples(self.ptr) as _ }
    }

    /// Get number of channels.
    pub fn channels(&self) -> u32 {
        unsafe { ffw_frame_channels(self.ptr) as _ }
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> ChannelLayout {
        unsafe { ffw_frame_channel_layout(self.ptr) as _ }
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_frame_pts(self.ptr) as _ }
    }

    /// Get raw pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get mutable raw pointer.
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Make the frame immutable.
    pub fn freeze(mut self) -> AudioFrame {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        AudioFrame { ptr: ptr }
    }
}

impl Drop for AudioFrameMut {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for AudioFrameMut {}
unsafe impl Sync for AudioFrameMut {}

/// Immutable audio frame.
pub struct AudioFrame {
    ptr: *mut c_void,
}

impl AudioFrame {
    /// Create a new audio frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> AudioFrame {
        AudioFrame { ptr: ptr }
    }

    /// Get frame sample format.
    pub fn sample_format(&self) -> SampleFormat {
        unsafe { ffw_frame_format(self.ptr) }
    }

    /// Get number of samples (per channel) in this frame.
    pub fn samples(&self) -> usize {
        unsafe { ffw_frame_nb_samples(self.ptr) as _ }
    }

    /// Get number of channels.
    pub fn channels(&self) -> u32 {
        unsafe { ffw_frame_channels(self.ptr) as _ }
    }

    /// Get channel layout.
    pub fn channel_layout(&self) -> ChannelLayout {
        unsafe { ffw_frame_channel_layout(self.ptr) as _ }
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_frame_pts(self.ptr) as _ }
    }

    /// Get raw pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }
}

impl Clone for AudioFrame {
    fn clone(&self) -> AudioFrame {
        let ptr = unsafe { ffw_frame_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone a frame");
        }

        AudioFrame { ptr: ptr }
    }
}

impl Drop for AudioFrame {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for AudioFrame {}
unsafe impl Sync for AudioFrame {}
