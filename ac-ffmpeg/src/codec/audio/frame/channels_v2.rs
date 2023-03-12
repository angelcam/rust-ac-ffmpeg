use std::{
    borrow::Borrow,
    ffi::CString,
    io,
    ops::Deref,
    os::raw::{c_char, c_int, c_void},
    ptr,
    str::FromStr,
};

use super::UnknownChannelLayout;

use crate::Error;

extern "C" {
    fn ffw_channel_layout_get_default(layout: *mut *mut c_void, channels: u32) -> c_int;
    fn ffw_channel_layout_from_string(layout: *mut *mut c_void, s: *const c_char) -> c_int;
    fn ffw_channel_layout_clone(dst: *mut *mut c_void, src: *const c_void) -> c_int;
    fn ffw_channel_layout_is_valid(layout: *const c_void) -> c_int;
    fn ffw_channel_layout_get_channels(layout: *const c_void) -> u32;
    fn ffw_channel_layout_compare(a: *const c_void, b: *const c_void) -> c_int;
    fn ffw_channel_layout_free(layout: *mut c_void);
}

/// Channel layout reference.
pub struct ChannelLayoutRef(());

impl ChannelLayoutRef {
    /// Create channel layout reference from a given pointer.
    pub(crate) unsafe fn from_raw_ptr<'a>(ptr: *const c_void) -> &'a Self {
        &*(ptr as *const Self)
    }

    /// Get raw pointer.
    pub(crate) fn as_ptr(&self) -> *const c_void {
        self as *const Self as *const c_void
    }

    /// Get number of channels.
    pub fn channels(&self) -> u32 {
        unsafe { ffw_channel_layout_get_channels(self.as_ptr()) }
    }

    /// Check if the channel layout is valid.
    fn is_valid(&self) -> bool {
        unsafe { ffw_channel_layout_is_valid(self.as_ptr()) != 0 }
    }
}

impl PartialEq for ChannelLayoutRef {
    fn eq(&self, other: &Self) -> bool {
        unsafe { ffw_channel_layout_compare(self.as_ptr(), other.as_ptr()) == 0 }
    }
}

impl ToOwned for ChannelLayoutRef {
    type Owned = ChannelLayout;

    fn to_owned(&self) -> Self::Owned {
        unsafe {
            let mut dst = ptr::null_mut();

            let ret = ffw_channel_layout_clone(&mut dst, self.as_ptr());

            if ret == 0 {
                ChannelLayout::from_raw_ptr(dst)
            } else {
                panic!("unable to allocate channel layout")
            }
        }
    }
}

/// Channel layout.
pub struct ChannelLayout {
    ptr: *mut c_void,
}

impl ChannelLayout {
    /// Create channel layout from a given pointer.
    unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
        Self { ptr }
    }

    /// Get default channel layout for a given number of channels.
    pub fn from_channels(channels: u32) -> Option<Self> {
        unsafe {
            let mut ptr = ptr::null_mut();

            let ret = ffw_channel_layout_get_default(&mut ptr, channels);

            if ret != 0 {
                panic!("unable to allocate channel layout");
            }

            let res = Self::from_raw_ptr(ptr);

            if res.is_valid() {
                Some(res)
            } else {
                None
            }
        }
    }
}

impl Drop for ChannelLayout {
    fn drop(&mut self) {
        unsafe {
            ffw_channel_layout_free(self.ptr);
        }
    }
}

impl AsRef<ChannelLayoutRef> for ChannelLayout {
    fn as_ref(&self) -> &ChannelLayoutRef {
        unsafe { ChannelLayoutRef::from_raw_ptr(self.ptr) }
    }
}

impl Borrow<ChannelLayoutRef> for ChannelLayout {
    fn borrow(&self) -> &ChannelLayoutRef {
        self.as_ref()
    }
}

impl Deref for ChannelLayout {
    type Target = ChannelLayoutRef;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl PartialEq for ChannelLayout {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl PartialEq<ChannelLayoutRef> for ChannelLayout {
    fn eq(&self, other: &ChannelLayoutRef) -> bool {
        self.as_ref() == other
    }
}

impl PartialEq<ChannelLayout> for ChannelLayoutRef {
    fn eq(&self, other: &ChannelLayout) -> bool {
        self == other.as_ref()
    }
}

impl Clone for ChannelLayout {
    fn clone(&self) -> Self {
        self.as_ref().to_owned()
    }
}

impl FromStr for ChannelLayout {
    type Err = UnknownChannelLayout;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = CString::new(s).expect("invalid channel layout name");

        let mut ptr = ptr::null_mut();

        let ret = unsafe { ffw_channel_layout_from_string(&mut ptr, name.as_ptr()) };

        if ret == 0 {
            unsafe { Ok(Self::from_raw_ptr(ptr)) }
        } else {
            let err = Error::from_raw_error_code(ret)
                .to_io_error()
                .map(|err| err.kind());

            if err == Some(io::ErrorKind::OutOfMemory) {
                panic!("unable to allocate channel layout")
            } else {
                Err(UnknownChannelLayout)
            }
        }
    }
}

unsafe impl Send for ChannelLayout {}
unsafe impl Sync for ChannelLayout {}
