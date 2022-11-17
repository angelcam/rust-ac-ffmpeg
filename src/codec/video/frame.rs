//! Video frame.

use std::{
    ffi::{CStr, CString},
    fmt::{self, Display, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    os::raw::{c_char, c_int, c_void},
    ptr,
    slice::{self, Chunks, ChunksMut},
    str::FromStr,
};

use crate::time::{TimeBase, Timestamp};

extern "C" {
    fn ffw_get_pixel_format_by_name(name: *const c_char) -> c_int;
    fn ffw_pixel_format_is_none(format: c_int) -> c_int;
    fn ffw_get_pixel_format_name(format: c_int) -> *const c_char;

    fn ffw_frame_new_black(pixel_format: c_int, width: c_int, height: c_int) -> *mut c_void;
    fn ffw_frame_get_format(frame: *const c_void) -> c_int;
    fn ffw_frame_get_width(frame: *const c_void) -> c_int;
    fn ffw_frame_get_height(frame: *const c_void) -> c_int;
    fn ffw_frame_get_best_effort_timestamp(frame: *const c_void) -> i64;
    fn ffw_frame_get_pts(frame: *const c_void) -> i64;
    fn ffw_frame_set_pts(frame: *mut c_void, pts: i64);
    fn ffw_frame_get_plane_data(frame: *mut c_void, index: usize) -> *mut u8;
    fn ffw_frame_get_line_size(frame: *const c_void, plane: usize) -> usize;
    fn ffw_frame_get_line_count(frame: *const c_void, plane: usize) -> usize;
    fn ffw_frame_clone(frame: *const c_void) -> *mut c_void;
    fn ffw_frame_free(frame: *mut c_void);
    fn ffw_frame_is_writable(frame: *const c_void) -> c_int;
    fn ffw_frame_make_writable(frame: *mut c_void) -> c_int;
}

/// An error indicating an unknown pixel format.
#[derive(Debug, Copy, Clone)]
pub struct UnknownPixelFormat;

impl Display for UnknownPixelFormat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str("unknown pixel format")
    }
}

impl std::error::Error for UnknownPixelFormat {}

/// Pixel format.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct PixelFormat(c_int);

impl PixelFormat {
    /// Create a pixel format value from a given raw representation.
    pub(crate) fn from_raw(v: c_int) -> Self {
        Self(v)
    }

    /// Get the raw value.
    pub(crate) fn into_raw(self) -> c_int {
        let Self(format) = self;

        format
    }

    /// Get name of the pixel format.
    pub fn name(self) -> &'static str {
        unsafe {
            let ptr = ffw_get_pixel_format_name(self.into_raw());

            if ptr.is_null() {
                panic!("invalid pixel format");
            }

            let name = CStr::from_ptr(ptr as _);

            name.to_str().unwrap()
        }
    }
}

impl FromStr for PixelFormat {
    type Err = UnknownPixelFormat;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = CString::new(s).expect("invalid pixel format name");

        unsafe {
            let format = ffw_get_pixel_format_by_name(name.as_ptr() as _);

            if ffw_pixel_format_is_none(format) == 0 {
                Ok(Self(format))
            } else {
                Err(UnknownPixelFormat)
            }
        }
    }
}

/// Get a pixel format with a given name.
pub fn get_pixel_format(name: &str) -> PixelFormat {
    PixelFormat::from_str(name).unwrap()
}

/// Picture plane (i.e. a planar array of pixel components).
pub struct Plane<'a> {
    frame: *mut c_void,
    index: usize,
    phantom: PhantomData<&'a ()>,
}

impl Plane<'_> {
    /// Create a new plane.
    fn new(frame: *mut c_void, index: usize) -> Self {
        Self {
            frame,
            index,
            phantom: PhantomData,
        }
    }

    /// Get plane data.
    pub fn data(&self) -> &[u8] {
        let line_size = self.line_size();
        let line_count = self.line_count();

        unsafe {
            let data = ffw_frame_get_plane_data(self.frame, self.index as _);

            slice::from_raw_parts(data, line_size * line_count)
        }
    }

    /// Get mutable plane data.
    pub fn data_mut(&mut self) -> &mut [u8] {
        let line_size = self.line_size();
        let line_count = self.line_count();

        unsafe {
            let data = ffw_frame_get_plane_data(self.frame, self.index as _);

            slice::from_raw_parts_mut(data, line_size * line_count)
        }
    }

    /// Get a single line.
    pub fn line(&self, index: usize) -> Option<&[u8]> {
        if index < self.line_count() {
            let line_size = self.line_size();
            let data = self.data();
            let offset = index * line_size;

            Some(&data[offset..offset + line_size])
        } else {
            None
        }
    }

    /// Get a single mutable line.
    pub fn line_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        if index < self.line_count() {
            let line_size = self.line_size();
            let data = self.data_mut();
            let offset = index * line_size;

            Some(&mut data[offset..offset + line_size])
        } else {
            None
        }
    }

    /// Get an iterator over all lines.
    pub fn lines(&self) -> LinesIter {
        let line_size = self.line_size();
        let data = self.data();

        LinesIter::new(data.chunks(line_size))
    }

    /// Get an iterator over all mutable lines.
    pub fn lines_mut(&mut self) -> LinesIterMut {
        let line_size = self.line_size();
        let data = self.data_mut();

        LinesIterMut::new(data.chunks_mut(line_size))
    }

    /// Get line size (note: the line size doesn't necessarily need to be equal to picture width).
    pub fn line_size(&self) -> usize {
        unsafe { ffw_frame_get_line_size(self.frame, self.index as _) as _ }
    }

    /// Get number of lines (note: the number of lines doesn't necessarily need to be equal to
    /// to picture height).
    pub fn line_count(&self) -> usize {
        unsafe { ffw_frame_get_line_count(self.frame, self.index as _) as _ }
    }
}

