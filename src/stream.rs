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
        seek_target: c_int,
    ) -> c_int;
    fn ffw_stream_free(stream: *mut c_void);
}

#[repr(i32)]
enum SeekType {
    Time,
    Byte,
    Frame,
}

/// Used to specify a search direction when a stream cannot seek exactly to the requested target
/// point; timestamp, frame or byte.
#[repr(i32)]
pub enum SeekTarget {
    /// Seek, at least, to the requested target point in the stream. If the target cannot be met
    /// then move forward in the stream until a possible seek target can be hit.
    From,
    /// Seek, at most, to the requested target point in the stream. If the target cannot be met
    /// then move backward in the stream until a possible seek target can be hit.
    UpTo,
    /// Force seeking to the requested target point in the stream, even if the Demuxer for this
    /// type of stream, does not support it.
    Precise,
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

    /// Seek to a specific timestamp in the stream.
    #[inline]
    pub fn seek_to_timestamp(
        &self,
        timestamp: Timestamp,
        seek_target: SeekTarget,
    ) -> Result<(), Error> {
        self.seek(
            timestamp.with_time_base(self.time_base).timestamp(),
            SeekType::Time,
            seek_target,
        )
    }

    /// Seek to a specific frame in the stream.
    #[inline]
    pub fn seek_to_frame(&self, frame_index: usize, seek_target: SeekTarget) -> Result<(), Error> {
        self.seek(frame_index as _, SeekType::Frame, seek_target)
    }

    /// Seek to a specific byte offset in the stream.
    #[inline]
    pub fn seek_to_byte(&self, byte: usize, seek_target: SeekTarget) -> Result<(), Error> {
        self.seek(byte as _, SeekType::Byte, seek_target)
    }

    fn seek(
        &self,
        target_position: i64,
        seek_by: SeekType,
        seek_target: SeekTarget,
    ) -> Result<(), Error> {
        let index = self.index() as c_uint;

        let res = unsafe {
            ffw_stream_seek_frame(
                self.ptr,
                index,
                target_position,
                seek_by as _,
                seek_target as _,
            )
        };

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
