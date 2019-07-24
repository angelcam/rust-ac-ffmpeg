use std::io;
use std::slice;

use std::io::{Read, Write};

use bytes::{Bytes, BytesMut};

use libc::{c_int, c_void};

type ReadPacket = extern "C" fn(opaque: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int;
type WritePacket = extern "C" fn(opaque: *mut c_void, buffer: *mut u8, buffer_size: c_int) -> c_int;
type Seek = extern "C" fn(opaque: *mut c_void, offset: i64, whence: c_int) -> i64;

extern "C" {
    fn ffw_io_context_new(
        buffer_size: c_int,
        write_flag: c_int,
        opaque: *mut c_void,
        read_packet: Option<ReadPacket>,
        write_packet: Option<WritePacket>,
        seek: Option<Seek>,
    ) -> *mut c_void;
    fn ffw_io_context_free(context: *mut c_void);
    fn ffw_io_error_eof() -> c_int;
    fn ffw_io_error_would_block() -> c_int;
    fn ffw_io_error_unknown() -> c_int;
    fn ffw_io_error_posix(error: c_int) -> c_int;
}

/// Marker trait for readable IOs.
pub trait Reader {}

/// Marker trait for writable IOs.
pub trait Writer {}

/// IO context.
pub struct IOContext {
    ptr: *mut c_void,
}

impl IOContext {
    /// Create a new IO context from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        IOContext { ptr }
    }

    /// Get a pointer to the underlying AVIOContext.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
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

/// A ReadPacket function for the IOReader.
extern "C" fn io_reader_read_packet<T>(
    opaque: *mut c_void,
    buffer: *mut u8,
    buffer_size: c_int,
) -> c_int
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
                unsafe { ffw_io_error_eof() }
            }
        }
        Err(err) => {
            if let Some(code) = err.raw_os_error() {
                unsafe { ffw_io_error_posix(code as _) }
            } else if err.kind() == io::ErrorKind::WouldBlock {
                unsafe { ffw_io_error_would_block() }
            } else {
                unsafe { ffw_io_error_unknown() }
            }
        }
    }
}

/// An AVIO reader that reads data from a given input.
pub struct IOReader<T> {
    io_context: IOContext,
    input: Box<T>,
}

impl<T> IOReader<T>
where
    T: Read,
{
    /// Create a new AVIO reader from a given input.
    pub fn new(input: T) -> Self {
        Self::from(Box::new(input))
    }

    /// Get reference to the underlying input.
    pub fn input(&self) -> &T {
        self.input.as_ref()
    }

    /// Get mutable reference to the underlying input.
    pub fn input_mut(&mut self) -> &mut T {
        self.input.as_mut()
    }
}

impl<T> From<Box<T>> for IOReader<T>
where
    T: Read,
{
    fn from(mut input: Box<T>) -> Self {
        let input_ptr = input.as_mut() as *mut T;
        let opaque_ptr = input_ptr as *mut c_void;

        let io_context = unsafe {
            ffw_io_context_new(
                4096,
                0,
                opaque_ptr,
                Some(io_reader_read_packet::<T>),
                None,
                None,
            )
        };

        if io_context.is_null() {
            panic!("unable to allocate an AVIO context");
        }

        let io_context = unsafe { IOContext::from_raw_ptr(io_context) };

        Self { io_context, input }
    }
}

impl<T> AsRef<IOContext> for IOReader<T> {
    fn as_ref(&self) -> &IOContext {
        &self.io_context
    }
}

impl<T> AsMut<IOContext> for IOReader<T> {
    fn as_mut(&mut self) -> &mut IOContext {
        &mut self.io_context
    }
}

impl<T> Reader for IOReader<T> {}

/// Helper struct.
struct BytesReader {
    data: Bytes,
    closed: bool,
}

impl BytesReader {
    /// Push more data to the reader.
    fn push_data<T>(&mut self, data: T)
    where
        T: AsRef<[u8]>,
    {
        if self.closed {
            panic!("unable to push more data, the input has been already closed");
        }

        self.data.extend_from_slice(data.as_ref());
    }

    /// Close the reader.
    fn close(&mut self) {
        self.closed = true;
    }
}

impl Default for BytesReader {
    fn default() -> Self {
        Self::from(Bytes::new())
    }
}

impl From<Bytes> for BytesReader {
    fn from(data: Bytes) -> Self {
        Self {
            data,
            closed: false,
        }
    }
}

impl Read for BytesReader {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, io::Error> {
        if self.data.is_empty() {
            if self.closed {
                return Ok(0);
            } else {
                return Err(io::Error::from(io::ErrorKind::WouldBlock));
            }
        }

        let available = self.data.len();
        let capacity = buffer.len();

        let take = available.min(capacity);

        let data = self.data.split_to(take);
        let buffer = &mut buffer[..take];

        buffer.copy_from_slice(data.as_ref());

        Ok(take)
    }
}

/// An AVIO reader that takes everything from memory.
pub struct MemReader {
    inner: IOReader<BytesReader>,
}

impl MemReader {
    /// Create a new memory reader for given data.
    pub fn new<T>(data: T) -> Self
    where
        T: AsRef<[u8]>,
    {
        Self::from(Bytes::from(data.as_ref()))
    }
}

