//! A/V demuxer.

use std::{
    borrow::{Borrow, BorrowMut},
    ffi::CString,
    io::Read,
    ops::{Deref, DerefMut},
    os::raw::{c_char, c_int, c_uint, c_void},
    ptr,
};

use crate::{codec::CodecParameters, format::io::IO, packet::Packet, time::TimeBase, Error};

extern "C" {
    fn ffw_find_input_format(short_name: *const c_char) -> *mut c_void;

    fn ffw_demuxer_new() -> *mut c_void;
    fn ffw_demuxer_init(
        demuxer: *mut c_void,
        io_context: *mut c_void,
        format: *mut c_void,
    ) -> c_int;
    fn ffw_demuxer_set_initial_option(
        demuxer: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_demuxer_set_option(
        demuxer: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_demuxer_find_stream_info(demuxer: *mut c_void, max_analyze_duration: i64) -> c_int;
    fn ffw_demuxer_get_nb_streams(demuxer: *const c_void) -> c_uint;
    fn ffw_demuxer_get_codec_parameters(
        demuxer: *const c_void,
        stream_index: c_uint,
    ) -> *mut c_void;
    fn ffw_demuxer_read_frame(
        demuxer: *mut c_void,
        packet: *mut *mut c_void,
        tb_num: *mut u32,
        tb_den: *mut u32,
    ) -> c_int;
    fn ffw_demuxer_free(demuxer: *mut c_void);
}

/// Demuxer builder.
pub struct DemuxerBuilder {
    ptr: *mut c_void,
    input_format: Option<InputFormat>,
}

impl DemuxerBuilder {
    /// Create a new demuxer builder.
    fn new() -> DemuxerBuilder {
        let ptr = unsafe { ffw_demuxer_new() };

        if ptr.is_null() {
            panic!("unable to allocate a demuxer context");
        }

        DemuxerBuilder {
            ptr,
            input_format: None,
        }
    }

    /// Set a demuxer option.
    pub fn set_option<V>(self, name: &str, value: V) -> DemuxerBuilder
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            ffw_demuxer_set_initial_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _)
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Set input format. If the input format is not set, it will be guessed from the input.
    pub fn input_format(mut self, format: Option<InputFormat>) -> DemuxerBuilder {
        self.input_format = format;
        self
    }

    /// Build the demuxer.
    ///
    /// # Arguments
    /// * `io` - an AVIO reader
    pub fn build<T>(mut self, mut io: IO<T>) -> Result<Demuxer<T>, Error>
    where
        T: Read,
    {
        let io_context_ptr = io.io_context_mut().as_mut_ptr();

        let format_ptr = self
            .input_format
            .take()
            .map(|f| f.ptr)
            .unwrap_or(ptr::null_mut());

        let ret = unsafe { ffw_demuxer_init(self.ptr, io_context_ptr, format_ptr) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = Demuxer { ptr, io };

        Ok(res)
    }
}

impl Drop for DemuxerBuilder {
    fn drop(&mut self) {
        unsafe { ffw_demuxer_free(self.ptr) }
    }
}

unsafe impl Send for DemuxerBuilder {}
unsafe impl Sync for DemuxerBuilder {}

/// Demuxer.
pub struct Demuxer<T> {
    ptr: *mut c_void,
    io: IO<T>,
}

impl Demuxer<()> {
    /// Get a demuxer builder.
    pub fn builder() -> DemuxerBuilder {
        DemuxerBuilder::new()
    }
}

impl<T> Demuxer<T> {
    /// Set an option.
    pub fn set_option<V>(&mut self, name: &str, value: V) -> Result<(), Error>
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret =
            unsafe { ffw_demuxer_set_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _) };

        if ret < 0 {
            Err(Error::from_raw_error_code(ret))
        } else {
            Ok(())
        }
    }

    /// Take the next packet from the demuxer or None on EOF. The pts and dts fields of the
    /// returned packet will be in microseconds.
    pub fn take(&mut self) -> Result<Option<Packet>, Error> {
        let mut pptr = ptr::null_mut();

        let mut tb_num = 0;
        let mut tb_den = 0;

        let ret = unsafe { ffw_demuxer_read_frame(self.ptr, &mut pptr, &mut tb_num, &mut tb_den) };

        if ret < 0 {
            Err(Error::from_raw_error_code(ret))
        } else if pptr.is_null() {
            Ok(None)
        } else {
            let packet = unsafe { Packet::from_raw_ptr(pptr, TimeBase::new(tb_num, tb_den)) };

            Ok(Some(packet))
        }
    }

    /// Try to find stream info. Optionally, you can pass `max_analyze_duration` which will tell
    /// FFmpeg how far it should look for stream info. The `max_analyze_duration` should be in
    /// microseconds.
    pub fn find_stream_info(
        self,
        max_analyze_duration: Option<i64>,
    ) -> Result<DemuxerWithCodecParameters<T>, (Demuxer<T>, Error)> {
        let max_analyze_duration = max_analyze_duration.unwrap_or(0);

        let ret = unsafe { ffw_demuxer_find_stream_info(self.ptr, max_analyze_duration) };

        if ret < 0 {
            return Err((self, Error::from_raw_error_code(ret)));
        }

        let stream_count = unsafe { ffw_demuxer_get_nb_streams(self.ptr) };

        let mut codec_parameters = Vec::with_capacity(stream_count as usize);

        for i in 0..stream_count {
            let params = unsafe {
                let ptr = ffw_demuxer_get_codec_parameters(self.ptr, i as _);

                if ptr.is_null() {
                    panic!("unable to allocate codec parameters");
                }

                CodecParameters::from_raw_ptr(ptr)
            };

            codec_parameters.push(params);
        }

        let res = DemuxerWithCodecParameters {
            inner: self,
            codec_parameters,
        };

        Ok(res)
    }

    /// Get reference to the underlying IO.
    pub fn io(&self) -> &IO<T> {
        &self.io
    }

    /// Get mutable reference to the underlying IO.
    pub fn io_mut(&mut self) -> &mut IO<T> {
        &mut self.io
    }
}

