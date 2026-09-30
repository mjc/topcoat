use std::fmt::{self, Display};

/// The reason a locale, language identifier, or subtag failed to parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocaleParseError {
    /// The language subtag is missing or malformed.
    InvalidLanguage,
    /// A script subtag is malformed.
    InvalidScript,
    /// A region subtag is malformed.
    InvalidRegion,
    /// A variant subtag is malformed.
    InvalidVariant,
    /// The same variant appears twice.
    DuplicateVariant,
    /// More variants than a language identifier can hold. See
    /// [`MAX_VARIANTS`](crate::MAX_VARIANTS).
    TooManyVariants,
    /// A subtag fits none of the positions in a language identifier.
    InvalidSubtag,
    /// The extensions after the language identifier are malformed.
    InvalidExtension,
}

impl Display for LocaleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLanguage => "invalid language subtag",
            Self::InvalidScript => "invalid script subtag",
            Self::InvalidRegion => "invalid region subtag",
            Self::InvalidVariant => "invalid variant subtag",
            Self::DuplicateVariant => "duplicate variant subtag",
            Self::TooManyVariants => "too many variant subtags",
            Self::InvalidSubtag => "subtag fits no position in the language identifier",
            Self::InvalidExtension => "invalid locale extension",
        })
    }
}

impl std::error::Error for LocaleParseError {}
