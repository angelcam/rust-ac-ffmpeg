use std::os::raw::{c_int, c_uint, c_void};

use crate::time::{TimeBase, Timestamp};
use crate::Error;

extern "C" {
    fn ffw_stream_from_format_context(context: *mut c_void, stream_index: c_uint) -> *mut c_void;
    fn ffw_stream_get_index(stream: *const c_void) -> c_int;
    fn ffw_stream_get_id(stream: *const c_void) -> c_int;
    fn ffw_stream_get_time_base(stream: *const c_void, num: *mut u32, den: *mut u32);
    fn ffw_stream_get_start_time(stream: *const c_void) -> i64;
    fn ffw_stream_get_duration(stream: *const c_void) -> i64;
    fn ffw_stream_get_nb_frames(stream: *const c_void) -> i64;
    fn ffw_stream_seek_frame(
        stream: *mut c_void,
        stream_index: c_uint,
        timestamp: i64,
        seek_by: c_int,
        direction: c_int,
        seek_to_keyframes_only: c_int,
    ) -> c_int;
    fn ffw_stream_free(stream: *mut c_void);
}

#[repr(i32)]
enum SeekType {
    Time,
    Byte,
    Frame,
}

#[repr(i32)]
pub enum Direction {
    Forward,
    Backward,
}

/// Stream with immutable data.
pub struct Stream {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl Stream {
    /// Create a new immutable stream from its raw representation.
    pub(crate) unsafe fn from_format_context(ptr: *mut c_void, stream_index: c_uint) -> Self {
        let stream = ffw_stream_from_format_context(ptr, stream_index);

        let mut num = 0_u32;
        let mut den = 0_u32;
        ffw_stream_get_time_base(stream, &mut num, &mut den);

        Stream {
            ptr: stream,
            time_base: TimeBase::new(num, den),
        }
    }

    /// Get stream index.
    pub fn index(&self) -> usize {
        unsafe { ffw_stream_get_index(self.ptr) as _ }
    }

    /// Get format-specific stream id.
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

    /// Get the duration of the stream.
    pub fn nb_frames(&self) -> Option<usize> {
        let count = unsafe { ffw_stream_get_nb_frames(self.ptr) as usize };

        if count == 0 {
            None
        } else {
            Some(count)
        }
    }

    /// Seek to a specific timestamp in the stream.
    #[inline]
    pub fn seek_to_timestamp(&mut self, timestamp: Timestamp, direction: Direction, keyframes_only: bool) -> Result<(), Error> {
        self.seek(timestamp.timestamp(), SeekType::Time, direction, keyframes_only)
    }

    /// Seek to a specific frame in the stream.
    #[inline]
    pub fn seek_to_frame(&mut self, frame_index: usize, direction: Direction, keyframes_only: bool) -> Result<(), Error> {
        self.seek(frame_index as _, SeekType::Frame, direction, keyframes_only)
    }

    /// Seek to a specific byte offset in the stream.
    #[inline]
    pub fn seek_to_byte(&mut self, byte: usize, direction: Direction, keyframes_only: bool) -> Result<(), Error> {
        self.seek(byte as _, SeekType::Byte, direction, keyframes_only)
    }

    /// Seek to a specific position in the stream.
    fn seek(&mut self, target_position: i64, seek_by: SeekType, direction: Direction, keyframes_only: bool) -> Result<(), Error> {
        let index = self.index() as c_uint;
        let keyframes_only = if keyframes_only == true { 1 } else { 0 };

        let res = unsafe { ffw_stream_seek_frame(self.ptr, index, target_position, seek_by as _, direction as _, keyframes_only) };

        if res >= 0 {
            Ok(())
        } else {
            Err(Error::from_raw_error_code(res))
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        unsafe { ffw_stream_free(self.ptr) }
    }
}
