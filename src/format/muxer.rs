use std::mem;
use std::ptr;

use std::ffi::CString;

use bytes::BytesMut;

use libc::{c_char, c_int, c_uint, c_void, uint8_t};

use Error;

use format::io::Writer;
use packet::PacketMut;

extern "C" {
    fn ffw_guess_output_format(
        short_name: *const c_char,
        file_name: *const c_char,
        mime_type: *const c_char,
    ) -> *mut c_void;

    fn ffw_muxer_new() -> *mut c_void;
    fn ffw_muxer_get_nb_streams(muxer: *const c_void) -> c_uint;
    fn ffw_muxer_new_stream(muxer: *mut c_void, codec: *const c_char) -> c_int;
    fn ffw_muxer_set_stream_extradata(
        muxer: *mut c_void,
        stream_index: c_uint,
        extradata: *mut uint8_t,
        extradata_size: c_int,
    );
    fn ffw_muxer_init(
        muxer: *mut c_void,
        io_context: *mut c_void,
        format: *mut c_void,
        options: *mut c_void,
    ) -> c_int;
    fn ffw_muxer_write_frame(muxer: *mut c_void, packet: *mut c_void) -> c_int;
    fn ffw_muxer_interleaved_write_frame(muxer: *mut c_void, packet: *mut c_void) -> c_int;
    fn ffw_muxer_free(muxer: *mut c_void);
}

/// Muxer builder.
pub struct MuxerBuilder {
    ptr: *mut c_void,
    extradata: Vec<Option<BytesMut>>,
}

impl MuxerBuilder {
    /// Create a new muxer builder.
    fn new() -> MuxerBuilder {
        let ptr = unsafe { ffw_muxer_new() };

        if ptr.is_null() {
            panic!("unable to allocate a muxer context");
        }

        MuxerBuilder {
            ptr: ptr,
            extradata: Vec::new(),
        }
    }

    /// Add a new stream with a given codec and optional extradata.
    pub fn add_stream<T>(&mut self, codec: &str, extradata: Option<T>) -> Result<(), Error>
    where
        T: Into<BytesMut>,
    {
        let codec = CString::new(codec).expect("invalid codec name");

        let res = unsafe { ffw_muxer_new_stream(self.ptr, codec.as_ptr() as *const _) };

        if res < 0 {
            return Err(Error::new("unknown codec"));
        }

        let stream_index = res as usize;

        while stream_index >= self.extradata.len() {
            self.extradata.push(None);
        }

        let mut extradata = extradata.map(|d| d.into());

        let extradata_ptr;
        let extradata_size;

        if let Some(extradata) = extradata.as_mut() {
            extradata_ptr = extradata.as_mut_ptr();
            extradata_size = extradata.len();
        } else {
            extradata_ptr = ptr::null_mut();
            extradata_size = 0;
        }

        self.extradata[stream_index] = extradata;

        unsafe {
            ffw_muxer_set_stream_extradata(
                self.ptr,
                stream_index as c_uint,
                extradata_ptr,
                extradata_size as c_int,
            );
        }

        Ok(())
    }

    /// Build the muxer.
    ///
    /// # Arguments
    /// * `io_context` - an AVIO writer
    /// * `format` - an output format
    pub fn build<T>(mut self, mut io_context: T, format: OutputFormat) -> Result<Muxer<T>, Error>
    where
        T: Writer,
    {
        let io_context_ptr = io_context.as_mut_ptr();
        let format_ptr = format.ptr;

        let res = unsafe { ffw_muxer_init(self.ptr, io_context_ptr, format_ptr, ptr::null_mut()) };

        if res < 0 {
            return Err(Error::new("unable to initialize the muxer"));
        }

        let muxer_ptr = self.ptr;
        let extradata = mem::replace(&mut self.extradata, Vec::new());

        self.ptr = ptr::null_mut();

        let res = Muxer {
            ptr: muxer_ptr,
            extradata: extradata,
            io_context: Some(io_context),
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
    #[allow(dead_code)]
    extradata: Vec<Option<BytesMut>>,
    #[allow(dead_code)]
    io_context: Option<T>,
}

impl Muxer<()> {
    /// Get a muxer builder.
    pub fn builder() -> MuxerBuilder {
        MuxerBuilder::new()
    }
}

impl<T> Muxer<T> {
    /// Write a given frame.
    pub fn write_frame(&mut self, mut packet: PacketMut) -> Result<(), Error> {
        let nb_streams = unsafe { ffw_muxer_get_nb_streams(self.ptr) as usize };

        assert!(packet.stream_index() < nb_streams);

        let res = unsafe { ffw_muxer_write_frame(self.ptr, packet.as_mut_ptr()) };

        if res < 0 {
            Err(Error::new("unable to write a given packet"))
        } else {
            Ok(())
        }
    }

    /// Write a given frame and handle interleaving internally.
    pub fn interleaved_write_frame(&mut self, mut packet: PacketMut) -> Result<(), Error> {
        let nb_streams = unsafe { ffw_muxer_get_nb_streams(self.ptr) as usize };

        assert!(packet.stream_index() < nb_streams);

        // note: this is OK even though the function takes ownership of the
        // packet data because we still need to deallocate the envelope
        let res = unsafe { ffw_muxer_interleaved_write_frame(self.ptr, packet.as_mut_ptr()) };

        if res < 0 {
            Err(Error::new("unable to write a given packet"))
        } else {
            Ok(())
        }
    }

    /// Flush the muxer.
    pub fn flush(&mut self) -> Result<(), Error> {
        let mut res = 0;

        while res != 1 {
            res = unsafe { ffw_muxer_interleaved_write_frame(self.ptr, ptr::null_mut()) };

            if res < 0 {
                return Err(Error::new("unable to write a given packet"));
            }
        }

        Ok(())
    }

    /// Take the output.
    pub fn output(mut self) -> T {
        self.io_context.take().unwrap()
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

        let res = OutputFormat { ptr: ptr };

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

        let res = OutputFormat { ptr: ptr };

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

        let res = OutputFormat { ptr: ptr };

        Some(res)
    }
}