impl From<Bytes> for MemReader {
    fn from(data: Bytes) -> Self {
        let mut reader = BytesReader::from(data);

        reader.close();

        Self {
            inner: IOReader::new(reader),
        }
    }
}

impl AsRef<IOContext> for MemReader {
    fn as_ref(&self) -> &IOContext {
        self.inner.as_ref()
    }
}

impl AsMut<IOContext> for MemReader {
    fn as_mut(&mut self) -> &mut IOContext {
        self.inner.as_mut()
    }
}

impl Reader for MemReader {}

/// An AVIO reader that takes everything from memory. This reader allows asynchronous operation.
/// You can push more data (if available). You also need to call the close method in order to tell
/// the consumer that there will be no more data.
pub struct AsyncMemReader {
    inner: IOReader<BytesReader>,
}

impl AsyncMemReader {
    /// Push more data to the input.
    pub fn push_data<T>(&mut self, data: T)
    where
        T: AsRef<[u8]>,
    {
        self.inner.input_mut().push_data(data)
    }

    /// Close the input.
    pub fn close(&mut self) {
        self.inner.input_mut().close()
    }
}

impl Default for AsyncMemReader {
    fn default() -> Self {
        Self {
            inner: IOReader::new(BytesReader::default()),
        }
    }
}

impl AsRef<IOContext> for AsyncMemReader {
    fn as_ref(&self) -> &IOContext {
        self.inner.as_ref()
    }
}

impl AsMut<IOContext> for AsyncMemReader {
    fn as_mut(&mut self) -> &mut IOContext {
        self.inner.as_mut()
    }
}

impl Reader for AsyncMemReader {}

/// A WritePacket function for the IOWriter.
extern "C" fn io_writer_write_packet<T>(
    opaque: *mut c_void,
    buffer: *mut u8,
    buffer_size: c_int,
) -> c_int
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
                    unsafe { ffw_io_error_eof() }
                }
            }
            Err(err) => {
                if let Some(code) = err.raw_os_error() {
                    unsafe { ffw_io_error_posix(code as _) }
                } else if err.kind() == io::ErrorKind::WouldBlock {
                    unsafe { ffw_io_error_would_block() }
                } else {
                    unsafe { ffw_io_error_unknown() }
                }
            }
        }
    } else if let Err(err) = output.flush() {
        if let Some(code) = err.raw_os_error() {
            unsafe { ffw_io_error_posix(code) }
        } else if err.kind() == io::ErrorKind::WouldBlock {
            unsafe { ffw_io_error_would_block() }
        } else {
            unsafe { ffw_io_error_unknown() }
        }
    } else {
        0
    }
}

/// An AVIO writer that writes data into a given output.
pub struct IOWriter<T> {
    io_context: IOContext,
    output: Box<T>,
}

impl<T> IOWriter<T>
where
    T: Write,
{
    /// Create a new AVIO writer from a given output.
    pub fn new(output: T) -> Self {
        Self::from(Box::new(output))
    }

    /// Get reference to the underlying output.
    pub fn output(&self) -> &T {
        self.output.as_ref()
    }

    /// Get mutable reference to the underlying output.
    pub fn output_mut(&mut self) -> &mut T {
        self.output.as_mut()
    }
}

impl<T> From<Box<T>> for IOWriter<T>
where
    T: Write,
{
    fn from(mut output: Box<T>) -> Self {
        let output_ptr = output.as_mut() as *mut T;
        let opaque_ptr = output_ptr as *mut c_void;

        let io_context = unsafe {
            ffw_io_context_new(
                4096,
                1,
                opaque_ptr,
                None,
                Some(io_writer_write_packet::<T>),
                None,
            )
        };

        if io_context.is_null() {
            panic!("unable to allocate an AVIO context");
        }

        let io_context = unsafe { IOContext::from_raw_ptr(io_context) };

        Self { io_context, output }
    }
}

impl<T> AsRef<IOContext> for IOWriter<T> {
    fn as_ref(&self) -> &IOContext {
        &self.io_context
    }
}

impl<T> AsMut<IOContext> for IOWriter<T> {
    fn as_mut(&mut self) -> &mut IOContext {
        &mut self.io_context
    }
}

impl<T> Writer for IOWriter<T> {}

/// Helper struct.
struct BytesMutWriter {
    inner: BytesMut,
}

impl BytesMutWriter {
    /// Take the currently buffered data.
    fn take_data(&mut self) -> BytesMut {
        self.inner.take()
    }
}

impl Default for BytesMutWriter {
    fn default() -> Self {
        Self {
            inner: BytesMut::new(),
        }
    }
}

impl Write for BytesMutWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.inner.extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}

/// An AVIO writer that puts everything into memory.
pub struct MemWriter {
    inner: IOWriter<BytesMutWriter>,
}

impl MemWriter {
    /// Take the currently buffered output data.
    pub fn take_data(&mut self) -> BytesMut {
        self.inner.output_mut().take_data()
    }
}

impl Default for MemWriter {
    fn default() -> Self {
        Self {
            inner: IOWriter::new(BytesMutWriter::default()),
        }
    }
}

impl AsRef<IOContext> for MemWriter {
    fn as_ref(&self) -> &IOContext {
        self.inner.as_ref()
    }
}

impl AsMut<IOContext> for MemWriter {
    fn as_mut(&mut self) -> &mut IOContext {
        self.inner.as_mut()
    }
}

impl Writer for MemWriter {}
