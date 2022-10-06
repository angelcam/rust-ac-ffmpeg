//! A/V stream information.

use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
};

use crate::{
    codec::CodecParameters,
    time::{TimeBase, Timestamp},
    Rational,
};

extern "C" {
    fn ffw_stream_get_time_base(stream: *const c_void, num: *mut c_int, den: *mut c_int);
    fn ffw_stream_get_r_frame_rate(stream: *const c_void, num: *mut c_int, den: *mut c_int);
    fn ffw_stream_get_avg_frame_rate(stream: *const c_void, num: *mut c_int, den: *mut c_int);
    fn ffw_stream_get_start_time(stream: *const c_void) -> i64;
    fn ffw_stream_get_duration(stream: *const c_void) -> i64;
    fn ffw_stream_get_nb_frames(stream: *const c_void) -> i64;
    fn ffw_stream_get_codec_parameters(stream: *const c_void) -> *mut c_void;
    fn ffw_stream_set_metadata(
        stream: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
}

/// Stream.
pub struct Stream {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl Stream {
    /// Create a new stream from its raw representation.
    pub(crate) unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        let mut num = 0;
        let mut den = 0;

        ffw_stream_get_time_base(ptr, &mut num, &mut den);

        Stream {
            ptr,
            time_base: TimeBase::new(num, den),
        }
    }

    /// Get stream time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Get real base framerate of the stream.
    pub fn r_frame_rate(&self) -> Rational {
        let mut num = 0;
        let mut den = 0;

        unsafe {
            ffw_stream_get_r_frame_rate(self.ptr, &mut num, &mut den);
        }

        Rational::new(num, den)
    }

    /// Get average stream frame rate.
    pub fn avg_frame_rate(&self) -> Option<Rational> {
        let mut num = 0;
        let mut den = 0;

        unsafe {
            ffw_stream_get_avg_frame_rate(self.ptr, &mut num, &mut den);
        }

        if num != 0 && den != 0 {
            Some(Rational::new(num, den))
        } else {
            None
        }
    }

    /// Get the pts of the first frame of the stream in presentation order.
    pub fn start_time(&self) -> Timestamp {
        let pts = unsafe { ffw_stream_get_start_time(self.ptr) as _ };

        Timestamp::new(pts, self.time_base)
    }

    /// Get the duration of the stream.
    pub fn duration(&self) -> Timestamp {
        let pts = unsafe { ffw_stream_get_duration(self.ptr) as _ };

        Timestamp::new(pts, self.time_base)
    }

    /// Get the number of frames in the stream.
    ///
    /// # Note
    /// The number may not represent the total number of frames, depending on the type of the
    /// stream and the demuxer it may represent only the total number of keyframes.
    pub fn frames(&self) -> Option<u64> {
        let count = unsafe { ffw_stream_get_nb_frames(self.ptr) };

        if count <= 0 {
            None
        } else {
            Some(count as _)
        }
    }

    /// Get codec parameters.
    pub fn codec_parameters(&self) -> CodecParameters {
        unsafe {
            let ptr = ffw_stream_get_codec_parameters(self.ptr);

            if ptr.is_null() {
                panic!("unable to allocate codec parameters");
            }

            CodecParameters::from_raw_ptr(ptr)
        }
    }

    /// Set stream metadata.
    pub fn set_metadata<V>(&mut self, key: &str, value: V)
    where
        V: ToString,
    {
        let key = CString::new(key).expect("invalid metadata key");
        let value = CString::new(value.to_string()).expect("invalid metadata value");

        let ret = unsafe { ffw_stream_set_metadata(self.ptr, key.as_ptr(), value.as_ptr()) };

        if ret < 0 {
            panic!("unable to allocate metadata");
        }
    }
}

unsafe impl Send for Stream {}
unsafe impl Sync for Stream {}
