use std::ptr;
use std::slice;

use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::slice::{Chunks, ChunksMut};

use libc::{c_char, c_int, c_void, size_t};

extern "C" {
    fn ffw_get_pixel_format_by_name(name: *const c_char) -> c_int;
    fn ffw_pixel_format_is_none(format: c_int) -> c_int;
    fn ffw_get_pixel_format_name(format: c_int) -> *const c_char;

    fn ffw_frame_new_black(pixel_format: c_int, width: c_int, height: c_int) -> *mut c_void;
    fn ffw_frame_get_format(frame: *const c_void) -> c_int;
    fn ffw_frame_get_width(frame: *const c_void) -> c_int;
    fn ffw_frame_get_height(frame: *const c_void) -> c_int;
    fn ffw_frame_get_pts(frame: *const c_void) -> i64;
    fn ffw_frame_set_pts(frame: *mut c_void, pts: i64);
    fn ffw_frame_get_plane_data(frame: *mut c_void, index: size_t) -> *mut u8;
    fn ffw_frame_get_line_size(frame: *const c_void, plane: size_t) -> size_t;
    fn ffw_frame_get_line_count(frame: *const c_void, plane: size_t) -> size_t;
    fn ffw_frame_clone(frame: *const c_void) -> *mut c_void;
    fn ffw_frame_free(frame: *mut c_void);
}

/// Pixel format.
pub type PixelFormat = c_int;

/// Get pixel format with a given name.
pub fn get_pixel_format(name: &str) -> PixelFormat {
    let name = CString::new(name).expect("invalid pixel format name");

    unsafe {
        let format = ffw_get_pixel_format_by_name(name.as_ptr() as _);

        if ffw_pixel_format_is_none(format) != 0 {
            panic!("no such pixel format");
        }

        format
    }
}

/// Get name of a given pixel format.
pub fn get_pixel_format_name(format: PixelFormat) -> &'static str {
    unsafe {
        let ptr = ffw_get_pixel_format_name(format);

        if ptr.is_null() {
            panic!("invalid pixel format");
        }

        let name = CStr::from_ptr(ptr as _);

        name.to_str().unwrap()
    }
}

/// Picture plane (i.e. a planar array of pixel components).
pub struct Plane<'a> {
    frame: *mut c_void,
    index: usize,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Plane<'a> {
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

    /// Get an iterator over all lines.
    pub fn lines(&self) -> LinesIter {
        let line_size = self.line_size();
        let data = self.data();

        LinesIter::new(data.chunks(line_size))
    }

    /// Get line size (note: the line size don't necessarily need to be equal to picture width).
    pub fn line_size(&self) -> usize {
        unsafe { ffw_frame_get_line_size(self.frame, self.index as _) as _ }
    }

    /// Get number of lines (note: the number of lines don't necessarily need to be equal to
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

/// Mutable picture plane (i.e. a planar array of pixel components).
pub struct PlaneMut<'a> {
    plane: Plane<'a>,
}

impl<'a> PlaneMut<'a> {
    /// Create a new plane.
    fn new(frame: *mut c_void, index: usize) -> Self {
        Self {
            plane: Plane::new(frame, index),
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

    /// Get an iterator over all mutable lines.
    pub fn lines_mut(&mut self) -> LinesIterMut {
        let line_size = self.line_size();
        let data = self.data_mut();

        LinesIterMut::new(data.chunks_mut(line_size))
    }
}

impl<'a> Deref for PlaneMut<'a> {
    type Target = Plane<'a>;

    fn deref(&self) -> &Self::Target {
        &self.plane
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
    inner: [PlaneMut<'a>; 4],
}

impl<'a> From<&'a mut VideoFrameMut> for PlanesMut<'a> {
    fn from(frame: &'a mut VideoFrameMut) -> Self {
        // NOTE: creating multiple mutable references to the frame is safe here because the planes
        // are distinct
        let inner = [
            PlaneMut::new(frame.ptr, 0),
            PlaneMut::new(frame.ptr, 1),
            PlaneMut::new(frame.ptr, 2),
            PlaneMut::new(frame.ptr, 3),
        ];

        Self { inner }
    }
}

impl<'a> Deref for PlanesMut<'a> {
    type Target = [PlaneMut<'a>];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DerefMut for PlanesMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Mutable video frame.
pub struct VideoFrameMut {
    ptr: *mut c_void,
}

impl VideoFrameMut {
    /// Create a black video frame.
    pub fn black(pixel_format: PixelFormat, width: usize, height: usize) -> VideoFrameMut {
        let ptr = unsafe { ffw_frame_new_black(pixel_format, width as _, height as _) };

        if ptr.is_null() {
            panic!("unable to allocate a video frame");
        }

        VideoFrameMut { ptr }
    }

    /// Create a new video frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> VideoFrameMut {
        VideoFrameMut { ptr }
    }

    /// Get frame pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { ffw_frame_get_format(self.ptr) }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_frame_get_width(self.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_frame_get_height(self.ptr) as _ }
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_frame_get_pts(self.ptr) }
    }

    /// Set presentation timestamp.
    pub fn with_pts(self, pts: i64) -> VideoFrameMut {
        unsafe { ffw_frame_set_pts(self.ptr, pts) }

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

    /// Get raw pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get mutable raw pointer.
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Make the frame immutable.
    pub fn freeze(mut self) -> VideoFrame {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        VideoFrame { ptr }
    }
}

impl Drop for VideoFrameMut {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for VideoFrameMut {}
unsafe impl Sync for VideoFrameMut {}

/// Immutable video frame.
pub struct VideoFrame {
    ptr: *mut c_void,
}

impl VideoFrame {
    /// Create a new video frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> VideoFrame {
        VideoFrame { ptr }
    }

    /// Get frame pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        unsafe { ffw_frame_get_format(self.ptr) }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_frame_get_width(self.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_frame_get_height(self.ptr) as _ }
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_frame_get_pts(self.ptr) }
    }

    /// Get picture planes.
    pub fn planes(&self) -> Planes {
        Planes::from(self)
    }

    /// Set presentation timestamp.
    pub fn with_pts(self, pts: i64) -> VideoFrame {
        unsafe { ffw_frame_set_pts(self.ptr, pts) }

        self
    }

    /// Get raw pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }
}

impl Clone for VideoFrame {
    fn clone(&self) -> VideoFrame {
        let ptr = unsafe { ffw_frame_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone a frame");
        }

        VideoFrame { ptr }
    }
}

impl Drop for VideoFrame {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for VideoFrame {}
unsafe impl Sync for VideoFrame {}
