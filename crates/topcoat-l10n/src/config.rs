use crate::Locale;

/// The locales an application serves.
///
/// Register it with the router's `l10n` method. Use [`L10nConfig::builder`]
/// to create it.
#[derive(Debug, Clone)]
pub struct L10nConfig {
    supported: Vec<Locale>,
    fallback: Locale,
}

impl L10nConfig {
    /// Creates a builder for a localization configuration.
    #[must_use]
    pub fn builder() -> L10nConfigBuilder {
        L10nConfigBuilder::default()
    }

    /// Returns the supported locales in order of preference. The fallback
    /// locale is always among them.
    #[must_use]
    pub fn supported(&self) -> &[Locale] {
        &self.supported
    }

    /// Returns the locale used when a request matches none of the supported
    /// locales.
    #[must_use]
    pub const fn fallback(&self) -> &Locale {
        &self.fallback
    }
}

/// Assembles an [`L10nConfig`]. Created with [`L10nConfig::builder`].
#[derive(Debug, Default)]
pub struct L10nConfigBuilder {
    supported: Vec<Locale>,
    fallback: Option<Locale>,
}

impl L10nConfigBuilder {
    /// Adds supported locales. Earlier locales win when a request could match
    /// several.
    #[must_use]
    pub fn supported(mut self, locales: impl IntoIterator<Item = Locale>) -> Self {
        self.supported.extend(locales);
        self
    }

    /// Overrides the fallback locale. Without this, the first supported locale
    /// is the fallback.
    #[must_use]
    pub fn fallback(mut self, locale: Locale) -> Self {
        self.fallback = Some(locale);
        self
    }

    /// Consumes the builder, returning the finished [`L10nConfig`].
    ///
    /// # Panics
    ///
    /// Panics when neither a supported locale nor a fallback was set.
    #[must_use]
    #[track_caller]
    pub fn build(self) -> L10nConfig {
        let mut supported = self.supported;
        let fallback = match self.fallback {
            Some(fallback) => fallback,
            None => match supported.first() {
                Some(first) => first.clone(),
                None => panic!(
                    "no locales configured: add at least one with `L10nConfigBuilder::supported`"
                ),
            },
        };
        if !supported.contains(&fallback) {
            supported.push(fallback.clone());
        }
        L10nConfig {
            supported,
            fallback,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    #[test]
    fn first_supported_locale_is_the_default_fallback() {
        let config = L10nConfig::builder()
            .supported([locale!("de"), locale!("en")])
            .build();

        assert_eq!(config.fallback(), &locale!("de"));
    }

    #[test]
    fn an_explicit_fallback_counts_as_supported() {
        let config = L10nConfig::builder()
            .supported([locale!("de")])
            .fallback(locale!("en"))
            .build();

        assert_eq!(config.fallback(), &locale!("en"));
        assert!(config.supported().contains(&locale!("en")));
        assert!(config.supported().contains(&locale!("de")));
    }

    #[test]
    #[should_panic(expected = "no locales configured")]
    fn building_without_locales_panics() {
        let _ = L10nConfig::builder().build();
    }
}
