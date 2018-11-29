use std::ptr;
use std::slice;

use libc::{c_int, c_void};

extern "C" {
    fn ffw_packet_alloc() -> *mut c_void;
    fn ffw_packet_new(size: c_int) -> *mut c_void;
    fn ffw_packet_clone(src: *const c_void) -> *mut c_void;
    fn ffw_packet_free(packet: *mut c_void);
    fn ffw_packet_get_size(packet: *const c_void) -> c_int;
    fn ffw_packet_get_data(packet: *mut c_void) -> *mut c_void;
}

/// Mutable packet.
pub struct PacketMut {
    ptr: *mut c_void,
}

impl PacketMut {
    /// Create a new packet of a given size.
    pub fn new(size: usize) -> PacketMut {
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

            PacketMut::from_raw_ptr(ptr)
        }
    }

    /// Create a new packet from its raw representation.
    pub unsafe fn from_raw_ptr(ptr: *mut c_void) -> PacketMut {
        PacketMut { ptr: ptr }
    }

    /// Get raw pointer.
    pub fn raw_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get mutable raw pointer.
    pub fn mut_ptr(&mut self) -> *mut c_void {
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

impl<T> From<T> for PacketMut
where
    T: AsRef<[u8]>,
{
    fn from(data: T) -> PacketMut {
        let bytes = data.as_ref();

        let mut packet = PacketMut::new(bytes.len());

        packet.bytes_mut().copy_from_slice(bytes);

        packet
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

    /// Get raw pointer.
    pub fn raw_ptr(&self) -> *const c_void {
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
