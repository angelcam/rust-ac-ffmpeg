use std::ptr;

use libc::{c_int, c_void, int64_t};

pub type Format = c_int;

extern "C" {
    fn ffw_frame_format(frame: *const c_void) -> c_int;
    fn ffw_frame_width(frame: *const c_void) -> c_int;
    fn ffw_frame_height(frame: *const c_void) -> c_int;
    fn ffw_frame_pts(frame: *const c_void) -> int64_t;
    fn ffw_frame_clone(frame: *const c_void) -> *mut c_void;
    fn ffw_frame_free(frame: *mut c_void);
}

/// Mutable video frame.
pub struct FrameMut {
    ptr: *mut c_void,
}

impl FrameMut {
    /// Create a new video frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> Frame {
        Frame { ptr: ptr }
    }

    /// Get frame pixel format.
    pub fn format(&self) -> Format {
        unsafe { ffw_frame_format(self.ptr) }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_frame_width(self.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_frame_height(self.ptr) as _ }
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_frame_pts(self.ptr) as _ }
    }

    /// Get raw pointer.
    pub fn raw_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get mutable raw pointer.
    pub fn mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Make the frame immutable.
    pub fn freeze(mut self) -> Frame {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        Frame { ptr: ptr }
    }
}

impl Drop for FrameMut {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for FrameMut {}
unsafe impl Sync for FrameMut {}

/// Immutable video frame.
pub struct Frame {
    ptr: *mut c_void,
}

impl Frame {
    /// Create a new video frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> Frame {
        Frame { ptr: ptr }
    }

    /// Get frame pixel format.
    pub fn format(&self) -> Format {
        unsafe { ffw_frame_format(self.ptr) }
    }

    /// Get frame width.
    pub fn width(&self) -> usize {
        unsafe { ffw_frame_width(self.ptr) as _ }
    }

    /// Get frame height.
    pub fn height(&self) -> usize {
        unsafe { ffw_frame_height(self.ptr) as _ }
    }

    /// Get presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_frame_pts(self.ptr) as _ }
    }

    /// Get raw pointer.
    pub fn raw_ptr(&self) -> *const c_void {
        self.ptr
    }
}

impl Clone for Frame {
    fn clone(&self) -> Frame {
        let ptr = unsafe { ffw_frame_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone a frame");
        }

        Frame { ptr: ptr }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}
