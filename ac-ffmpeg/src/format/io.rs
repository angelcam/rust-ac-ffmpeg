//! Elementary IO used by the muxer and demuxer.

use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    os::raw::{c_int, c_void},
    slice,
};

type ReadPacketCallback =
    extern "C" fn(opaque: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int;
type WritePacketCallback =
    extern "C" fn(opaque: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int;
type SeekCallback = extern "C" fn(opaque: *mut c_void, offset: i64, whence: c_int) -> i64;

extern "C" {
    fn ffw_io_is_avseek_size(whence: c_int) -> c_int;

    fn ffw_io_context_new(
        buffer_size: c_int,
        write_flag: c_int,
        opaque: *mut c_void,
        read_packet: Option<ReadPacketCallback>,
        write_packet: Option<WritePacketCallback>,
        seek: Option<SeekCallback>,
    ) -> *mut c_void;
    fn ffw_io_context_free(context: *mut c_void);
}

/// IO context.
#[allow(clippy::upper_case_acronyms)]
pub(crate) struct IOContext {
    ptr: *mut c_void,
}

impl IOContext {
    /// Create a new IO context from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        IOContext { ptr }
    }

    /// Get a mut pointer to the underlying AVIOContext.
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }
}

impl Drop for IOContext {
    fn drop(&mut self) {
        unsafe { ffw_io_context_free(self.ptr) }
    }
}

unsafe impl Send for IOContext {}
unsafe impl Sync for IOContext {}

/// Helper function to get the length of a seekable stream. It will be replaced
/// by `Seek::stream_len()` once it gets stabilized.
fn get_seekable_length<T>(seekable: &mut T) -> Result<u64, std::io::Error>
where
    T: Seek,
{
    let current_position = seekable.seek(SeekFrom::Current(0))?;
    let end_position = seekable.seek(SeekFrom::End(0))?;

    seekable.seek(SeekFrom::Start(current_position))?;

    Ok(end_position)
}

/// A SeekCallback function for the IO.
extern "C" fn io_seek<T>(opaque: *mut c_void, offset: i64, whence: c_int) -> i64
where
    T: Seek,
{
    let input_ptr = opaque as *mut T;

    let input = unsafe { &mut *input_ptr };

    let is_avseek_size = unsafe { ffw_io_is_avseek_size(whence) != 0 };

    let seek = if is_avseek_size {
        get_seekable_length(input)
    } else {
        input.seek(SeekFrom::Start(offset as u64))
    };

    match seek {
        Ok(len) => len as i64,
        Err(err) => err
            .raw_os_error()
            .map(|code| unsafe { crate::ffw_error_from_posix(code as _) })
            .unwrap_or_else(|| unsafe { crate::ffw_error_unknown() }) as i64,
    }
}