/// Iterator over plane lines.
pub struct LinesIter<'a> {
    inner: Chunks<'a, u8>,
}

impl<'a> LinesIter<'a> {
    /// Create a new iterator.
    fn new(chunks: Chunks<'a, u8>) -> Self {
        Self { inner: chunks }
    }
}

impl<'a> Iterator for LinesIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterator over plane lines.
pub struct LinesIterMut<'a> {
    inner: ChunksMut<'a, u8>,
}

impl<'a> LinesIterMut<'a> {
    /// Create a new iterator.
    fn new(chunks: ChunksMut<'a, u8>) -> Self {
        Self { inner: chunks }
    }
}

impl<'a> Iterator for LinesIterMut<'a> {
    type Item = &'a mut [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// A collection of picture planes.
pub struct Planes<'a> {
    inner: [Plane<'a>; 4],
}

impl<'a> From<&'a VideoFrame> for Planes<'a> {
    fn from(frame: &'a VideoFrame) -> Self {
        let inner = [
            Plane::new(frame.ptr, 0),
            Plane::new(frame.ptr, 1),
            Plane::new(frame.ptr, 2),
            Plane::new(frame.ptr, 3),
        ];

        Self { inner }
    }
}

impl<'a> From<&'a VideoFrameMut> for Planes<'a> {
    fn from(frame: &'a VideoFrameMut) -> Self {
        let inner = [
            Plane::new(frame.ptr, 0),
            Plane::new(frame.ptr, 1),
            Plane::new(frame.ptr, 2),
            Plane::new(frame.ptr, 3),
        ];

        Self { inner }
    }
}

impl<'a> Deref for Planes<'a> {
    type Target = [Plane<'a>];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A collection of mutable picture planes.
pub struct PlanesMut<'a> {
    inner: [Plane<'a>; 4],
}

impl<'a> From<&'a mut VideoFrameMut> for PlanesMut<'a> {
    fn from(frame: &'a mut VideoFrameMut) -> Self {
        // NOTE: creating multiple mutable references to the frame is safe here because the planes
        // are distinct
        let inner = [
            Plane::new(frame.ptr, 0),
            Plane::new(frame.ptr, 1),
            Plane::new(frame.ptr, 2),
            Plane::new(frame.ptr, 3),
        ];

        Self { inner }
    }
}

impl<'a> Deref for PlanesMut<'a> {
    type Target = [Plane<'a>];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DerefMut for PlanesMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A video frame with mutable data.
pub struct VideoFrameMut {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl VideoFrameMut {
    /// Create a black video frame. The time base of the frame will be in
    /// microseconds.
    pub fn black(pixel_format: PixelFormat, width: usize, height: usize) -> Self {
        let ptr = unsafe { ffw_frame_new_black(pixel_format.into_raw(), width as _, height as _) };

        if ptr.is_null() {
            panic!("unable to allocate a video frame");
        }

        VideoFrameMut {
            ptr,
            time_base: TimeBase::MICROSECONDS,
        }
    }

    /// Get frame pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { PixelFormat::from_raw(ffw_frame_get_format(self.ptr)) }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_frame_get_width(self.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_frame_get_height(self.ptr) as _ }
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

    /// Get picture planes.
    pub fn planes(&self) -> Planes {
        Planes::from(self)
    }

    /// Get mutable picture planes.
    pub fn planes_mut(&mut self) -> PlanesMut {
        PlanesMut::from(self)
    }

    /// Make the frame immutable.
    pub fn freeze(mut self) -> VideoFrame {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        VideoFrame {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Drop for VideoFrameMut {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for VideoFrameMut {}
unsafe impl Sync for VideoFrameMut {}

/// A video frame with immutable data.
pub struct VideoFrame {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl VideoFrame {
    /// Create a new video frame from its raw representation.
    pub(crate) unsafe fn from_raw_ptr(ptr: *mut c_void, time_base: TimeBase) -> Self {
        Self { ptr, time_base }
    }

    /// Get frame pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { PixelFormat::from_raw(ffw_frame_get_format(self.ptr)) }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_frame_get_width(self.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_frame_get_height(self.ptr) as _ }
    }

    /// Get picture planes.
    pub fn planes(&self) -> Planes {
        Planes::from(self)
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

    /// Get timestamp estimated using various heuristics.
    pub fn best_effort_timestamp(&self) -> Timestamp {
        let pts = unsafe { ffw_frame_get_best_effort_timestamp(self.ptr) };

        Timestamp::new(pts, self.time_base)
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

    /// Try to make this frame mutable. Returns VideoFrameMut if it can be made
    /// into mutable without copying the data, otherwise returns VideoFrame.
    pub fn try_into_mut(self) -> Result<VideoFrameMut, VideoFrame> {
        let res = unsafe { ffw_frame_is_writable(self.ptr) };
        if res > 0 {
            Ok(self.into_mut())
        } else {
            Err(self)
        }
    }

    /// Make this frame mutable. This will copy the data if it is not already
    /// mutable.
    pub fn into_mut(mut self) -> VideoFrameMut {
        let res = unsafe { ffw_frame_make_writable(self.ptr) };

        if res < 0 {
            panic!("unable to make the frame mutable");
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        VideoFrameMut {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Clone for VideoFrame {
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

impl Drop for VideoFrame {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for VideoFrame {}
unsafe impl Sync for VideoFrame {}
