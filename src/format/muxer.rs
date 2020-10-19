//! A/V muxer.

use std::{ffi::CString, io::Write, ptr};

use libc::{c_char, c_int, c_uint, c_void};

use crate::{codec::CodecParameters, format::io::IO, packet::Packet, Error};

extern "C" {
    fn ffw_guess_output_format(
        short_name: *const c_char,
        file_name: *const c_char,
        mime_type: *const c_char,
    ) -> *mut c_void;

    fn ffw_muxer_new() -> *mut c_void;
    fn ffw_muxer_get_nb_streams(muxer: *const c_void) -> c_uint;
    fn ffw_muxer_new_stream(muxer: *mut c_void, params: *const c_void) -> c_int;
    fn ffw_muxer_init(muxer: *mut c_void, io_context: *mut c_void, format: *mut c_void) -> c_int;
    fn ffw_muxer_set_initial_option(
        muxer: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_muxer_set_option(muxer: *mut c_void, key: *const c_char, value: *const c_char) -> c_int;
    fn ffw_muxer_set_url(muxer: *mut c_void, url: *const c_char) -> c_int;
    fn ffw_muxer_write_frame(
        muxer: *mut c_void,
        packet: *mut c_void,
        tb_num: u32,
        tb_den: u32,
    ) -> c_int;
    fn ffw_muxer_interleaved_write_frame(
        muxer: *mut c_void,
        packet: *mut c_void,
        tb_num: u32,
        tb_den: u32,
    ) -> c_int;
    fn ffw_muxer_free(muxer: *mut c_void);
}

/// Muxer builder.
pub struct MuxerBuilder {
    ptr: *mut c_void,
    interleaved: bool,
}

impl MuxerBuilder {
    /// Create a new muxer builder.
    fn new() -> MuxerBuilder {
        let ptr = unsafe { ffw_muxer_new() };

        if ptr.is_null() {
            panic!("unable to allocate a muxer context");
        }

        MuxerBuilder {
            ptr,
            interleaved: false,
        }
    }

    /// Add a new stream with given parameters.
    pub fn add_stream(&mut self, params: &CodecParameters) -> Result<(), Error> {
        let ret = unsafe { ffw_muxer_new_stream(self.ptr, params.as_ptr()) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        Ok(())
    }

    /// Set a muxer option.
    pub fn set_option<V>(self, name: &str, value: V) -> MuxerBuilder
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            ffw_muxer_set_initial_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _)
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    pub fn set_url(self, url: &str) -> MuxerBuilder {
        let url = CString::new(url).expect("invalid URL value");
        let ret = unsafe { ffw_muxer_set_url(self.ptr, url.as_ptr() as _) };
        if ret < 0 {
            panic!("unable to allocate memory for URL")
        }
        self
    }

    /// Set the muxer to do the interleaving automatically. It is disabled by
    /// default.
    pub fn interleaved(mut self, interleaved: bool) -> MuxerBuilder {
        self.interleaved = interleaved;
        self
    }

    /// Build the muxer.
    ///
    /// # Arguments
    /// * `io_context` - an AVIO writer
    /// * `format` - an output format
    pub fn build<T>(mut self, mut io: IO<T>, format: OutputFormat) -> Result<Muxer<T>, Error>
    where
        T: Write,
    {
        let io_context_ptr = io.io_context_mut().as_mut_ptr();
        let format_ptr = format.ptr;

        let ret = unsafe { ffw_muxer_init(self.ptr, io_context_ptr, format_ptr) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        let muxer_ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = Muxer {
            ptr: muxer_ptr,
            io,
            interleaved: self.interleaved,
        };

        Ok(res)
    }
}

impl Drop for MuxerBuilder {
    fn drop(&mut self) {
        unsafe { ffw_muxer_free(self.ptr) }
    }
}

unsafe impl Send for MuxerBuilder {}
unsafe impl Sync for MuxerBuilder {}

/// Muxer.
pub struct Muxer<T> {
    ptr: *mut c_void,
    io: IO<T>,
    interleaved: bool,
}

impl Muxer<()> {
    /// Get a muxer builder.
    pub fn builder() -> MuxerBuilder {
        MuxerBuilder::new()
    }
}

impl<T> Muxer<T> {
    /// Set an option.
    pub fn set_option<V>(&mut self, name: &str, value: V) -> Result<(), Error>
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret =
            unsafe { ffw_muxer_set_option(self.ptr, name.as_ptr() as _, value.as_ptr() as _) };

        if ret < 0 {
            Err(Error::from_raw_error_code(ret))
        } else {
            Ok(())
        }
    }

    /// Mux a given packet. The packet pts and dts are expected to be in
    /// microseconds. They will be automatically rescaled to match the time
    /// base of the corresponding stream.
    pub fn push(&mut self, mut packet: Packet) -> Result<(), Error> {
        let nb_streams = unsafe { ffw_muxer_get_nb_streams(self.ptr) as usize };

        assert!(packet.stream_index() < nb_streams);

        let tb = packet.time_base();

        let ret = unsafe {
            if self.interleaved {
                ffw_muxer_interleaved_write_frame(self.ptr, packet.as_mut_ptr(), tb.num(), tb.den())
            } else {
                ffw_muxer_write_frame(self.ptr, packet.as_mut_ptr(), tb.num(), tb.den())
            }
        };

        if ret < 0 {
            Err(Error::from_raw_error_code(ret))
        } else {
            Ok(())
        }
    }

    /// Flush the muxer.
    pub fn flush(&mut self) -> Result<(), Error> {
        let ret = unsafe {
            if self.interleaved {
                ffw_muxer_interleaved_write_frame(self.ptr, ptr::null_mut(), 1, 1_000_000)
            } else {
                ffw_muxer_write_frame(self.ptr, ptr::null_mut(), 1, 1_000_000)
            }
        };

        if ret < 0 {
            Err(Error::from_raw_error_code(ret))
        } else {
            Ok(())
        }
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

impl<T> Drop for Muxer<T> {
    fn drop(&mut self) {
        unsafe { ffw_muxer_free(self.ptr) }
    }
}

unsafe impl<T> Send for Muxer<T> where T: Send {}
unsafe impl<T> Sync for Muxer<T> where T: Sync {}

/// FFmpeg output format.
pub struct OutputFormat {
    ptr: *mut c_void,
}

impl OutputFormat {
    /// Try to find an output format by its name.
    pub fn find_by_name(name: &str) -> Option<OutputFormat> {
        let name = CString::new(name).expect("invalid format name");

        let ptr =
            unsafe { ffw_guess_output_format(name.as_ptr() as *const _, ptr::null(), ptr::null()) };

        if ptr.is_null() {
            return None;
        }

        let res = OutputFormat { ptr };

        Some(res)
    }

    /// Try to find an output format by the MIME type.
    pub fn find_by_mime_type(mime_type: &str) -> Option<OutputFormat> {
        let mime_type = CString::new(mime_type).expect("invalid MIME type");

        let ptr = unsafe {
            ffw_guess_output_format(ptr::null(), ptr::null(), mime_type.as_ptr() as *const _)
        };

        if ptr.is_null() {
            return None;
        }

        let res = OutputFormat { ptr };

        Some(res)
    }

    /// Try to guess an output format from a file name.
    pub fn guess_from_file_name(file_name: &str) -> Option<OutputFormat> {
        let file_name = CString::new(file_name).expect("invalid file name");

        let ptr = unsafe {
            ffw_guess_output_format(ptr::null(), file_name.as_ptr() as *const _, ptr::null())
        };

        if ptr.is_null() {
            return None;
        }

        let res = OutputFormat { ptr };

        Some(res)
    }
}

unsafe impl Send for OutputFormat {}
unsafe impl Sync for OutputFormat {}
