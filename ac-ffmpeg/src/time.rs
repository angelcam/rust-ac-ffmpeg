//! Time base aware timestamps.

use std::{
    cmp::{Eq, Ordering, PartialEq, PartialOrd},
    fmt::{self, Debug, Formatter},
    ops::{Add, AddAssign, Sub, SubAssign},
    time::Duration,
};

extern "C" {
    static ffw_null_timestamp: i64;

    fn ffw_rescale_q(n: i64, aq_num: u32, aq_den: u32, bq_num: u32, bq_den: u32) -> i64;
}

/// A rational time base (e.g. 1/1000 is a millisecond time base).
#[derive(Copy, Clone)]
pub struct TimeBase {
    num: u32,
    den: u32,
}

impl TimeBase {
    /// A microseconds time base.
    pub const MICROSECONDS: TimeBase = TimeBase::new(1, 1_000_000);

    /// Create a new time base as a rational number with a given numerator and
    /// denominator.
    #[inline]
    pub const fn new(num: u32, den: u32) -> Self {
        Self { num, den }
    }

    /// Get the numerator.
    #[inline]
    pub const fn num(&self) -> u32 {
        self.num
    }

    /// Get the denominator.
    #[inline]
    pub const fn den(&self) -> u32 {
        self.den
    }
}

impl Debug for TimeBase {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}/{}", self.num(), self.den())
    }
}

/// A timestamp supporting various time bases. All comparisons are done within
/// microsecond time base.
#[derive(Copy, Clone)]
pub struct Timestamp {
    timestamp: i64,
    time_base: TimeBase,
}

impl Timestamp {
    /// Create a "null" timestamp (i.e. a timestamp set to the AV_NOPTS_VALUE).
    #[inline]
    pub fn null() -> Self {
        unsafe {
            Self {
                timestamp: ffw_null_timestamp,
                time_base: TimeBase::MICROSECONDS,
            }
        }
    }

    /// Create a new timestamp with a given time base.
    #[inline]
    pub const fn new(timestamp: i64, time_base: TimeBase) -> Self {
        Self {
            timestamp,
            time_base,
        }
    }

    /// Create a new timestamp with 1/1 time base.
    #[inline]
    pub const fn from_secs(timestamp: i64) -> Self {
        Self::new(timestamp, TimeBase::new(1, 1))
    }

    /// Create a new timestamp with 1/1_000 time base.
    #[inline]
    pub const fn from_millis(timestamp: i64) -> Self {
        Self::new(timestamp, TimeBase::new(1, 1_000))
    }

    /// Create a new timestamp with 1/1_000_000 time base.
    #[inline]
    pub const fn from_micros(timestamp: i64) -> Self {
        Self::new(timestamp, TimeBase::new(1, 1_000_000))
    }

    /// Create a new timestamp with 1/1_000_000_000 time base.
    #[inline]
    pub const fn from_nanos(timestamp: i64) -> Self {
        Self::new(timestamp, TimeBase::new(1, 1_000_000_000))
    }

    /// Get the time base.
    #[inline]
    pub const fn time_base(&self) -> TimeBase {
        self.time_base
    }