impl<T> Drop for Demuxer<T> {
    fn drop(&mut self) {
        unsafe { ffw_demuxer_free(self.ptr) }
    }
}

unsafe impl<T> Send for Demuxer<T> where T: Send {}
unsafe impl<T> Sync for Demuxer<T> where T: Sync {}

/// Demuxer with information about individual streams.
pub struct DemuxerWithCodecParameters<T> {
    inner: Demuxer<T>,
    codec_parameters: Vec<CodecParameters>,
}

impl<T> DemuxerWithCodecParameters<T> {
    /// Get codec parameters.
    pub fn codec_parameters(&self) -> &[CodecParameters] {
        &self.codec_parameters
    }

    /// Deconstruct this object into a plain demuxer and codec parameters.
    pub fn deconstruct(self) -> (Demuxer<T>, Vec<CodecParameters>) {
        (self.inner, self.codec_parameters)
    }
}

impl<T> AsRef<Demuxer<T>> for DemuxerWithCodecParameters<T> {
    fn as_ref(&self) -> &Demuxer<T> {
        &self.inner
    }
}

impl<T> AsMut<Demuxer<T>> for DemuxerWithCodecParameters<T> {
    fn as_mut(&mut self) -> &mut Demuxer<T> {
        &mut self.inner
    }
}

impl<T> Borrow<Demuxer<T>> for DemuxerWithCodecParameters<T> {
    fn borrow(&self) -> &Demuxer<T> {
        &self.inner
    }
}

impl<T> BorrowMut<Demuxer<T>> for DemuxerWithCodecParameters<T> {
    fn borrow_mut(&mut self) -> &mut Demuxer<T> {
        &mut self.inner
    }
}

impl<T> Deref for DemuxerWithCodecParameters<T> {
    type Target = Demuxer<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for DemuxerWithCodecParameters<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// FFmpeg input format.
pub struct InputFormat {
    ptr: *mut c_void,
}

impl InputFormat {
    /// Try to find an input format by its name.
    pub fn find_by_name(name: &str) -> Option<InputFormat> {
        let name = CString::new(name).expect("invalid format name");

        let ptr = unsafe { ffw_find_input_format(name.as_ptr() as *const _) };

        if ptr.is_null() {
            return None;
        }

        let res = InputFormat { ptr };

        Some(res)
    }
}

unsafe impl Send for InputFormat {}
unsafe impl Sync for InputFormat {}
