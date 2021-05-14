//! Safe Rust interface for FFmpeg libraries. See the `examples` folder for
//! code examples.

pub mod codec;
pub mod format;
pub mod packet;
pub mod stream;
pub mod time;

use std::{
    ffi::CStr,
    fmt::{self, Display, Formatter},
    io,
    os::raw::{c_char, c_int},
    sync::RwLock,
};

use lazy_static::lazy_static;

lazy_static! {
    /// Log callback.
    static ref LOG_CALLBACK: RwLock<LogCallback> = {
        RwLock::new(LogCallback::new())
    };
}

extern "C" {
    fn ffw_set_log_callback(callback: extern "C" fn(c_int, *const c_char));

    fn ffw_error_again() -> c_int;
    fn ffw_error_eof() -> c_int;
    fn ffw_error_would_block() -> c_int;
    fn ffw_error_unknown() -> c_int;
    fn ffw_error_from_posix(error: c_int) -> c_int;
    fn ffw_error_to_posix(error: c_int) -> c_int;
    fn ffw_error_get_error_string(error: c_int, buffer: *mut c_char, buffer_size: usize);
}

/// A C function passed to the native library as a log callback. The function
/// calls a closure saved in LOG_CALLBACK (if any).
extern "C" fn log_callback(level: c_int, message: *const c_char) {
    let msg = unsafe { CStr::from_ptr(message as _) };

    // level 32 and lower is INFO, WARNING or higher in terms of FFmpeg
    if level <= 32 {
        LOG_CALLBACK
            .read()
            .unwrap()
            .call(level as _, &msg.to_string_lossy());
    }
}

/// Wrapper around a log closure.
struct LogCallback {
    callback: Option<Box<dyn Fn(i32, &str) + Send + Sync>>,
}

impl LogCallback {
    /// Create a new empty log callback.
    fn new() -> LogCallback {
        LogCallback { callback: None }
    }

    /// Store a log callback closure.
    fn set<F>(&mut self, callback: F)
    where
        F: 'static + Fn(i32, &str) + Send + Sync,
    {
        self.callback = Some(Box::new(callback));
    }

    /// Call the stored closure (if any).
    fn call(&self, level: i32, message: &str) {
        if let Some(callback) = self.callback.as_ref() {
            callback(level, message);
        }
    }
}

/// Set log callback for FFmpeg. All log messages from FFmpeg will be passed
/// to a given closure.
pub fn set_log_callback<F>(callback: F)
where
    F: 'static + Fn(i32, &str) + Send + Sync,
{
    LOG_CALLBACK.write().unwrap().set(callback);

    unsafe {
        ffw_set_log_callback(log_callback);
    }
}

/// Error variants.
#[derive(Debug, Clone)]
enum ErrorVariant {
    FFmpeg(c_int),
    Other(String),
}

/// An error.
#[derive(Debug, Clone)]
pub struct Error {
    variant: ErrorVariant,
}

impl Error {
    /// Create a new FFmpeg error.
    pub fn new<T>(msg: T) -> Self
    where
        T: ToString,
    {
        Self {
            variant: ErrorVariant::Other(msg.to_string()),
        }
    }

    /// Convert this error into a standard IO error (if possible).
    pub fn to_io_error(&self) -> Option<io::Error> {
        if let ErrorVariant::FFmpeg(code) = &self.variant {
            let posix = unsafe { ffw_error_to_posix(*code) };
            let err = io::Error::from_raw_os_error(posix as _);

            Some(err)
        } else {
            None
        }
    }

    /// Create a new FFmpeg error from a given FFmpeg error code.
    fn from_raw_error_code(code: c_int) -> Self {
        Self {
            variant: ErrorVariant::FFmpeg(code),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match &self.variant {
            ErrorVariant::FFmpeg(code) => {
                let mut buffer = [0u8; 256];

                let buffer_ptr = buffer.as_mut_ptr();
                let buffer_len = buffer.len();

                let msg = unsafe {
                    ffw_error_get_error_string(*code, buffer_ptr as _, buffer_len as _);

                    CStr::from_ptr(buffer.as_ptr() as _)
                        .to_str()
                        .expect("UTF-8 encoded error string expected")
                };

                write!(f, "{}", msg)
            }
            ErrorVariant::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}
