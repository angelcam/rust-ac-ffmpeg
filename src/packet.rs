//! AVPacket interface.
//!
//! A "packet" in the FFmpeg terminology is an encoded part of an elementary
//! stream (i.e. audio or video stream).

use std::{
    os::raw::{c_int, c_void},
    ptr, slice,
};

use crate::time::{TimeBase, Timestamp};

extern "C" {
    fn ffw_packet_alloc() -> *mut c_void;
    fn ffw_packet_new(size: c_int) -> *mut c_void;
    fn ffw_packet_clone(src: *const c_void) -> *mut c_void;
    fn ffw_packet_free(packet: *mut c_void);
    fn ffw_packet_get_size(packet: *const c_void) -> c_int;
    fn ffw_packet_get_data(packet: *mut c_void) -> *mut c_void;
    fn ffw_packet_get_pts(packet: *const c_void) -> i64;
    fn ffw_packet_set_pts(packet: *mut c_void, pts: i64);
    fn ffw_packet_get_dts(packet: *const c_void) -> i64;
    fn ffw_packet_set_dts(packet: *mut c_void, pts: i64);
    fn ffw_packet_is_key(packet: *const c_void) -> c_int;
    fn ffw_packet_set_key(packet: *mut c_void, key: c_int);
    fn ffw_packet_get_stream_index(packet: *const c_void) -> c_int;
    fn ffw_packet_set_stream_index(packet: *mut c_void, index: c_int);
    fn ffw_packet_make_writable(packet: *mut c_void) -> c_int;
}

/// Packet with mutable data.
pub struct PacketMut {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl PacketMut {
    /// Create a new packet of a given size. The time base of the packet will
    /// be in microseconds.
    pub fn new(size: usize) -> Self {
        unsafe {
            let ptr = if size == 0 {
                ffw_packet_alloc()
            } else {
                ffw_packet_new(size as c_int)
            };

            if ptr.is_null() {
                panic!("unable to allocate a packet");
            }

            Self {
                ptr,
                time_base: TimeBase::MICROSECONDS,
            }
        }
    }

    /// Get stream index.
    pub fn stream_index(&self) -> usize {
        unsafe { ffw_packet_get_stream_index(self.ptr) as _ }
    }

    /// Set stream index.
    pub fn with_stream_index(self, index: usize) -> Self {
        unsafe { ffw_packet_set_stream_index(self.ptr, index as _) }

        self
    }

    /// Get packet time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Set packet time base. (This will rescale the current timestamps into a
    /// given time base.)
    pub fn with_time_base(mut self, time_base: TimeBase) -> Self {
        let new_pts = self.pts().with_time_base(time_base);
        let new_dts = self.dts().with_time_base(time_base);

        unsafe {
            ffw_packet_set_pts(self.ptr, new_pts.timestamp());
            ffw_packet_set_dts(self.ptr, new_dts.timestamp());
        }

        self.time_base = time_base;

        self
    }

    /// Get packet presentation timestamp.
    pub fn pts(&self) -> Timestamp {
        let pts = unsafe { ffw_packet_get_pts(self.ptr) };

        Timestamp::new(pts, self.time_base)
    }

    /// Set packet presentation timestamp.
    pub fn with_pts(self, pts: Timestamp) -> Self {
        let pts = pts.with_time_base(self.time_base);

        unsafe { ffw_packet_set_pts(self.ptr, pts.timestamp()) }

        self
    }

    /// Get packet decoding timestamp.
    pub fn dts(&self) -> Timestamp {
        let dts = unsafe { ffw_packet_get_dts(self.ptr) };

        Timestamp::new(dts, self.time_base)
    }

    /// Set packet decoding timestamp.
    pub fn with_dts(self, dts: Timestamp) -> Self {
        let dts = dts.with_time_base(self.time_base);

        unsafe { ffw_packet_set_dts(self.ptr, dts.timestamp()) }

        self
    }

    /// Check if the key flag is set.
    pub fn is_key(&self) -> bool {
        unsafe { ffw_packet_is_key(self.ptr) != 0 }
    }

    /// Set or unset the key flag.
    pub fn with_key_flag(self, key: bool) -> Self {
        unsafe { ffw_packet_set_key(self.ptr, key as _) }

        self
    }

