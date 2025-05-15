//! Video filter.

use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
    ptr,
};

use crate::{
    codec::{
        video::frame::{PixelFormat, VideoFrame},
        CodecError, Filter,
    },
    time::TimeBase,
    Error,
};

extern "C" {
    fn ffw_filtergraph_new() -> *mut c_void;
    fn ffw_filtersource_new(
        filter_ctx: *mut *mut c_void,
        filter_graph: *mut c_void,
        src_args: *const c_char,
    ) -> c_int;
    fn ffw_filtersink_new(filter_ctx: *mut *mut c_void, filter_graph: *mut c_void) -> c_int;
    fn ffw_filtergraph_init(
        filter_graph: *mut c_void,
        buffersrc_ctx: *mut c_void,
        buffersink_ctx: *mut c_void,
        filters_descr: *const c_char,
    ) -> c_int;
    fn ffw_filtergraph_push_frame(context: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_filtergraph_take_frame(context: *mut c_void, frame: *mut *mut c_void) -> c_int;
    fn ffw_filtergraph_free(context: *mut c_void);
}

/// A builder for video filters.
pub struct VideoFilterBuilder {
    ptr: *mut c_void,

    width: usize,
    height: usize,
    pixel_fmt: PixelFormat,
    pixel_aspect_num: u32,
    pixel_aspect_den: u32,
    input_time_base: TimeBase,
    output_time_base: Option<TimeBase>,
}

impl VideoFilterBuilder {
    /// Create a new video filter builder.
    fn new(width: usize, height: usize, pixel_fmt: PixelFormat) -> Self {
        let ptr = unsafe { ffw_filtergraph_new() };

        if ptr.is_null() {
            panic!("unable to allocate a filter graph");
        }

        Self {
            ptr,

            width,
            height,
            pixel_fmt,
            pixel_aspect_num: 1,
            pixel_aspect_den: 1,
            input_time_base: TimeBase::MICROSECONDS,
            output_time_base: None,
        }
    }

    /// Set the pixel aspect ratio (the default is `1:1`).
    pub fn pixel_aspect_ratio(mut self, num: u32, den: u32) -> Self {
        self.pixel_aspect_num = num;
        self.pixel_aspect_den = den;
        self
    }

    /// Set input time base (the default is `1/1000000`).
    pub fn input_time_base(mut self, time_base: TimeBase) -> Self {
        self.input_time_base = time_base;
        self
    }

    /// Set output time base.
    pub fn output_time_base(mut self, time_base: TimeBase) -> Self {
        self.output_time_base = Some(time_base);
        self
    }

    /// Build the filtergraph.
    pub fn build(mut self, filter_description: &str) -> Result<VideoFilter, Error> {
        let filter_description =
            CString::new(filter_description).expect("invalid filter description");

        let output_time_base = self.output_time_base.unwrap_or(self.input_time_base);

        let src_args = format!(
            "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
            self.width,
            self.height,
            self.pixel_fmt.into_raw(),
            self.input_time_base.num(),
            self.input_time_base.den(),
            self.pixel_aspect_num,
            self.pixel_aspect_den,
        );

        let src_args = CString::new(src_args).unwrap();

        // init source buffer
        let mut source = ptr::null_mut();

        let ret = unsafe { ffw_filtersource_new(&mut source, self.ptr, src_args.as_ptr()) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        } else if source.is_null() {
            panic!("unable to allocate a filter source buffer");
        }

        // init sink buffer
        let mut sink = ptr::null_mut();

        let ret = unsafe { ffw_filtersink_new(&mut sink, self.ptr) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        } else if sink.is_null() {
            panic!("unable to allocate a filter sink buffer");
        }

        // initialize the filter graph
        let ret =
            unsafe { ffw_filtergraph_init(self.ptr, source, sink, filter_description.as_ptr()) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        let res = VideoFilter {
            ptr: std::mem::replace(&mut self.ptr, ptr::null_mut()),
            source,
            sink,
            input_time_base: self.input_time_base,
            output_time_base,
        };

        Ok(res)
    }
}

impl Drop for VideoFilterBuilder {
    fn drop(&mut self) {
        unsafe { ffw_filtergraph_free(self.ptr) }
    }
}

unsafe impl Send for VideoFilterBuilder {}
unsafe impl Sync for VideoFilterBuilder {}

/// Video filter.
pub struct VideoFilter {
    ptr: *mut c_void,
    source: *mut c_void,
    sink: *mut c_void,
    input_time_base: TimeBase,
    output_time_base: TimeBase,
}

impl VideoFilter {
    /// Get a video filter builder.
    pub fn builder(width: usize, height: usize, pixel_fmt: PixelFormat) -> VideoFilterBuilder {
        VideoFilterBuilder::new(width, height, pixel_fmt)
    }
}

impl Drop for VideoFilter {
    fn drop(&mut self) {
        unsafe { ffw_filtergraph_free(self.ptr) }
    }
}

unsafe impl Send for VideoFilter {}
unsafe impl Sync for VideoFilter {}

impl Filter for VideoFilter {
    type Frame = VideoFrame;

    fn try_push(&mut self, frame: VideoFrame) -> Result<(), CodecError> {
        let frame = frame.with_time_base(self.input_time_base);

        unsafe {
            match ffw_filtergraph_push_frame(self.source, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all frames must be consumed before pushing a new frame",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn try_flush(&mut self) -> Result<(), CodecError> {
        unsafe {
            match ffw_filtergraph_push_frame(self.source, ptr::null()) {
                1 => Ok(()),
                0 => Err(CodecError::again(
                    "all frames must be consumed before flushing",
                )),
                e => Err(CodecError::from_raw_error_code(e)),
            }
        }
    }

    fn take(&mut self) -> Result<Option<VideoFrame>, Error> {
        let mut ptr = ptr::null_mut();

        unsafe {
            match ffw_filtergraph_take_frame(self.sink, &mut ptr) {
                1 if ptr.is_null() => panic!("no frame received"),
                1 => Ok(Some(VideoFrame::from_raw_ptr(ptr, self.output_time_base))),
                0 => Ok(None),
                e => Err(Error::from_raw_error_code(e)),
            }
        }
    }
}
