use std::ptr;

use std::ffi::{CStr, CString};

use libc::{c_char, c_int, c_void, int64_t};

extern "C" {
    fn ffw_get_pixel_format_by_name(name: *const c_char) -> c_int;
    fn ffw_pixel_format_is_none(format: c_int) -> c_int;
    fn ffw_get_pixel_format_name(format: c_int) -> *const c_char;

    fn ffw_frame_new_black(pixel_format: c_int, width: c_int, height: c_int) -> *mut c_void;
    fn ffw_frame_get_format(frame: *const c_void) -> c_int;
    fn ffw_frame_get_width(frame: *const c_void) -> c_int;
    fn ffw_frame_get_height(frame: *const c_void) -> c_int;
    fn ffw_frame_get_pts(frame: *const c_void) -> int64_t;
    fn ffw_frame_set_pts(frame: *mut c_void, pts: int64_t);
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

        VideoFrameMut { ptr: ptr }
    }

    /// Create a new video frame from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> VideoFrameMut {
        VideoFrameMut { ptr: ptr }
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
        unsafe { ffw_frame_get_pts(self.ptr) as _ }
    }

    /// Set presentation timestamp.
    pub fn with_pts(self, pts: i64) -> VideoFrameMut {
        unsafe { ffw_frame_set_pts(self.ptr, pts as _) }

        self
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

        VideoFrame { ptr: ptr }
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
        VideoFrame { ptr: ptr }
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
        unsafe { ffw_frame_get_pts(self.ptr) as _ }
    }

    /// Set presentation timestamp.
    pub fn with_pts(self, pts: i64) -> VideoFrame {
        unsafe { ffw_frame_set_pts(self.ptr, pts as _) }

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

        VideoFrame { ptr: ptr }
    }
}

impl Drop for VideoFrame {
    fn drop(&mut self) {
        unsafe { ffw_frame_free(self.ptr) }
    }
}

unsafe impl Send for VideoFrame {}
unsafe impl Sync for VideoFrame {}
