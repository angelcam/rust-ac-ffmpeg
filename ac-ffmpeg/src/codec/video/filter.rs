//! Video filter.

use crate::{
    codec::{CodecError, Filter},
    time::TimeBase,
    Error,
};
use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
    ptr,
};

use super::{VideoCodecParameters, VideoFrame};

extern "C" {
    fn ffw_filtergraph_new() -> *mut c_void;
    fn ffw_filtersource_new(
        source: *mut *mut c_void,
        graph: *mut c_void,
        codec: *mut c_void,
        tb_num: c_int,
        tb_den: c_int,
    ) -> c_int;
    fn ffw_filtersink_new(sink: *mut *mut c_void, graph: *mut c_void) -> c_int;
    fn ffw_filtergraph_init(
        graph: *mut c_void,
        source: *mut c_void,
        sink: *mut c_void,
        filters_descr: *const c_char,
    ) -> c_int;
    fn ffw_filtergraph_push_frame(context: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_filtergraph_take_frame(context: *mut c_void, frame: *mut *mut c_void) -> c_int;
    fn ffw_filtergraph_free(context: *mut c_void);
}

/// A builder for video filters.
pub struct VideoFilterBuilder {
    ptr: *mut c_void,
    input_time_base: Option<TimeBase>,
    output_time_base: Option<TimeBase>,
    description: Option<String>,
    codec_parameters: Option<VideoCodecParameters>,
}

impl VideoFilterBuilder {
    /// Create a video filter builder with the given description.
    fn new() -> Self {
        let graph = unsafe { ffw_filtergraph_new() };
        if graph.is_null() {
            panic!("unable to allocate a filtergraph");
        }
        Self {
            ptr: graph,
            input_time_base: None,
            output_time_base: None,
            description: None,
            codec_parameters: None,
        }
    }

    /// Set input codec parameters.
    pub fn input_codec_parameters(mut self, codec_parameters: &VideoCodecParameters) -> Self {
        self.codec_parameters = Some(codec_parameters.to_owned());
        self
    }

    /// Set input time base.
    pub fn input_time_base(mut self, time_base: TimeBase) -> Self {
        self.input_time_base = Some(time_base);
        self
    }

    /// Set output time base.
    pub fn output_time_base(mut self, time_base: TimeBase) -> Self {
        self.output_time_base = Some(time_base);
        self
    }

    /// Set fillter description, which describes the nodes in the filter graph.
    pub fn filter_description(mut self, filters_description: &str) -> Self {
        self.description = Some(filters_description.to_owned());
        self
    }

    /// Build the filtergraph.
    pub fn build(mut self) -> Result<VideoFilter, Error> {
        // vaidate params
        let filters_descr = self
            .description
            .take()
            .map(CString::new)
            .ok_or_else(|| Error::new("filter description not set"))?
            .expect("invalid filter description");
        let codec_parameters = self
            .codec_parameters
            .take()
            .ok_or_else(|| Error::new("codec parameters not set"))?;
        let input_time_base = self
            .input_time_base
            .ok_or_else(|| Error::new("input time base not set"))?;

        // fallback on input timebase if not supplied
        let output_time_base = self.output_time_base.unwrap_or(input_time_base);

        // init source and sink buffer filters
        let mut source = ptr::null_mut();
        let ret = unsafe {
            ffw_filtersource_new(
                &mut source,
                self.ptr,
                codec_parameters.as_ptr() as _,
                input_time_base.num() as _,
                input_time_base.den() as _,
            )
        };
        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        } else if source.is_null() {
            return Err(Error::new("unable to allocate a source"));
        }

        let mut sink = ptr::null_mut();
        let ret = unsafe { ffw_filtersink_new(&mut sink, self.ptr) };
        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        } else if sink.is_null() {
            return Err(Error::new("unable to allocate a source"));
        }

        // init the filtergraph
        let ret = unsafe {
            ffw_filtergraph_init(
                self.ptr,
                source as _,
                sink as _,
                filters_descr.as_ptr() as _,
            )
        };
        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        let ptr = self.ptr;
        self.ptr = ptr::null_mut();

        Ok(VideoFilter {
            ptr,
            source,
            sink,
            input_time_base: input_time_base,
            output_time_base: output_time_base,
        })
    }
}

unsafe impl Send for VideoFilter {}
unsafe impl Sync for VideoFilter {}

impl Drop for VideoFilterBuilder {
    fn drop(&mut self) {
        unsafe { ffw_filtergraph_free(self.ptr) }
    }
}

pub struct VideoFilter {
    ptr: *mut c_void,
    source: *mut c_void,
    sink: *mut c_void,
    input_time_base: TimeBase,
    output_time_base: TimeBase,
}

impl VideoFilter {
    pub fn builder() -> VideoFilterBuilder {
        VideoFilterBuilder::new()
    }
}

impl Drop for VideoFilter {
    fn drop(&mut self) {
        unsafe { ffw_filtergraph_free(self.ptr) }
    }
}

unsafe impl Send for VideoFilterBuilder {}
unsafe impl Sync for VideoFilterBuilder {}

impl Filter for VideoFilter {
    type Frame = VideoFrame;

    /// Push a given frame to the filter.
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

    /// Flush the filter.
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

    /// Take the next packet from the filter.
    fn take(&mut self) -> Result<Option<VideoFrame>, Error> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match ffw_filtergraph_take_frame(self.sink, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(VideoFrame::from_raw_ptr(fptr, self.output_time_base)))
                    }
                }
                0 => Ok(None),
                e => Err(Error::from_raw_error_code(e)),
            }
        }
    }
}
