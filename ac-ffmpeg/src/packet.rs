//! AVPacket interface.
//!
//! A "packet" in the FFmpeg terminology is an encoded part of an elementary
//! stream (i.e. audio or video stream).

use std::{
    convert::TryFrom,
    ffi::CStr,
    os::raw::{c_char, c_int, c_void},
    ptr, slice,
    time::Duration,
};

use crate::time::{TimeBase, Timestamp};

extern "C" {
    fn ffw_packet_alloc() -> *mut c_void;
    fn ffw_packet_new(size: c_int) -> *mut c_void;
    fn ffw_packet_clone(src: *const c_void) -> *mut c_void;
    fn ffw_packet_free(packet: *mut c_void);
    fn ffw_packet_get_size(packet: *const c_void) -> c_int;
    fn ffw_packet_get_data(packet: *mut c_void) -> *mut c_void;
    fn ffw_packet_get_pos(packet: *const c_void) -> i64;
    fn ffw_packet_set_pos(packet: *mut c_void, pos: i64);
    fn ffw_packet_get_pts(packet: *const c_void) -> i64;
    fn ffw_packet_set_pts(packet: *mut c_void, pts: i64);
    fn ffw_packet_get_dts(packet: *const c_void) -> i64;
    fn ffw_packet_set_dts(packet: *mut c_void, pts: i64);
    fn ffw_packet_get_duration(packet: *const c_void) -> i64;
    fn ffw_packet_set_duration(packet: *mut c_void, duration: i64);
    fn ffw_packet_is_key(packet: *const c_void) -> c_int;
    fn ffw_packet_set_key(packet: *mut c_void, key: c_int);
    fn ffw_packet_get_stream_index(packet: *const c_void) -> c_int;
    fn ffw_packet_set_stream_index(packet: *mut c_void, index: c_int);
    fn ffw_packet_is_writable(packet: *const c_void) -> c_int;
    fn ffw_packet_make_writable(packet: *mut c_void) -> c_int;
    fn ffw_packet_side_data_get_size(side_data: *const c_void) -> usize;
    fn ffw_packet_side_data_get_data(side_data: *const c_void) -> *const u8;
    fn ffw_packet_side_data_get_type(side_data: *const c_void) -> c_int;
    fn ffw_packet_get_side_data_name(side_data_type: c_int) -> *const c_char;
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

    /// Set packet presentation timestamp without time base.
    pub fn with_raw_pts(self, pts: i64) -> Self {
        unsafe { ffw_packet_set_pts(self.ptr, pts) }

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

    /// Set packet decoding timestamp without time base.
    pub fn with_raw_dts(self, dts: i64) -> Self {
        unsafe { ffw_packet_set_dts(self.ptr, dts) }

        self
    }

    /// Get packet duration.
    ///
    /// The method returns `None` if the duration is lower or equal to zero.
    pub fn duration(&self) -> Option<Duration> {
        let duration = self.raw_duration();

        if duration > 0 {
            let z = Timestamp::new(0, self.time_base);
            let d = Timestamp::new(duration, self.time_base);

            Some(d - z)
        } else {
            None
        }
    }

    /// Set packet duration.
    pub fn with_duration(self, duration: Duration) -> Self {
        let d = Timestamp::new(0, self.time_base) + duration;

        unsafe { ffw_packet_set_duration(self.ptr, d.timestamp()) }

        self
    }

    /// Get packet duration in time base units.
    pub fn raw_duration(&self) -> i64 {
        unsafe { ffw_packet_get_duration(self.ptr) }
    }

    /// Set packet duration in time base units.
    pub fn with_raw_duration(self, duration: i64) -> Self {
        unsafe { ffw_packet_set_duration(self.ptr, duration) }

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

    /// Get the byte position of the packet data within the input stream, if known.
    pub fn pos(&self) -> Option<u64> {
        let pos = unsafe { ffw_packet_get_pos(self.ptr) };
        if pos >= 0 {
            Some(pos as u64)
        } else {
            None
        }
    }

    /// Get the byte position of the packet data within the input stream, if known.
    pub fn with_pos(self, pos: Option<u64>) {
        let set_pos = pos.and_then(|pos| i64::try_from(pos).ok()).unwrap_or(-1);
        unsafe {
            ffw_packet_set_pos(self.ptr, set_pos);
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

    /// Set packet presentation timestamp without time base.
    pub fn with_raw_pts(self, pts: i64) -> Self {
        unsafe { ffw_packet_set_pts(self.ptr, pts) }

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

    /// Set packet decoding timestamp without time base.
    pub fn with_raw_dts(self, dts: i64) -> Self {
        unsafe { ffw_packet_set_dts(self.ptr, dts) }

        self
    }

    /// Get packet duration.
    ///
    /// The method returns `None` if the duration is lower or equal to zero.
    pub fn duration(&self) -> Option<Duration> {
        let duration = self.raw_duration();

        if duration > 0 {
            let z = Timestamp::new(0, self.time_base);
            let d = Timestamp::new(duration, self.time_base);

            Some(d - z)
        } else {
            None
        }
    }

    /// Set packet duration.
    pub fn with_duration(self, duration: Duration) -> Self {
        let d = Timestamp::new(0, self.time_base) + duration;

        unsafe { ffw_packet_set_duration(self.ptr, d.timestamp()) }

        self
    }

    /// Get packet duration in time base units.
    pub fn raw_duration(&self) -> i64 {
        unsafe { ffw_packet_get_duration(self.ptr) }
    }

    /// Set packet duration in time base units.
    pub fn with_raw_duration(self, duration: i64) -> Self {
        unsafe { ffw_packet_set_duration(self.ptr, duration) }

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

    /// Get the byte position of the packet data within the input stream, if known.
    pub fn pos(&self) -> Option<u64> {
        let pos = unsafe { ffw_packet_get_pos(self.ptr) };
        if pos >= 0 {
            Some(pos as u64)
        } else {
            None
        }
    }

    /// Try to make this packet mutable.
    ///
    /// The method returns `PacketMut` if the packet can be made mutable
    /// without copying the data, otherwise it returns `Packet`.
    pub fn try_into_mut(self) -> Result<PacketMut, Self> {
        let res = unsafe { ffw_packet_is_writable(self.ptr) };

        if res == 0 {
            Err(self)
        } else {
            Ok(self.into_mut())
        }
    }

    /// Make this packet mutable.
    ///
    /// If there are no other references to the packet data, the mutable packet
    /// will be created without copying the data.
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

/// Reference to the packet side data.
pub struct SideDataRef(());

impl SideDataRef {
    /// Create a packet side data from its raw representation.
    pub(crate) unsafe fn from_raw_ptr<'a>(ptr: *const c_void) -> &'a Self {
        unsafe { &*(ptr as *const Self) }
    }

    /// Get raw pointer.
    fn as_ptr(&self) -> *const c_void {
        self as *const Self as _
    }

    /// Get data.
    pub fn data(&self) -> &[u8] {
        unsafe {
            let data = ffw_packet_side_data_get_data(self.as_ptr());
            let len = ffw_packet_side_data_get_size(self.as_ptr());

            std::slice::from_raw_parts(data, len)
        }
    }

    /// Get data type.
    pub fn data_type(&self) -> SideDataType {
        let data_type = unsafe { ffw_packet_side_data_get_type(self.as_ptr()) };

        SideDataType::from_raw(data_type)
    }
}

/// Packet side data type.
pub struct SideDataType(c_int);

impl SideDataType {
    /// Create a packet side data type value from a given raw representation.
    pub(crate) fn from_raw(v: c_int) -> Self {
        Self(v)
    }

    /// Get the raw value.
    pub(crate) fn into_raw(self) -> c_int {
        self.0
    }

    /// Get name of the packet side data type.
    pub fn name(self) -> &'static str {
        unsafe {
            let ptr = ffw_packet_get_side_data_name(self.into_raw());

            if ptr.is_null() {
                panic!("invalid packet side data type");
            }

            let name = CStr::from_ptr(ptr as _);

            name.to_str().unwrap()
        }
    }
}
