use std::{
    fmt::{self, Display},
    str::FromStr,
};

pub use icu_locale_core::ParseError as LocaleParseError;

use crate::{Language, Region, Script, Variant};

/// A Unicode locale identifier, such as `en`, `de-AT`, or `zh-Hant-TW`.
///
/// Create one with the [`locale!`](crate::locale) macro for a tag known at
/// compile time, or parse one with [`str::parse`] at runtime. Parsing
/// canonicalizes letter case, so `de-at` and `de-AT` are the same locale.
///
/// The request's locale is available through [`locale(cx)`](crate::locale).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Locale(icu_locale_core::Locale);

impl Locale {
    /// Wraps a locale from the `icu_locale_core` crate.
    #[must_use]
    pub const fn from_icu(locale: icu_locale_core::Locale) -> Self {
        Self(locale)
    }

    /// Returns the underlying `icu_locale_core` locale.
    #[must_use]
    pub const fn as_icu(&self) -> &icu_locale_core::Locale {
        &self.0
    }

    /// Returns the language subtag.
    #[must_use]
    pub fn language(&self) -> Language {
        Language::from_icu(self.0.id.language)
    }

    /// Returns the script subtag, if present.
    #[must_use]
    pub fn script(&self) -> Option<Script> {
        self.0.id.script.map(Script::from_icu)
    }

    /// Returns the region subtag, if present.
    #[must_use]
    pub fn region(&self) -> Option<Region> {
        self.0.id.region.map(Region::from_icu)
    }

    /// Returns the variant subtags in canonical order.
    pub fn variants(&self) -> impl Iterator<Item = Variant> + '_ {
        self.0.id.variants.iter().copied().map(Variant::from_icu)
    }

    /// Returns whether both locales name the same language, script, region,
    /// and variants, ignoring extensions.
    pub(crate) fn same_id(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl From<icu_locale_core::Locale> for Locale {
    fn from(locale: icu_locale_core::Locale) -> Self {
        Self(locale)
    }
}

impl From<Locale> for icu_locale_core::Locale {
    fn from(locale: Locale) -> Self {
        locale.0
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Locale {
    type Err = LocaleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        icu_locale_core::Locale::try_from_str(s).map(Self)
    }
}

/// Creates a [`Locale`] from a tag known at compile time.
///
/// The tag is checked and canonicalized during compilation, so the macro is
/// usable in `const` and `static` items.
///
/// # Examples
///
/// ```rust
/// use topcoat::l10n::{Locale, locale, region};
///
/// const DEFAULT: Locale = locale!("en-US");
///
/// assert_eq!(DEFAULT.language().as_str(), "en");
/// assert_eq!(DEFAULT.region(), Some(region!("US")));
/// ```
#[macro_export]
macro_rules! locale {
    ($tag:literal) => {
        $crate::Locale::from_icu($crate::__icu::locale!($tag))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{language, region, script, variant};

    #[test]
    fn parsing_canonicalizes_case() {
        let lower: Locale = "de-at".parse().unwrap();
        let canonical = locale!("de-AT");

        assert_eq!(lower, canonical);
        assert_eq!(lower.to_string(), canonical.to_string());
    }

    #[test]
    fn exposes_subtags() {
        let locale: Locale = "zh-Hant-TW-posix".parse().unwrap();

        assert_eq!(locale.language(), language!("zh"));
        assert_eq!(locale.script(), Some(script!("Hant")));
        assert_eq!(locale.region(), Some(region!("TW")));
        assert_eq!(locale.variants().collect::<Vec<_>>(), [variant!("posix")]);
        assert_eq!(locale!("en").region(), None);
    }

    #[test]
    fn rejects_invalid_tags() {
        assert!("*".parse::<Locale>().is_err());
        assert!("en;q=0.8".parse::<Locale>().is_err());
    }
}
