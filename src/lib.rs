pub mod codec;
pub mod format;
pub mod packet;

use std::fmt;

use std::error::Error as ErrorTrait;
use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use std::sync::RwLock;

use lazy_static::lazy_static;

use libc::{c_char, c_int};

lazy_static! {
    /// Log callback.
    static ref LOG_CALLBACK: RwLock<LogCallback> = {
        RwLock::new(LogCallback::new())
    };
}

extern "C" {
    fn ffw_set_log_callback(callback: extern "C" fn(c_int, *const c_char));
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
    callback: Option<Box<Fn(i32, &str) + Send + Sync>>,
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

/// An FFmpeg error.
#[derive(Debug, Clone)]
pub struct Error {
    msg: String,
}

impl Error {
    /// Create a new FFmpeg error.
    pub fn new<T>(msg: T) -> Error
    where
        T: ToString,
    {
        Error {
            msg: msg.to_string(),
        }
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