/// A ReadPacketCallback function for the IO.
extern "C" fn io_read_packet<T>(opaque: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int
where
    T: Read,
{
    let input_ptr = opaque as *mut T;

    let input = unsafe { &mut *input_ptr };

    let buffer = unsafe { slice::from_raw_parts_mut(buffer, buffer_size as usize) };

    match input.read(buffer) {
        Ok(n) => {
            if n > 0 {
                n as c_int
            } else {
                unsafe { crate::ffw_error_eof() }
            }
        }
        Err(err) => {
            if let Some(code) = err.raw_os_error() {
                unsafe { crate::ffw_error_from_posix(code as _) }
            } else if err.kind() == io::ErrorKind::WouldBlock {
                unsafe { crate::ffw_error_would_block() }
            } else {
                unsafe { crate::ffw_error_unknown() }
            }
        }
    }
}

/// A WritePacketCallback function for the IO.
extern "C" fn io_write_packet<T>(opaque: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int
where
    T: Write,
{
    let output_ptr = opaque as *mut T;

    let output = unsafe { &mut *output_ptr };

    if !buffer.is_null() && buffer_size > 0 {
        let buffer = unsafe { slice::from_raw_parts(buffer, buffer_size as usize) };

        match output.write(buffer) {
            Ok(n) => {
                if n > 0 {
                    n as c_int
                } else {
                    unsafe { crate::ffw_error_eof() }
                }
            }
            Err(err) => {
                if let Some(code) = err.raw_os_error() {
                    unsafe { crate::ffw_error_from_posix(code as _) }
                } else if err.kind() == io::ErrorKind::WouldBlock {
                    unsafe { crate::ffw_error_would_block() }
                } else {
                    unsafe { crate::ffw_error_unknown() }
                }
            }
        }
    } else if let Err(err) = output.flush() {
        if let Some(code) = err.raw_os_error() {
            unsafe { crate::ffw_error_from_posix(code) }
        } else if err.kind() == io::ErrorKind::WouldBlock {
            unsafe { crate::ffw_error_would_block() }
        } else {
            unsafe { crate::ffw_error_unknown() }
        }
    } else {
        0
    }
}

/// An AVIO IO that connects FFmpeg AVIO context with Rust streams.
#[allow(clippy::upper_case_acronyms)]
pub struct IO<T> {
    io_context: IOContext,
    stream: Box<T>,
}

impl<T> IO<T> {
    /// Create a new IO.
    fn new(
        stream: T,
        read_packet: Option<ReadPacketCallback>,
        write_packet: Option<WritePacketCallback>,
        seek: Option<SeekCallback>,
    ) -> Self {
        let mut stream = Box::new(stream);
        let stream_ptr = stream.as_mut() as *mut T;
        let opaque_ptr = stream_ptr as *mut c_void;

        let write_flag = i32::from(write_packet.is_some());

        let io_context = unsafe {
            ffw_io_context_new(
                4096,
                write_flag,
                opaque_ptr,
                read_packet,
                write_packet,
                seek,
            )
        };

        if io_context.is_null() {
            panic!("unable to allocate an AVIO context");
        }

        let io_context = unsafe { IOContext::from_raw_ptr(io_context) };

        Self { io_context, stream }
    }

    /// Get mutable reference to the underlying IO context.
    pub(crate) fn io_context_mut(&mut self) -> &mut IOContext {
        &mut self.io_context
    }

    /// Get reference to the underlying stream.
    pub fn stream(&self) -> &T {
        self.stream.as_ref()
    }

    /// Get mutable reference to the underlying stream.
    pub fn stream_mut(&mut self) -> &mut T {
        self.stream.as_mut()
    }

    /// Take the underlying stream dropping this IO.
    pub fn into_stream(self) -> T {
        *self.stream
    }
}

impl<T> IO<T>
where
    T: Read,
{
    /// Create a new IO from a given stream.
    pub fn from_read_stream(stream: T) -> Self {
        Self::new(stream, Some(io_read_packet::<T>), None, None)
    }
}

impl<T> IO<T>
where
    T: Read + Seek,
{
    /// Create a new IO from a given stream.
    pub fn from_seekable_read_stream(stream: T) -> Self {
        Self::new(stream, Some(io_read_packet::<T>), None, Some(io_seek::<T>))
    }
}

impl<T> IO<T>
where
    T: Write,
{
    /// Create a new IO from a given stream.
    pub fn from_write_stream(stream: T) -> Self {
        Self::new(stream, None, Some(io_write_packet::<T>), None)
    }
}

impl<T> IO<T>
where
    T: Write + Seek,
{
    /// Create a new IO from a given stream.
    pub fn from_seekable_write_stream(stream: T) -> Self {
        Self::new(stream, None, Some(io_write_packet::<T>), Some(io_seek::<T>))
    }
}

/// Writer that puts everything in memory. It also allows taking the data on
/// the fly.
#[derive(Default)]
pub struct MemWriter {
    data: Vec<u8>,
}

impl MemWriter {
    /// Take data from the writer.
    pub fn take_data(&mut self) -> Vec<u8> {
        let res = Vec::from(self.data.as_slice());

        self.data.clear();

        res
    }
}

impl Write for MemWriter {
    fn write(&mut self, buffer: &[u8]) -> Result<usize, io::Error> {
        self.data.extend_from_slice(buffer);

        Ok(buffer.len())
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}
