use libc::{c_int, c_void, size_t};

use Error;

use video::frame::{Format, Frame};

const ALG_ID_FAST_BILINEAR: usize = 0;
const ALG_ID_BILINEAR: usize = 1;
const ALG_ID_BICUBIC: usize = 2;

extern "C" {
    fn ffw_frame_scaler_new(
        sformat: c_int,
        swidth: c_int,
        sheight: c_int,
        tformat: c_int,
        twidth: c_int,
        theight: c_int,
        flags: c_int,
    ) -> *mut c_void;

    fn ffw_frame_scaler_scale(scaler: *mut c_void, src: *const c_void) -> *mut c_void;

    fn ffw_frame_scaler_free(scaler: *mut c_void);

    fn ffw_alg_id_to_flags(id: size_t) -> c_int;
}

/// Scaling algorithm.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Algorithm {
    FastBilinear,
    Bilinear,
    Bicubic,
}

impl Algorithm {
    /// Get algorithm ID.
    fn id(&self) -> usize {
        match self {
            &Algorithm::FastBilinear => ALG_ID_FAST_BILINEAR,
            &Algorithm::Bilinear => ALG_ID_BILINEAR,
            &Algorithm::Bicubic => ALG_ID_BICUBIC,
        }
    }
}

/// Builder for a video frame scaler.
pub struct FrameScalerBuilder {
    sformat: c_int,
    swidth: c_int,
    sheight: c_int,

    tformat: Option<c_int>,
    twidth: c_int,
    theight: c_int,

    flags: c_int,
}

impl FrameScalerBuilder {
    /// Create a new video frame scaler builder.
    fn new() -> FrameScalerBuilder {
        let default_algorithm = Algorithm::Bicubic;

        let flags = unsafe { ffw_alg_id_to_flags(default_algorithm.id()) };

        FrameScalerBuilder {
            sformat: -1,
            swidth: 0,
            sheight: 0,

            tformat: None,
            twidth: 0,
            theight: 0,

            flags: flags,
        }
    }

    /// Set source pixel format.
    pub fn source_format(mut self, format: Format) -> FrameScalerBuilder {
        self.sformat = format;
        self
    }

    /// Set source frame width.
    pub fn source_width(mut self, width: usize) -> FrameScalerBuilder {
        self.swidth = width as _;
        self
    }

    /// Set source frame height.
    pub fn source_height(mut self, height: usize) -> FrameScalerBuilder {
        self.sheight = height as _;
        self
    }

    /// Set target pixel format. The default is equal to the source format.
    pub fn target_format(mut self, format: Format) -> FrameScalerBuilder {
        self.tformat = Some(format);
        self
    }

    /// Set target frame width.
    pub fn target_width(mut self, width: usize) -> FrameScalerBuilder {
        self.twidth = width as _;
        self
    }

    /// Set target frame height.
    pub fn target_height(mut self, height: usize) -> FrameScalerBuilder {
        self.theight = height as _;
        self
    }

    /// Set scaling algorithm. The default is bicubic.
    pub fn algorithm(mut self, algorithm: Algorithm) -> FrameScalerBuilder {
        self.flags = unsafe { ffw_alg_id_to_flags(algorithm.id()) };

        self
    }

    /// Build the video frame scaler.
    pub fn build(self) -> Result<FrameScaler, Error> {
        let tformat = self.tformat.unwrap_or(self.sformat);

        if self.sformat < 0 {
            return Err(Error::new("invalid source format"));
        } else if tformat < 0 {
            return Err(Error::new("invalid target format"));
        } else if self.swidth < 1 {
            return Err(Error::new("invalid source width"));
        } else if self.sheight < 1 {
            return Err(Error::new("invalid source height"));
        } else if self.twidth < 1 {
            return Err(Error::new("invalid target width"));
        } else if self.theight < 1 {
            return Err(Error::new("invalid target height"));
        }

        let ptr = unsafe {
            ffw_frame_scaler_new(
                self.sformat,
                self.swidth,
                self.sheight,
                tformat,
                self.twidth,
                self.theight,
                self.flags,
            )
        };

        if ptr.is_null() {
            return Err(Error::new("unable to create a frame scaler"));
        }

        let res = FrameScaler {
            ptr: ptr,

            sformat: self.sformat,
            swidth: self.swidth as _,
            sheight: self.sheight as _,
        };

        Ok(res)
    }
}

/// Video frame scaler.
pub struct FrameScaler {
    ptr: *mut c_void,

    sformat: Format,
    swidth: usize,
    sheight: usize,
}

impl FrameScaler {
    /// Get a frame scaler builder.
    pub fn builder() -> FrameScalerBuilder {
        FrameScalerBuilder::new()
    }

    /// Scale a given frame.
    pub fn scale(&mut self, frame: &Frame) -> Result<Frame, Error> {
        if self.swidth != frame.width() {
            return Err(Error::new("frame width does not match"));
        } else if self.sheight != frame.height() {
            return Err(Error::new("frame height does not match"));
        } else if self.sformat != frame.format() {
            return Err(Error::new("frame pixel format does not match"));
        }

        let res = unsafe { ffw_frame_scaler_scale(self.ptr, frame.as_ptr()) };

        if res.is_null() {
            panic!("unable to scale a frame");
        }

        let frame = unsafe { Frame::from_raw_ptr(res) };

        Ok(frame)
    }
}

impl Drop for FrameScaler {
    fn drop(&mut self) {
        unsafe { ffw_frame_scaler_free(self.ptr) }
    }
}

unsafe impl Send for FrameScaler {}
unsafe impl Sync for FrameScaler {}
