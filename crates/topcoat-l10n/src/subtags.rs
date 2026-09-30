use std::{
    fmt::{self, Display},
    str::FromStr,
};

use icu_locale_core::subtags;

use crate::LocaleParseError;

/// Defines a `Copy` wrapper around one of ICU's locale subtag types.
macro_rules! subtag {
    (
        $(#[$meta:meta])*
        $name:ident($icu:ty), $macro:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($icu);

        impl $name {
            /// Wraps a subtag from the `icu_locale_core` crate.
            #[must_use]
            pub const fn from_icu(subtag: $icu) -> Self {
                Self(subtag)
            }

            /// Returns the underlying `icu_locale_core` subtag.
            #[must_use]
            pub const fn as_icu(&self) -> &$icu {
                &self.0
            }

            /// Returns the canonical text of the subtag.
            #[must_use]
            pub const fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl From<$icu> for $name {
            fn from(subtag: $icu) -> Self {
                Self(subtag)
            }
        }

        impl From<$name> for $icu {
            fn from(subtag: $name) -> Self {
                subtag.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = LocaleParseError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                <$icu>::try_from_str(s).map(Self)
            }
        }
    };
}

subtag! {
    /// A language subtag, such as `en` or `zh`.
    ///
    /// Create one with the [`language!`](crate::language) macro or parse one
    /// with [`str::parse`]. Parsing canonicalizes letter case.
    Language(subtags::Language), language
}

impl Language {
    /// The unknown language, written `und`.
    pub const UNKNOWN: Self = Self(subtags::Language::UNKNOWN);

    /// Returns whether this is the unknown language.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        self.0.is_unknown()
    }
}

subtag! {
    /// A script subtag, such as `Latn` or `Hant`.
    ///
    /// Create one with the [`script!`](crate::script) macro or parse one with
    /// [`str::parse`]. Parsing canonicalizes letter case.
    Script(subtags::Script), script
}

subtag! {
    /// A region subtag, such as `US` or `419`.
    ///
    /// Create one with the [`region!`](crate::region) macro or parse one with
    /// [`str::parse`]. Parsing canonicalizes letter case.
    Region(subtags::Region), region
}

subtag! {
    /// A variant subtag, such as `posix` or `1996`.
    ///
    /// Create one with the [`variant!`](crate::variant) macro or parse one
    /// with [`str::parse`]. Parsing canonicalizes letter case.
    Variant(subtags::Variant), variant
}

/// Creates a [`Language`] from a subtag known at compile time.
///
/// The subtag is checked during compilation, so the macro is usable in
/// `const` and `static` items.
#[macro_export]
macro_rules! language {
    ($subtag:literal) => {
        $crate::Language::from_icu($crate::__icu::subtags::language!($subtag))
    };
}

/// Creates a [`Script`] from a subtag known at compile time.
///
/// The subtag is checked during compilation, so the macro is usable in
/// `const` and `static` items.
#[macro_export]
macro_rules! script {
    ($subtag:literal) => {
        $crate::Script::from_icu($crate::__icu::subtags::script!($subtag))
    };
}

/// Creates a [`Region`] from a subtag known at compile time.
///
/// The subtag is checked during compilation, so the macro is usable in
/// `const` and `static` items.
#[macro_export]
macro_rules! region {
    ($subtag:literal) => {
        $crate::Region::from_icu($crate::__icu::subtags::region!($subtag))
    };
}

/// Creates a [`Variant`] from a subtag known at compile time.
///
/// The subtag is checked during compilation, so the macro is usable in
/// `const` and `static` items.
#[macro_export]
macro_rules! variant {
    ($subtag:literal) => {
        $crate::Variant::from_icu($crate::__icu::subtags::variant!($subtag))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_canonicalizes_case() {
        assert_eq!("DE".parse::<Language>().unwrap(), language!("de"));
        assert_eq!("hant".parse::<Script>().unwrap(), script!("Hant"));
        assert_eq!("at".parse::<Region>().unwrap(), region!("AT"));
        assert_eq!("POSIX".parse::<Variant>().unwrap(), variant!("posix"));
    }

    #[test]
    fn displays_the_canonical_text() {
        assert_eq!(region!("us").to_string(), "US");
        assert_eq!(script!("LATN").to_string(), "Latn");
    }

    #[test]
    fn rejects_malformed_subtags() {
        assert!("english".parse::<Language>().is_err());
        assert!("Latin".parse::<Script>().is_err());
        assert!("USA".parse::<Region>().is_err());
    }

    #[test]
    fn unknown_language_is_recognized() {
        assert!(Language::UNKNOWN.is_unknown());
        assert!(!language!("en").is_unknown());
        assert_eq!("und".parse::<Language>().unwrap(), Language::UNKNOWN);
    }
}
