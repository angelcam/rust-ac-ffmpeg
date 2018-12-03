use std::fmt;
use std::ptr;

use std::error::Error as ErrorTrait;
use std::ffi::CString;
use std::fmt::{Display, Formatter};

use libc::{c_char, c_int, c_void};

use packet::Packet;
use video::frame::{Format, Frame};

extern "C" {
    fn ffw_decoder_new(codec: *const c_char) -> *mut c_void;
    fn ffw_decoder_push_packet(decoder: *mut c_void, packet: *const c_void) -> c_int;
    fn ffw_decoder_take_frame(decoder: *mut c_void, frame: *mut *mut c_void) -> c_int;
    fn ffw_decoder_free(decoder: *mut c_void);

    fn ffw_encoder_new(codec: *const c_char) -> *mut c_void;
    fn ffw_encoder_set_bit_rate(encoder: *mut c_void, bit_rate: c_int);
    fn ffw_encoder_set_pixel_format(encoder: *mut c_void, format: c_int);
    fn ffw_encoder_set_width(encoder: *mut c_void, width: c_int);
    fn ffw_encoder_set_height(encoder: *mut c_void, height: c_int);
    fn ffw_encoder_set_time_base(encoder: *mut c_void, num: c_int, den: c_int);
    fn ffw_encoder_open(encoder: *mut c_void) -> c_int;
    fn ffw_encoder_push_frame(encoder: *mut c_void, frame: *const c_void) -> c_int;
    fn ffw_encoder_take_packet(encoder: *mut c_void, packet: *mut *mut c_void) -> c_int;
    fn ffw_encoder_free(encoder: *mut c_void);
}

/// A decoding or encoding error.
#[derive(Debug, Clone)]
pub struct Error {
    msg: String,
    again: bool,
}

impl Error {
    /// Create a new error.
    pub fn new<T>(msg: T) -> Error
    where
        T: ToString,
    {
        Error {
            msg: msg.to_string(),
            again: false,
        }
    }

    /// Create a new "again" error. This error indicates that another operation
    /// needs to be done before continuing with the current operation.
    pub fn again<T>(msg: T) -> Error
    where
        T: ToString,
    {
        Error {
            msg: msg.to_string(),
            again: true,
        }
    }

    /// Check if there is another operation that needs to be done before
    /// continuing with the current operation.
    pub fn is_again(&self) -> bool {
        self.again
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str(&self.msg)
    }
}

impl ErrorTrait for Error {
    fn description(&self) -> &str {
        &self.msg
    }
}

/// Video decoder.
///
/// # Decoder operation
/// 1. Push a packet to the decoder.
/// 2. Take all frames from the decoder until you get None.
/// 3. If there are more packets to be decoded, continue with 1.
/// 4. Flush the decoder.
/// 5. Take all frames from the decoder until you get None.
pub struct Decoder {
    ptr: *mut c_void,
}

impl Decoder {
    /// Create a new video decoder for a given codec.
    pub fn new(codec: &str) -> Result<Decoder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_decoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        let res = Decoder { ptr: ptr };

        Ok(res)
    }

    /// Push a given packet to the decoder.
    pub fn push(&mut self, packet: &Packet) -> Result<(), Error> {
        unsafe {
            match ffw_decoder_push_packet(self.ptr, packet.as_ptr()) {
                1 => Ok(()),
                0 => Err(Error::again(
                    "all frames must be consumed before pushing a new packet",
                )),
                _ => Err(Error::new("decoding error")),
            }
        }
    }

    /// Flush the decoder.
    pub fn flush(&mut self) -> Result<(), Error> {
        unsafe {
            match ffw_decoder_push_packet(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(Error::again("all frames must be consumed before flushing")),
                _ => Err(Error::new("decoding error")),
            }
        }
    }

    /// Take the next decoded frame from the decoder.
    pub fn take(&mut self) -> Result<Option<Frame>, Error> {
        let mut fptr = ptr::null_mut();

        unsafe {
            match ffw_decoder_take_frame(self.ptr, &mut fptr) {
                1 => {
                    if fptr.is_null() {
                        panic!("no frame received")
                    } else {
                        Ok(Some(Frame::from_raw_ptr(fptr)))
                    }
                }
                0 => Ok(None),
                _ => Err(Error::new("decoding error")),
            }
        }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { ffw_decoder_free(self.ptr) }
    }
}

unsafe impl Send for Decoder {}
unsafe impl Sync for Decoder {}

/// Builder for the video encoder.
pub struct EncoderBuilder {
    ptr: *mut c_void,

    format: Option<Format>,
    width: Option<usize>,
    height: Option<usize>,
}

