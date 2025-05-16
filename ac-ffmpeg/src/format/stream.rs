//! A/V stream information.

use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
};

use crate::{
    codec::CodecParameters,
    time::{TimeBase, Timestamp},
};

#[cfg(stream_side_data)]
use crate::{
    packet::{SideDataRef, SideDataType},
    Error,
};

extern "C" {
    fn ffw_stream_get_time_base(stream: *const c_void, num: *mut u32, den: *mut u32);
    fn ffw_stream_set_time_base(stream: *mut c_void, num: u32, den: u32);
    fn ffw_stream_get_start_time(stream: *const c_void) -> i64;
    fn ffw_stream_get_duration(stream: *const c_void) -> i64;
    fn ffw_stream_get_nb_frames(stream: *const c_void) -> i64;
    fn ffw_stream_get_codec_parameters(stream: *const c_void) -> *mut c_void;
    fn ffw_stream_get_id(stream: *const c_void) -> c_int;
    fn ffw_stream_set_metadata(
        stream: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_stream_set_id(stream: *mut c_void, id: c_int);

    #[cfg(stream_side_data)]
    fn ffw_stream_get_nb_side_data(stream: *const c_void) -> usize;

    #[cfg(stream_side_data)]
    fn ffw_stream_get_side_data(stream: *const c_void, index: usize) -> *const c_void;

    #[cfg(stream_side_data)]
    fn ffw_stream_add_side_data(
        stream: *mut c_void,
        data_type: c_int,
        data: *const u8,
        size: usize,
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
        let mut num = 0_u32;
        let mut den = 0_u32;

        ffw_stream_get_time_base(ptr, &mut num, &mut den);

        Stream {
            ptr,
            time_base: TimeBase::new(num, den),
        }
    }

    /// Get stream time base.
    #[inline]
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Provide a hint to the muxer about the desired timebase.
    pub fn set_time_base(&mut self, time_base: TimeBase) {
        self.time_base = time_base;
        unsafe {
            ffw_stream_set_time_base(self.ptr, self.time_base.num(), self.time_base.den());
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

    /// Get stream id.
    pub fn stream_id(&self) -> i32 {
        unsafe { ffw_stream_get_id(self.ptr) as i32 }
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

    /// Set stream id.
    pub fn set_stream_id(&mut self, id: i32) {
        unsafe { ffw_stream_set_id(self.ptr, id as c_int) };
    }

    /// Get stream side data.
    #[cfg(stream_side_data)]
    pub fn side_data(&self) -> SideDataIter<'_> {
        let len = unsafe { ffw_stream_get_nb_side_data(self.ptr) };

        SideDataIter {
            stream: self,
            index: 0,
            len,
        }
    }

    /// Add stream side data.
    #[cfg(stream_side_data)]
    pub fn add_side_data(&mut self, data_type: SideDataType, data: &[u8]) -> Result<(), Error> {
        let ret = unsafe {
            ffw_stream_add_side_data(self.ptr, data_type.into_raw(), data.as_ptr(), data.len())
        };

        if ret < 0 {
            Err(Error::from_raw_error_code(ret))
        } else {
            Ok(())
        }
    }
}

unsafe impl Send for Stream {}
unsafe impl Sync for Stream {}

/// Iterator over stream side data.
#[cfg(stream_side_data)]
pub struct SideDataIter<'a> {
    stream: &'a Stream,
    index: usize,
    len: usize,
}

#[cfg(stream_side_data)]
impl<'a> Iterator for SideDataIter<'a> {
    type Item = &'a SideDataRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.len {
            return None;
        }

        let side_data = unsafe {
            SideDataRef::from_raw_ptr(ffw_stream_get_side_data(self.stream.ptr, self.index))
        };

        self.index += 1;

        Some(side_data)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let hint = self.len - self.index;

        (hint, Some(hint))
    }
}

#[cfg(stream_side_data)]
impl ExactSizeIterator for SideDataIter<'_> {}
