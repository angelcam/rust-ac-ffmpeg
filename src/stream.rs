//! A/V stream information.

use std::os::raw::{c_int, c_void};

use crate::time::{TimeBase, Timestamp};

extern "C" {
    fn ffw_stream_get_id(stream: *const c_void) -> c_int;
    fn ffw_stream_get_time_base(stream: *const c_void, num: *mut u32, den: *mut u32);
    fn ffw_stream_get_start_time(stream: *const c_void) -> i64;
    fn ffw_stream_get_duration(stream: *const c_void) -> i64;
    fn ffw_stream_get_nb_frames(stream: *const c_void) -> i64;
}

/// Stream with immutable data.
pub struct Stream {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl Stream {
    /// Create a new immutable stream from its raw representation.
    pub(crate) unsafe fn from_raw(ptr: *mut c_void) -> Self {
        let mut num = 0_u32;
        let mut den = 0_u32;
        ffw_stream_get_time_base(ptr, &mut num, &mut den);

        Stream {
            ptr,
            time_base: TimeBase::new(num, den),
        }
    }

    #[doc(hidden)]
    pub fn id(&self) -> usize {
        unsafe { ffw_stream_get_id(self.ptr) as _ }
    }

    /// Get stream time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
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
    /// ## Note
    /// May not represent the total number of frames, depending on the type of the stream and the
    /// demuxer it may represent only the total number of keyframes.
    pub fn frame_count(&self) -> Option<usize> {
        let count = unsafe { ffw_stream_get_nb_frames(self.ptr) as usize };

        if count == 0 {
            None
        } else {
            Some(count)
        }
    }
}