    /// Get the raw timestamp value.
    #[inline]
    pub const fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// Set the timestamp with the current time base.
    #[inline]
    pub const fn with_raw_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Check if this is the "null" timestamp (i.e. it is equal to the
    /// AV_NOPTS_VALUE).
    #[inline]
    pub fn is_null(&self) -> bool {
        unsafe { self.timestamp == ffw_null_timestamp }
    }

    /// Rescale the timestamp value to a given time base.
    pub fn with_time_base(&self, time_base: TimeBase) -> Self {
        let timestamp = if self.is_null() {
            self.timestamp
        } else {
            unsafe {
                ffw_rescale_q(
                    self.timestamp,
                    self.time_base.num,
                    self.time_base.den,
                    time_base.num,
                    time_base.den,
                )
            }
        };

        Self {
            timestamp,
            time_base,
        }
    }

    /// Get the timestamp value in seconds.
    pub fn as_secs(&self) -> Option<i64> {
        if self.is_null() {
            None
        } else {
            let ts = self.with_time_base(TimeBase::new(1, 1));

            Some(ts.timestamp)
        }
    }

    /// Get the timestamp value in milliseconds.
    pub fn as_millis(&self) -> Option<i64> {
        if self.is_null() {
            None
        } else {
            let ts = self.with_time_base(TimeBase::new(1, 1_000));

            Some(ts.timestamp)
        }
    }

    /// Get the timestamp value in microseconds.
    pub fn as_micros(&self) -> Option<i64> {
        if self.is_null() {
            None
        } else {
            let ts = self.with_time_base(TimeBase::new(1, 1_000_000));

            Some(ts.timestamp)
        }
    }

    /// Get the timestamp value in nanoseconds.
    pub fn as_nanos(&self) -> Option<i64> {
        if self.is_null() {
            None
        } else {
            let ts = self.with_time_base(TimeBase::new(1, 1_000_000_000));

            Some(ts.timestamp)
        }
    }

    /// Get the timestamp value as a floating point number with 32-bit
    /// precision.
    pub fn as_f32(&self) -> Option<f32> {
        if self.is_null() {
            None
        } else {
            Some(self.timestamp as f32 * self.time_base.num as f32 / self.time_base.den as f32)
        }
    }

    /// Get the timestamp value as a floating point number with 64-bit
    /// precision.
    pub fn as_f64(&self) -> Option<f64> {
        if self.is_null() {
            None
        } else {
            Some(self.timestamp as f64 * self.time_base.num as f64 / self.time_base.den as f64)
        }
    }
}

impl Debug for Timestamp {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        if let Some(millis) = self.as_millis() {
            write!(f, "{}.{:03}s", millis / 1_000, millis % 1_000)
        } else {
            write!(f, "(null)")
        }
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(mut self, rhs: Duration) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<Duration> for Timestamp {
    fn add_assign(&mut self, rhs: Duration) {
        // do not add anything to null timestamps
        if self.is_null() {
            return;
        }

        self.timestamp += Self::from_secs(rhs.as_secs() as i64)
            .with_time_base(self.time_base)
            .timestamp();

        self.timestamp += Self::from_nanos(rhs.subsec_nanos() as i64)
            .with_time_base(self.time_base)
            .timestamp();
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(mut self, rhs: Duration) -> Self::Output {
        self -= rhs;
        self
    }
}

impl SubAssign<Duration> for Timestamp {
    fn sub_assign(&mut self, rhs: Duration) {
        // do not subtract anything from null timestamps
        if self.is_null() {
            return;
        }

        self.timestamp -= Self::from_secs(rhs.as_secs() as i64)
            .with_time_base(self.time_base)
            .timestamp();

        self.timestamp -= Self::from_nanos(rhs.subsec_nanos() as i64)
            .with_time_base(self.time_base)
            .timestamp();
    }
}

impl Sub for Timestamp {
    type Output = Duration;

    fn sub(mut self, rhs: Self) -> Self::Output {
        assert!(!self.is_null());
        assert!(!rhs.is_null());

        let rhs = rhs.with_time_base(self.time_base);

        self.timestamp -= rhs.timestamp;

        if self.timestamp < 0 {
            panic!("out of range");
        }

        let secs = self.with_time_base(TimeBase::new(1, 1));

        // calculate the remainder
        self.timestamp -= secs.with_time_base(self.time_base).timestamp();

        let nanos = self.as_nanos().unwrap();

        Duration::new(secs.timestamp as u64, nanos as u32)
    }
}

impl PartialEq for Timestamp {
    fn eq(&self, other: &Timestamp) -> bool {
        let a = self.as_micros();
        let b = other.as_micros();

        a == b
    }
}

impl Eq for Timestamp {}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Timestamp) -> Option<Ordering> {
        if let Some(a) = self.as_micros() {
            if let Some(b) = other.as_micros() {
                return a.partial_cmp(&b);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{TimeBase, Timestamp};

    #[test]
    fn test_duration_add() {
        let mut ts = Timestamp::new(333, TimeBase::new(1, 90_000));

        ts += Duration::from_millis(100);

        assert_eq!(ts.timestamp, 9333);
    }

    #[test]
    fn test_duration_sub() {
        let mut ts = Timestamp::new(333, TimeBase::new(1, 90_000));

        ts -= Duration::from_millis(50);

        assert_eq!(ts.timestamp, -4167);
    }

    #[test]
    fn test_timestamp_sub() {
        let a = Timestamp::new(333, TimeBase::new(1, 90_000));
        let b = Timestamp::new(79, TimeBase::new(1, 50_000));

        let delta = a - b;

        assert_eq!(delta.as_secs(), 0);
        assert_eq!(delta.subsec_nanos(), 2_122_222);
    }

    #[test]
    fn test_comparisons() {
        let a = Timestamp::from_secs(1);
        let b = Timestamp::from_millis(1_000);

        assert_eq!(a, b);

        let a = Timestamp::from_secs(1);
        let b = Timestamp::from_millis(1_001);

        assert_ne!(a, b);
        assert!(a < b);

        let a = Timestamp::from_secs(1);
        let b = Timestamp::from_micros(1_000_001);

        assert_ne!(a, b);
        assert!(a < b);

        // this is outside of the comparison scale
        let a = Timestamp::from_secs(1);
        let b = Timestamp::from_nanos(1_000_000_001);

        assert_eq!(a, b);
    }
}