    /// Get packet data.
    pub fn data(&self) -> &[u8] {
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
    pub fn data_mut(&mut self) -> &mut [u8] {
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

        Packet {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Drop for PacketMut {
    fn drop(&mut self) {
        unsafe { ffw_packet_free(self.ptr) }
    }
}

impl<T> From<T> for PacketMut
where
    T: AsRef<[u8]>,
{
    fn from(data: T) -> Self {
        let data = data.as_ref();

        let mut packet = Self::new(data.len());

        packet.data_mut().copy_from_slice(data);

        packet
    }
}

unsafe impl Send for PacketMut {}
unsafe impl Sync for PacketMut {}

/// Packet with immutable data.
pub struct Packet {
    ptr: *mut c_void,
    time_base: TimeBase,
}

impl Packet {
    /// Create a new immutable packet from its raw representation.
    pub(crate) unsafe fn from_raw_ptr(ptr: *mut c_void, time_base: TimeBase) -> Self {
        Packet { ptr, time_base }
    }

    /// Get stream index.
    pub fn stream_index(&self) -> usize {
        unsafe { ffw_packet_get_stream_index(self.ptr) as _ }
    }

    /// Set stream index.
    pub fn with_stream_index(self, index: usize) -> Packet {
        unsafe { ffw_packet_set_stream_index(self.ptr, index as _) }

        self
    }

    /// Get packet time base.
    pub fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Set packet time base. (This will rescale the current timestamps into a
    /// given time base.)
    pub fn with_time_base(mut self, time_base: TimeBase) -> Self {
        let new_pts = self.pts().with_time_base(time_base);
        let new_dts = self.dts().with_time_base(time_base);

        unsafe {
            ffw_packet_set_pts(self.ptr, new_pts.timestamp());
            ffw_packet_set_dts(self.ptr, new_dts.timestamp());
        }

        self.time_base = time_base;

        self
    }

    /// Get packet presentation timestamp.
    pub fn pts(&self) -> Timestamp {
        let pts = unsafe { ffw_packet_get_pts(self.ptr) };

        Timestamp::new(pts, self.time_base)
    }

    /// Set packet presentation timestamp.
    pub fn with_pts(self, pts: Timestamp) -> Self {
        let pts = pts.with_time_base(self.time_base);

        unsafe { ffw_packet_set_pts(self.ptr, pts.timestamp()) }

        self
    }

    /// Get packet decoding timestamp.
    pub fn dts(&self) -> Timestamp {
        let dts = unsafe { ffw_packet_get_dts(self.ptr) };

        Timestamp::new(dts, self.time_base)
    }

    /// Set packet decoding timestamp.
    pub fn with_dts(self, dts: Timestamp) -> Self {
        let dts = dts.with_time_base(self.time_base);

        unsafe { ffw_packet_set_dts(self.ptr, dts.timestamp()) }

        self
    }

    /// Check if the key flag is set.
    pub fn is_key(&self) -> bool {
        unsafe { ffw_packet_is_key(self.ptr) != 0 }
    }

    /// Get raw pointer.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.ptr
    }

    /// Get mutable raw pointer. Please note that even though it is required
    /// in some cases to pass a mut pointer to an immutable packet, it is not
    /// allowed to modify packet data in such cases.
    pub(crate) fn as_mut_ptr(&mut self) -> *mut c_void {
        self.ptr
    }

    /// Get packet data.
    pub fn data(&self) -> &[u8] {
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

    /// Make this packet mutable. If there are no other references to the
    /// packet data, the mutable packet will be created without copying the
    /// data.
    pub fn into_mut(mut self) -> PacketMut {
        let res = unsafe { ffw_packet_make_writable(self.ptr) };

        if res < 0 {
            panic!("unable to make the packet mutable");
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        PacketMut {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Clone for Packet {
    fn clone(&self) -> Packet {
        let ptr = unsafe { ffw_packet_clone(self.ptr) };

        if ptr.is_null() {
            panic!("unable to clone a packet");
        }

        Packet {
            ptr,
            time_base: self.time_base,
        }
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe { ffw_packet_free(self.ptr) }
    }
}

unsafe impl Send for Packet {}
unsafe impl Sync for Packet {}
