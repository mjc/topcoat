use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use super::common::Buffer;
use crate::LocaleParseError;

/// A region subtag, such as `AT` or `419`: two ASCII letters or three ASCII
/// digits.
///
/// Create one with the [`region!`](crate::region) macro for text known at
/// compile time, or parse one with [`str::parse`]. Parsing canonicalizes the
/// text to upper case.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region(Buffer<3>);

impl Region {
    /// Parses a region subtag, canonicalizing its case.
    ///
    /// # Errors
    ///
    /// Fails when the text is neither two ASCII letters nor three ASCII
    /// digits.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        match Buffer::new(text.as_bytes()) {
            Some(buffer) if buffer.len() == 2 && buffer.is_alphabetic() => {
                Ok(Self(buffer.to_uppercase()))
            }
            Some(buffer) if buffer.len() == 3 && buffer.is_numeric() => Ok(Self(buffer)),
            _ => Err(LocaleParseError::InvalidRegion),
        }
    }

    /// Returns the canonical text of the subtag.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for Region {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Debug for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Region").field(&self.as_str()).finish()
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Region {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

/// Creates a [`Region`] from text known at compile time.
///
/// Malformed text is a compile error. The macro is usable in `const` and
/// `static` items.
#[macro_export]
macro_rules! region {
    ($text:literal) => {
        const {
            match $crate::Region::try_from_str($text) {
                Ok(region) => region,
                Err(_) => panic!(concat!("invalid region subtag: ", $text)),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_letters_and_digits() {
        assert_eq!("at".parse::<Region>().unwrap(), region!("AT"));
        assert_eq!("419".parse::<Region>().unwrap().as_str(), "419");
    }

    #[test]
    fn canonicalizes_letters_to_upper_case() {
        assert_eq!("us".parse::<Region>().unwrap().to_string(), "US");
    }

    #[test]
    fn rejects_mixed_and_wrong_length_text() {
        assert!("A".parse::<Region>().is_err());
        assert!("USA".parse::<Region>().is_err());
        assert!("41".parse::<Region>().is_err());
        assert!("4A9".parse::<Region>().is_err());
    }
}
