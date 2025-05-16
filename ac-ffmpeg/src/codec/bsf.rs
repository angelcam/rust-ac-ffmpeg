//! Bitstream filter.

use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
    ptr,
};

use crate::{codec::CodecParameters, packet::Packet, time::TimeBase, Error};

extern "C" {
    fn ffw_bsf_new(name: *const c_char, context: *mut *mut c_void) -> c_int;
    fn ffw_bsf_set_input_codec_parameters(context: *mut c_void, params: *const c_void) -> c_int;
    fn ffw_bsf_set_output_codec_parameters(context: *mut c_void, params: *const c_void) -> c_int;
    fn ffw_bsf_init(
        context: *mut c_void,
        itb_num: c_int,
        itb_den: c_int,
        otb_num: c_int,
        otb_den: c_int,
    ) -> c_int;
    fn ffw_bsf_push(context: *mut c_void, packet: *mut c_void) -> c_int;
    fn ffw_bsf_flush(context: *mut c_void) -> c_int;
    fn ffw_bsf_take(context: *mut c_void, packet: *mut *mut c_void) -> c_int;
    fn ffw_bsf_free(context: *mut c_void);
}

/// A builder for bitstream filters.
pub struct BitstreamFilterBuilder {
    ptr: *mut c_void,

    input_time_base: TimeBase,
    output_time_base: TimeBase,
}

impl BitstreamFilterBuilder {
    /// Create a new bitstream filter builder for a given filter.
    fn new(name: &str) -> Result<Self, Error> {
        let name = CString::new(name).expect("invalid bitstream filter name");

        let mut ptr = ptr::null_mut();

        let ret = unsafe { ffw_bsf_new(name.as_ptr() as _, &mut ptr) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        } else if ptr.is_null() {
            panic!("unable to allocate a bitstream filter");
        }

        let res = BitstreamFilterBuilder {
            ptr,

            input_time_base: TimeBase::MICROSECONDS,
            output_time_base: TimeBase::MICROSECONDS,
        };

        Ok(res)
    }

    /// Set input time base. By default it's in microseconds. All input packets
    /// will be rescaled to this time base before passing them to the filter.
    #[inline]
    pub fn input_time_base(mut self, time_base: TimeBase) -> Self {
        self.input_time_base = time_base;
        self
    }

    /// Set input codec parameters.
    pub fn input_codec_parameters(self, codec_parameters: &CodecParameters) -> Self {
        let ret =
            unsafe { ffw_bsf_set_input_codec_parameters(self.ptr, codec_parameters.as_ptr()) };

        if ret < 0 {
            panic!("unable to set input codec parameters");
        }

        self
    }

    /// Set output time base. By default it's in microseconds. All output
    /// packets will use this time base.
    #[inline]
    pub fn output_time_base(mut self, time_base: TimeBase) -> Self {
        self.output_time_base = time_base;
        self
    }

    /// Set output codec parameters.
    pub fn output_codec_parameters(self, codec_parameters: &CodecParameters) -> Self {
        let ret =
            unsafe { ffw_bsf_set_output_codec_parameters(self.ptr, codec_parameters.as_ptr()) };

        if ret < 0 {
            panic!("unable to set output codec parameters");
        }

        self
    }

    /// Build the bitstream filter.
    pub fn build(mut self) -> Result<BitstreamFilter, Error> {
        let ret = unsafe {
            ffw_bsf_init(
                self.ptr,
                self.input_time_base.num() as _,
                self.input_time_base.den() as _,
                self.output_time_base.num() as _,
                self.output_time_base.den() as _,
            )
        };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = BitstreamFilter {
            ptr,
            output_time_base: self.output_time_base,
        };

        Ok(res)
    }
}

impl Drop for BitstreamFilterBuilder {
    fn drop(&mut self) {
        unsafe { ffw_bsf_free(self.ptr) }
    }
}

unsafe impl Send for BitstreamFilterBuilder {}
unsafe impl Sync for BitstreamFilterBuilder {}

/// A bitstream filter.
///
/// # Filter operation
/// 1. Push a packet to the filter.
/// 2. Take all packets from the filter until you get None.
/// 3. If there are more packets to be processed, continue with 1.
/// 4. Flush the filter.
/// 5. Take all packets from the filter until you get None.
pub struct BitstreamFilter {
    ptr: *mut c_void,
    output_time_base: TimeBase,
}

impl BitstreamFilter {
    /// Get a builder for a given bitstream filter.
    ///
    /// # Example
    /// ```text
    /// ...
    ///
    /// let filter = BitstreamFilter::builder("aac_adtstoasc")?
    ///     .input_codec_parameters(&params)
    ///     .build()?;
    ///
    /// ...
    /// ```
    pub fn builder(name: &str) -> Result<BitstreamFilterBuilder, Error> {
        BitstreamFilterBuilder::new(name)
    }

    /// Push a given packet to the filter.
    pub fn push(&mut self, mut packet: Packet) -> Result<(), Error> {
        let ret = unsafe { ffw_bsf_push(self.ptr, packet.as_mut_ptr()) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        Ok(())
    }

    /// Flush the filter.
    pub fn flush(&mut self) -> Result<(), Error> {
        let ret = unsafe { ffw_bsf_flush(self.ptr) };

        if ret < 0 {
            return Err(Error::from_raw_error_code(ret));
        }

        Ok(())
    }

    /// Take the next packet from the bitstream filter.
    pub fn take(&mut self) -> Result<Option<Packet>, Error> {
        let mut pptr = ptr::null_mut();

        unsafe {
            let ret = ffw_bsf_take(self.ptr, &mut pptr);

            if ret == crate::ffw_error_again || ret == crate::ffw_error_eof {
                Ok(None)
            } else if ret < 0 {
                Err(Error::from_raw_error_code(ret))
            } else if pptr.is_null() {
                panic!("unable to allocate a packet");
            } else {
                Ok(Some(Packet::from_raw_ptr(pptr, self.output_time_base)))
            }
        }
    }
}

impl Drop for BitstreamFilter {
    fn drop(&mut self) {
        unsafe { ffw_bsf_free(self.ptr) }
    }
}

unsafe impl Send for BitstreamFilter {}
unsafe impl Sync for BitstreamFilter {}