impl EncoderBuilder {
    /// Create a new encoder builder for a given codec.
    fn new(codec: &str) -> Result<EncoderBuilder, Error> {
        let codec = CString::new(codec).expect("invalid codec name");

        let ptr = unsafe { ffw_encoder_new(codec.as_ptr() as _) };

        if ptr.is_null() {
            return Err(Error::new("unknown codec"));
        }

        unsafe {
            ffw_encoder_set_bit_rate(ptr, 0);
            ffw_encoder_set_time_base(ptr, 1, 1000);
        }

        let res = EncoderBuilder {
            ptr: ptr,

            format: None,
            width: None,
            height: None,
        };

        Ok(res)
    }

    /// Set encoder bit rate. The default is 0 (i.e. automatic).
    pub fn bit_rate(self, bit_rate: usize) -> EncoderBuilder {
        unsafe {
            ffw_encoder_set_bit_rate(self.ptr, bit_rate as _);
        }

        self
    }

    /// Set encoder time base as a rational number. The default is 1/1000.
    pub fn time_base(self, num: u32, den: u32) -> EncoderBuilder {
        unsafe {
            ffw_encoder_set_time_base(self.ptr, num as _, den as _);
        }

        self
    }

    /// Set encoder pixel format.
    pub fn pixel_format(mut self, format: Format) -> EncoderBuilder {
        self.format = Some(format);
        self
    }

    /// Set frame width.
    pub fn width(mut self, width: usize) -> EncoderBuilder {
        self.width = Some(width);
        self
    }

    /// Set frame height.
    pub fn height(mut self, height: usize) -> EncoderBuilder {
        self.height = Some(height);
        self
    }

    /// Build the encoder.
    pub fn build(mut self) -> Result<Encoder, Error> {
        let format = self.format.ok_or(Error::new("pixel format not set"))?;
        let width = self.width.ok_or(Error::new("width not set"))?;
        let height = self.height.ok_or(Error::new("height not set"))?;

        unsafe {
            ffw_encoder_set_pixel_format(self.ptr, format);
            ffw_encoder_set_width(self.ptr, width as _);
            ffw_encoder_set_height(self.ptr, height as _);

            if ffw_encoder_open(self.ptr) != 0 {
                return Err(Error::new("unable to build the encoder"));
            }
        }

        let ptr = self.ptr;

        self.ptr = ptr::null_mut();

        let res = Encoder { ptr: ptr };

        Ok(res)
    }
}

impl Drop for EncoderBuilder {
    fn drop(&mut self) {
        unsafe { ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for EncoderBuilder {}
unsafe impl Sync for EncoderBuilder {}

/// Video encoder.
///
/// # Encoder operation
/// 1. Push a frame to the encoder.
/// 2. Take all packets from the encoder until you get None.
/// 3. If there are more frames to be encoded, continue with 1.
/// 4. Flush the encoder.
/// 5. Take all packets from the encoder until you get None.
pub struct Encoder {
    ptr: *mut c_void,
}

impl Encoder {
    /// Get encoder builder for a given codec.
    pub fn builder(codec: &str) -> Result<EncoderBuilder, Error> {
        EncoderBuilder::new(codec)
    }

    /// Push a given frame to the encoder.
    pub fn push(&mut self, frame: &Frame) -> Result<(), Error> {
        unsafe {
            match ffw_encoder_push_frame(self.ptr, frame.as_ptr()) {
                1 => Ok(()),
                0 => Err(Error::again(
                    "all packets must be consumed before pushing a new frame",
                )),
                _ => Err(Error::new("encoding error")),
            }
        }
    }

    /// Flush the encoder.
    pub fn flush(&mut self) -> Result<(), Error> {
        unsafe {
            match ffw_encoder_push_frame(self.ptr, ptr::null()) {
                1 => Ok(()),
                0 => Err(Error::again("all packets must be consumed before flushing")),
                _ => Err(Error::new("encoding error")),
            }
        }
    }

    /// Take the next packet from the encoder.
    pub fn take(&mut self) -> Result<Option<Packet>, Error> {
        let mut pptr = ptr::null_mut();

        unsafe {
            match ffw_encoder_take_packet(self.ptr, &mut pptr) {
                1 => {
                    if pptr.is_null() {
                        panic!("no packet received")
                    } else {
                        Ok(Some(Packet::from_raw_ptr(pptr)))
                    }
                }
                0 => Ok(None),
                _ => Err(Error::new("encoding error")),
            }
        }
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { ffw_encoder_free(self.ptr) }
    }
}

unsafe impl Send for Encoder {}
unsafe impl Sync for Encoder {}
