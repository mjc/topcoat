use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use super::common::Buffer;
use crate::LocaleParseError;

/// A variant subtag, such as `posix` or `1996`: five to eight ASCII
/// alphanumerics, or a digit followed by three alphanumerics.
///
/// Create one with the [`variant!`](crate::variant) macro for text known at
/// compile time, or parse one with [`str::parse`]. Parsing canonicalizes the
/// text to lower case.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant(Buffer<8>);

impl Variant {
    /// Parses a variant subtag, canonicalizing its case.
    ///
    /// # Errors
    ///
    /// Fails when the text is neither five to eight ASCII alphanumerics nor
    /// a digit followed by three alphanumerics.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        match Buffer::new(text.as_bytes()) {
            Some(buffer) if buffer.len() >= 5 => Ok(Self(buffer.to_lowercase())),
            Some(buffer) if buffer.len() == 4 && buffer.starts_with_digit() => {
                Ok(Self(buffer.to_lowercase()))
            }
            _ => Err(LocaleParseError::InvalidVariant),
        }
    }

    /// Returns the canonical text of the subtag.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts to the `icu_locale_core` subtag.
    #[must_use]
    pub const fn to_icu(self) -> icu_locale_core::subtags::Variant {
        // Both types accept the same canonical text.
        match icu_locale_core::subtags::Variant::try_from_raw(self.0.into_raw()) {
            Ok(variant) => variant,
            Err(_) => unreachable!(),
        }
    }

    /// Converts from the `icu_locale_core` subtag.
    #[must_use]
    pub const fn from_icu(variant: icu_locale_core::subtags::Variant) -> Self {
        Self(Buffer::from_raw(variant.into_raw()))
    }

    /// Compares two variants like `Ord`, for use in const context.
    pub(crate) const fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.0.compare(&other.0)
    }
}

impl AsRef<str> for Variant {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Variant").field(&self.as_str()).finish()
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Variant {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

/// Creates a [`Variant`] from text known at compile time.
///
/// Malformed text is a compile error. The macro is usable in `const` and
/// `static` items.
#[macro_export]
macro_rules! variant {
    ($text:literal) => {
        const {
            match $crate::Variant::try_from_str($text) {
                Ok(variant) => variant,
                Err(_) => panic!(concat!("invalid variant subtag: ", $text)),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_both_forms() {
        assert_eq!("posix".parse::<Variant>().unwrap(), variant!("posix"));
        assert_eq!("1996".parse::<Variant>().unwrap().as_str(), "1996");
        assert_eq!("1aaa".parse::<Variant>().unwrap().as_str(), "1aaa");
        assert_eq!("valencia".parse::<Variant>().unwrap().as_str(), "valencia");
    }

    #[test]
    fn canonicalizes_to_lower_case() {
        assert_eq!("POSIX".parse::<Variant>().unwrap().to_string(), "posix");
    }

    #[test]
    fn rejects_short_letter_only_text_and_overlong_text() {
        assert!("abcd".parse::<Variant>().is_err());
        assert!("abc".parse::<Variant>().is_err());
        assert!("123".parse::<Variant>().is_err());
        assert!("valencian".parse::<Variant>().is_err());
    }
}
