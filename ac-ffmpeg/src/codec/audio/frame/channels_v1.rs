use std::{
    borrow::Borrow,
    ffi::CString,
    ops::Deref,
    os::raw::{c_char, c_int, c_void},
    str::FromStr,
};

use super::UnknownChannelLayout;

extern "C" {
    fn ffw_get_channel_layout_by_name(name: *const c_char) -> u64;
    fn ffw_get_channel_layout_channels(layout: u64) -> c_int;
    fn ffw_get_default_channel_layout(channels: c_int) -> u64;
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
        unsafe { ffw_get_channel_layout_channels(self.to_raw()) as _ }
    }

    /// Get the raw representation.
    fn to_raw(&self) -> u64 {
        unsafe { *(self.as_ptr() as *const u64) }
    }
}

impl PartialEq for ChannelLayoutRef {
    fn eq(&self, other: &Self) -> bool {
        self.to_raw() == other.to_raw()
    }
}

impl ToOwned for ChannelLayoutRef {
    type Owned = ChannelLayout;

    fn to_owned(&self) -> Self::Owned {
        ChannelLayout(self.to_raw())
    }
}

/// Channel layout.
#[derive(Clone)]
pub struct ChannelLayout(u64);

impl ChannelLayout {
    /// Get default channel layout for a given number of channels.
    pub fn from_channels(channels: u32) -> Option<Self> {
        let layout = unsafe { ffw_get_default_channel_layout(channels as _) };

        if layout == 0 {
            None
        } else {
            Some(Self(layout))
        }
    }
}

impl AsRef<ChannelLayoutRef> for ChannelLayout {
    fn as_ref(&self) -> &ChannelLayoutRef {
        let Self(raw) = self;

        unsafe { ChannelLayoutRef::from_raw_ptr(raw as *const u64 as *const _) }
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

impl FromStr for ChannelLayout {
    type Err = UnknownChannelLayout;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let name = CString::new(s).expect("invalid channel layout name");

        let layout = unsafe { ffw_get_channel_layout_by_name(name.as_ptr() as _) };

        if layout == 0 {
            Err(UnknownChannelLayout)
        } else {
            Ok(Self(layout))
        }
    }
}
