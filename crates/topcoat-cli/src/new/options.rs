use super::choice::{DatabaseBackend, Interaction, value_name};

/// The Iconify set used by Topcoat UI components.
pub const UI_ICON_SET: &str = "lucide";

/// Validated choices for a new application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectOptions {
    pub database: DatabaseSetup,
    pub interaction: Interaction,
    pub tailwind: bool,
    pub icons: IconSetup,
    pub font: FontSetup,
    pub ui: bool,
}

impl ProjectOptions {
    /// The Iconify sets to stage at build time, without duplicates.
    #[cfg_attr(not(test), expect(dead_code, reason = "used by the icon integration"))]
    pub fn icon_sets(&self) -> Vec<&str> {
        let mut sets = Vec::new();
        if let IconSetup::Iconify { set } = &self.icons {
            sets.push(set.as_str());
        }
        if self.ui && !sets.contains(&UI_ICON_SET) {
            sets.push(UI_ICON_SET);
        }
        sets
    }

    /// Returns command-line flags that select exactly these options.
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        let mut push = |flag: &str, value: Option<String>| {
            args.push(flag.to_string());
            args.extend(value);
        };

        match &self.database {
            DatabaseSetup::None => push("--database", Some("none".into())),
            DatabaseSetup::Toasty { backend } => {
                push("--database", Some("toasty".into()));
                push("--database-backend", Some(value_name(backend)));
            }
        }
        push("--interaction", Some(value_name(&self.interaction)));
        push(
            if self.tailwind {
                "--tailwind"
            } else {
                "--no-tailwind"
            },
            None,
        );
        match &self.icons {
            IconSetup::None => push("--icons", Some("none".into())),
            IconSetup::Custom => push("--icons", Some("custom".into())),
            IconSetup::Iconify { set } => {
                push("--icons", Some("iconify".into()));
                push("--icon-set", Some(set.clone()));
            }
        }
        match &self.font {
            FontSetup::None => push("--font", Some("none".into())),
            FontSetup::Fontsource { family } => {
                push("--font", Some("fontsource".into()));
                push("--font-family", Some(family.clone()));
            }
        }
        push(if self.ui { "--ui" } else { "--no-ui" }, None);
        args
    }
}

/// The selected database integration and its settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DatabaseSetup {
    None,
    Toasty { backend: DatabaseBackend },
}

/// The selected icon source and its settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IconSetup {
    None,
    Custom,
    Iconify { set: String },
}

/// The selected font source and its settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontSetup {
    None,
    Fontsource { family: String },
}
