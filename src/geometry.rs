use std::fmt;
use std::num::NonZeroU64;

/// Number of internal geometry units in one compatibility-input unit.
pub const SCALE: u64 = 10;

/// Largest integer exactly representable by the compatibility DTO's `f64`.
///
/// Keeping scaled lengths at or below this value makes the one-time conversion
/// to integer geometry lossless.
pub const MAX_EXACT_SCALED_LENGTH: u64 = (1_u64 << f64::MANTISSA_DIGITS) - 1;

/// A positive length expressed entirely in scaled integer units.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Length(NonZeroU64);

impl Length {
    /// Convert a compatibility-input length to exact scaled integer units.
    ///
    /// One decimal place is accepted because [`SCALE`] is ten. Geometry code
    /// uses only the resulting integer and never the source `f64`.
    pub fn from_input_units(value: f64) -> Result<Self, LengthConversionError> {
        if !value.is_finite() {
            return Err(LengthConversionError::NonFinite);
        }
        if value <= 0.0 {
            return Err(LengthConversionError::NonPositive);
        }

        let scaled = value * SCALE as f64;
        if !scaled.is_finite() || scaled > MAX_EXACT_SCALED_LENGTH as f64 {
            return Err(LengthConversionError::OutOfRange);
        }
        if scaled.fract() != 0.0 {
            return Err(LengthConversionError::OverPrecision);
        }

        let scaled = scaled as u64;
        let non_zero = NonZeroU64::new(scaled)
            .expect("a positive input with integral scaled units cannot convert to zero");
        Ok(Self(non_zero))
    }

    /// Return this length in internal scaled units.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Reason a compatibility-input length cannot become exact integer geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LengthConversionError {
    NonFinite,
    NonPositive,
    OverPrecision,
    OutOfRange,
}

impl fmt::Display for LengthConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("must be finite"),
            Self::NonPositive => formatter.write_str("must be greater than zero"),
            Self::OverPrecision => formatter.write_str("must use no more than one decimal place"),
            Self::OutOfRange => formatter.write_str("is too large to convert exactly"),
        }
    }
}

impl std::error::Error for LengthConversionError {}

/// Axis-aligned dimensions expressed in exact scaled integer units.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Dimensions {
    width: Length,
    length: Length,
    height: Length,
}

impl Dimensions {
    #[must_use]
    pub const fn new(width: Length, length: Length, height: Length) -> Self {
        Self {
            width,
            length,
            height,
        }
    }

    #[must_use]
    pub const fn width(self) -> Length {
        self.width
    }

    #[must_use]
    pub const fn length(self) -> Length {
        self.length
    }

    #[must_use]
    pub const fn height(self) -> Length {
        self.height
    }

    /// Compute scaled volume without permitting integer wraparound.
    #[must_use]
    pub fn checked_volume(self) -> Option<u128> {
        u128::from(self.width.get())
            .checked_mul(u128::from(self.length.get()))?
            .checked_mul(u128::from(self.height.get()))
    }
}
