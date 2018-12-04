use std::ptr;
use std::slice;

use libc::{c_int, c_void, int64_t};

extern "C" {
    fn ffw_packet_alloc() -> *mut c_void;
    fn ffw_packet_new(size: c_int) -> *mut c_void;
    fn ffw_packet_clone(src: *const c_void) -> *mut c_void;
    fn ffw_packet_free(packet: *mut c_void);
    fn ffw_packet_get_size(packet: *const c_void) -> c_int;
    fn ffw_packet_get_data(packet: *mut c_void) -> *mut c_void;
    fn ffw_packet_get_pts(packet: *const c_void) -> int64_t;
    fn ffw_packet_set_pts(packet: *mut c_void, pts: int64_t);
    fn ffw_packet_get_dts(packet: *const c_void) -> int64_t;
    fn ffw_packet_set_dts(packet: *mut c_void, pts: int64_t);
    fn ffw_packet_is_key(packet: *const c_void) -> c_int;
    fn ffw_packet_set_key(packet: *mut c_void, key: c_int);
    fn ffw_packet_get_stream_index(packet: *const c_void) -> c_int;
    fn ffw_packet_set_stream_index(packet: *mut c_void, index: c_int);
}

/// Packet builder.
pub struct PacketBuilder {
    ptr: *mut c_void,
}

impl PacketBuilder {
    /// Create a packet builder for a packet of a given size.
    fn new(size: usize) -> PacketBuilder {
        unsafe {
            let ptr;

            if size == 0 {
                ptr = ffw_packet_alloc();
            } else {
                ptr = ffw_packet_new(size as c_int);
            }

            if ptr.is_null() {
                panic!("unable to allocate a packet");
            }

            PacketBuilder { ptr: ptr }
        }
    }

    /// Set stream index.
    pub fn stream_index(self, index: usize) -> PacketBuilder {
        unsafe { ffw_packet_set_stream_index(self.ptr, index as _) }

        self
    }

    /// Set packet presentation timestamp.
    pub fn pts(self, pts: i64) -> PacketBuilder {
        unsafe { ffw_packet_set_pts(self.ptr, pts as _) }

        self
    }

    /// Set packet decoding timestamp.
    pub fn dts(self, dts: i64) -> PacketBuilder {
        unsafe { ffw_packet_set_dts(self.ptr, dts as _) }

        self
    }

    /// Set or unset the key flag.
    pub fn key(self, key: bool) -> PacketBuilder {
        unsafe { ffw_packet_set_key(self.ptr, key as _) }

        self
    }

    /// Build the packet.
    pub fn build(mut self) -> PacketMut {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        PacketMut { ptr: ptr }
    }
}

impl Drop for PacketBuilder {
    fn drop(&mut self) {
        unsafe { ffw_packet_free(self.ptr) }
    }
}

impl<T> From<T> for PacketBuilder
where
    T: AsRef<[u8]>,
{
    fn from(data: T) -> PacketBuilder {
        let data = data.as_ref();

        let builder = PacketBuilder::new(data.len());

        let dst = unsafe {
            let data = ffw_packet_get_data(builder.ptr) as *mut u8;
            let size = ffw_packet_get_size(builder.ptr) as usize;

            if data.is_null() {
                &mut []
            } else {
                slice::from_raw_parts_mut(data, size)
            }
        };

        dst.copy_from_slice(data);

        builder
    }
}

unsafe impl Send for PacketBuilder {}
unsafe impl Sync for PacketBuilder {}

/// Mutable packet.
pub struct PacketMut {
    ptr: *mut c_void,
}

impl PacketMut {
    /// Create a new packet builder for a packet of a given size.
    pub fn builder(size: usize) -> PacketBuilder {
        PacketBuilder::new(size)
    }

    /// Create a new packet from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> PacketMut {
        PacketMut { ptr: ptr }
    }

    /// Get stream index.
    pub fn stream_index(&self) -> usize {
        unsafe { ffw_packet_get_stream_index(self.ptr) as _ }
    }

    /// Get packet presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_packet_get_pts(self.ptr) as _ }
    }

    /// Get packet decoding timestamp.
    pub fn dts(&self) -> i64 {
        unsafe { ffw_packet_get_dts(self.ptr) as _ }
    }

    /// Check if the key flag is set.
    pub fn is_key(&self) -> bool {
        unsafe { ffw_packet_is_key(self.ptr) != 0 }
    }

    /// Get raw pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get mutable raw pointer.
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Get packet data.
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            let data = ffw_packet_get_data(self.ptr) as *const u8;
            let size = ffw_packet_get_size(self.ptr) as usize;

            if data.is_null() {
                &[]
            } else {
                slice::from_raw_parts(data, size)
            }
        }
    }

    /// Get mutable reference to the packet data.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe {
            let data = ffw_packet_get_data(self.ptr) as *mut u8;
            let size = ffw_packet_get_size(self.ptr) as usize;

            if data.is_null() {
                &mut []
            } else {
                slice::from_raw_parts_mut(data, size)
            }
        }
    }

    /// Make the packet immutable.
    pub fn freeze(mut self) -> Packet {
        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        Packet { ptr: ptr }
    }
}

impl Drop for PacketMut {
    fn drop(&mut self) {
        unsafe { ffw_packet_free(self.ptr) }
    }
}

unsafe impl Send for PacketMut {}
unsafe impl Sync for PacketMut {}

/// Immutable packet.
pub struct Packet {
    ptr: *mut c_void,
}

impl Packet {
    /// Create a new immutable packet from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> Packet {
        Packet { ptr: ptr }
    }

    /// Get stream index.
    pub fn stream_index(&self) -> usize {
        unsafe { ffw_packet_get_stream_index(self.ptr) as _ }
    }

    /// Get packet presentation timestamp.
    pub fn pts(&self) -> i64 {
        unsafe { ffw_packet_get_pts(self.ptr) as _ }
    }

    /// Get packet decoding timestamp.
    pub fn dts(&self) -> i64 {
        unsafe { ffw_packet_get_dts(self.ptr) as _ }
    }

    /// Get raw pointer.
    pub fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get packet data.
    pub fn bytes(&self) -> &[u8] {
        unsafe {
            let data = ffw_packet_get_data(self.ptr) as *const u8;
            let size = ffw_packet_get_size(self.ptr) as usize;

            if data.is_null() {
                &[]
            } else {
                slice::from_raw_parts(data, size)
            }
        }
    }
}

impl Clone for Packet {
    fn clone(&self) -> Packet {
        let ptr = unsafe { ffw_packet_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone a packet");
        }

        Packet { ptr: ptr }
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe { ffw_packet_free(self.ptr) }
    }
}

unsafe impl Send for Packet {}
unsafe impl Sync for Packet {}
