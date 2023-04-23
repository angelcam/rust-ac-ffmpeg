use std::{
    ffi::CString,
    os::raw::{c_char, c_int, c_void},
    ptr,
};

use crate::{packet::Packet, time::TimeBase, Error};

use super::SubtitleCodecParameters;

extern "C" {
    fn ffw_subtitle_transcoder_new(
        in_codec: *const c_char,
        out_codec: *const c_char,
    ) -> *mut c_void;
    fn ffw_subtitle_transcoder_from_codec_parameters(
        in_params: *const c_void,
        out_params: *const c_void,
    ) -> *mut c_void;
    fn ffw_subtitle_decoder_set_initial_option(
        transcoder: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_subtitle_encoder_set_initial_option(
        transcoder: *mut c_void,
        key: *const c_char,
        value: *const c_char,
    ) -> c_int;
    fn ffw_subtitle_transcoder_open(
        transcoder: *mut c_void,
        in_tb_num: c_int,
        in_tb_den: c_int,
        out_tb_num: c_int,
        out_tb_den: c_int,
    ) -> c_int;
    fn ffw_subtitle_transcoder_push_packet(transcoder: *mut c_void, packet: *mut c_void) -> c_int;
    fn ffw_subtitle_transcoder_take_packet(
        transcoder: *mut c_void,
        packet: *mut *mut c_void,
    ) -> c_int;
    fn ffw_subtitle_transcoder_free(transcoder: *mut c_void);
}

/// A builder for subtitle transcoders.

pub struct SubtitleTranscoderBuilder {
    ptr: *mut c_void,

    input_time_base: TimeBase,
    output_time_base: TimeBase,
}

impl SubtitleTranscoderBuilder {
    /// Create a new builder.
    fn new(in_codec: &str, out_codec: &str) -> Result<Self, Error> {
        let in_codec = CString::new(in_codec).expect("invalid codec name");
        let out_codec = CString::new(out_codec).expect("invalid codec name");

        let ptr =
            unsafe { ffw_subtitle_transcoder_new(in_codec.as_ptr() as _, out_codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unable to create transcoder"));
        }

        Ok(SubtitleTranscoderBuilder {
            ptr,
            input_time_base: TimeBase::MICROSECONDS,
            output_time_base: TimeBase::MICROSECONDS,
        })
    }

    /// Create a new builder from given codec parameters.
    fn from_codec_parameters(
        in_params: &SubtitleCodecParameters,
        out_params: &SubtitleCodecParameters,
    ) -> Result<Self, Error> {
        let ptr = unsafe {
            ffw_subtitle_transcoder_from_codec_parameters(in_params.as_ptr(), out_params.as_ptr())
        };

        if ptr.is_null() {
            return Err(Error::new("unable to create transcoder"));
        }

        Ok(SubtitleTranscoderBuilder {
            ptr,
            input_time_base: TimeBase::MICROSECONDS,
            output_time_base: TimeBase::MICROSECONDS,
        })
    }

    /// Set a decoder option.
    pub fn set_decoder_option<V>(self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            ffw_subtitle_decoder_set_initial_option(
                self.ptr,
                name.as_ptr() as _,
                value.as_ptr() as _,
            )
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Set an encoder option.
    pub fn set_encoder_option<V>(self, name: &str, value: V) -> Self
    where
        V: ToString,
    {
        let name = CString::new(name).expect("invalid option name");
        let value = CString::new(value.to_string()).expect("invalid option value");

        let ret = unsafe {
            ffw_subtitle_encoder_set_initial_option(
                self.ptr,
                name.as_ptr() as _,
                value.as_ptr() as _,
            )
        };

        if ret < 0 {
            panic!("unable to allocate an option");
        }

        self
    }

    /// Set input time base. By default it's in microseconds.
    pub fn input_time_base(mut self, time_base: TimeBase) -> Self {
        self.output_time_base = time_base;
        self
    }

    /// Set output time base. By default it's in microseconds. All output
    /// packets will use this time base.
    pub fn output_time_base(mut self, time_base: TimeBase) -> Self {
        self.output_time_base = time_base;
        self
    }

    /// Build the transcoder.
    pub fn build(mut self) -> Result<SubtitleTranscoder, Error> {
        unsafe {
            let ret = ffw_subtitle_transcoder_open(
                self.ptr,
                self.input_time_base.num() as _,
                self.input_time_base.den() as _,
                self.output_time_base.num() as _,
                self.output_time_base.den() as _,
            );
            if ret != 0 {
                return Err(Error::from_raw_error_code(ret));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = SubtitleTranscoder {
            ptr,
            output_time_base: self.output_time_base,
        };

        Ok(res)
    }
}

impl Drop for SubtitleTranscoderBuilder {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffw_subtitle_transcoder_free(self.ptr) }
        }
    }
}

unsafe impl Send for SubtitleTranscoderBuilder {}
unsafe impl Sync for SubtitleTranscoderBuilder {}

pub struct SubtitleTranscoder {
    ptr: *mut c_void,

    output_time_base: TimeBase,
}

impl SubtitleTranscoder {
    /// Create a new video decoder for a given codec.
    pub fn new(in_codec: &str, out_codec: &str) -> Result<Self, Error> {
        SubtitleTranscoderBuilder::new(in_codec, out_codec).and_then(|builder| builder.build())
    }

    /// Create a new video decoder builder from given codec parameters.
    pub fn from_codec_parameters(
        in_params: &SubtitleCodecParameters,
        out_params: &SubtitleCodecParameters,
    ) -> Result<SubtitleTranscoderBuilder, Error> {
        SubtitleTranscoderBuilder::from_codec_parameters(in_params, out_params)
    }

    /// Get decoder builder for a given codec.
    pub fn builder(in_codec: &str, out_codec: &str) -> Result<SubtitleTranscoderBuilder, Error> {
        SubtitleTranscoderBuilder::new(in_codec, out_codec)
    }

    /// Push a given packet to the transcoder.
    pub fn push(&mut self, mut packet: Packet) -> Result<(), Error> {
        unsafe {
            let ret = ffw_subtitle_transcoder_push_packet(self.ptr, packet.as_mut_ptr());

            if !(ret == crate::ffw_error_again()) && ret < 0 {
                return Err(Error::from_raw_error_code(ret));
            }
        }

        return Ok(());
    }

    /// Take the next packet from the transcoder.
    pub fn take(&mut self) -> Result<Option<Packet>, Error> {
        let mut pptr = ptr::null_mut();

        unsafe {
            let ret = ffw_subtitle_transcoder_take_packet(self.ptr, &mut pptr);

            if ret == crate::ffw_error_again() || ret == crate::ffw_error_eof() {
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

// impl Drop for SubtitleTranscoder {
//     fn drop(&mut self) {
//         unsafe { ffw_subtitle_transcoder_free(self.ptr) }
//     }
// }

unsafe impl Send for SubtitleTranscoder {}
unsafe impl Sync for SubtitleTranscoder {}
