use std::slice;

use bytes::BytesMut;

use libc::{c_int, c_void, int64_t, uint8_t};

type ReadPacket =
    extern "C" fn(opaque: *mut c_void, buffer: *mut uint8_t, buffer_size: c_int) -> c_int;
type WritePacket =
    extern "C" fn(opaque: *mut c_void, buffer: *mut uint8_t, buffer_size: c_int) -> c_int;
type Seek = extern "C" fn(opaque: *mut c_void, offset: int64_t, whence: c_int) -> int64_t;

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
}

/// Common trait for writable IO contexts.
pub trait Writer {
    /// Get a pointer to the underlying AVIOContext.
    fn as_ptr(&self) -> *const c_void;

    /// Get a mut pointer to the underlying AVIOContext.
    fn as_mut_ptr(&mut self) -> *mut c_void;
}

/// IO context.
pub struct IOContext {
    ptr: *mut c_void,
}

impl IOContext {
    /// Create a new IO context from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> IOContext {
        IOContext { ptr: ptr }
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

/// A WritePacket function for the MemWriter.
extern "C" fn mem_writer_write_packet(
    opaque: *mut c_void,
    buffer: *mut uint8_t,
    buffer_size: c_int,
) -> c_int {
    let output_ptr = opaque as *mut BytesMut;

    let output = unsafe { &mut *output_ptr };

    let data = unsafe { slice::from_raw_parts(buffer as *mut u8, buffer_size as usize) };

    output.extend_from_slice(data);

    buffer_size
}

/// An AVIO writer that puts everything into memory.
pub struct MemWriter {
    io_context: IOContext,
    output: Box<BytesMut>,
}

impl MemWriter {
    /// Create a new writer.
    pub fn new() -> MemWriter {
        let mut output = Box::new(BytesMut::new());

        let output_ptr = output.as_mut() as *mut BytesMut;
        let opaque_ptr = output_ptr as *mut c_void;

        let io_context = unsafe {
            ffw_io_context_new(
                4096,
                1,
                opaque_ptr,
                None,
                Some(mem_writer_write_packet),
                None,
            )
        };

        if io_context.is_null() {
            panic!("unable to allocate an AVIO context");
        }

        let io_context = unsafe { IOContext::from_raw_ptr(io_context) };

        MemWriter {
            io_context: io_context,
            output: output,
        }
    }

    /// Take the currently buffered output data
    pub fn take_data(&mut self) -> BytesMut {
        self.output.take()
    }
}

impl Writer for MemWriter {
    fn as_ptr(&self) -> *const c_void {
        self.io_context.as_ptr()
    }

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self.io_context.as_mut_ptr()
    }
}
