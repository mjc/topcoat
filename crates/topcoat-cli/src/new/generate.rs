use askama::Template;
use topcoat_core_grammar::pretty::pretty_print_str;

use super::{
    choice::Interaction,
    manifest::{Dependency, Manifest},
    name::PackageName,
    options::{DatabaseSetup, FontSetup, IconSetup, ProjectOptions},
    plan::ProjectPlan,
};
use crate::common::format;

/// The `topcoat` version generated applications depend on. The CLI is released together
/// with `topcoat`.
const TOPCOAT_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The `tokio` version generated applications depend on.
const TOKIO_VERSION: &str = "1.51.1";

/// `src/main.rs`: the entry point, root layout, and home page.
#[derive(Template)]
#[template(path = "main.rs.askama", escape = "none")]
struct MainRs<'a> {
    /// The page title as a Rust string literal.
    title: &'a str,
}

#[derive(Template)]
#[template(path = "README.md.askama", escape = "none")]
struct Readme<'a> {
    name: &'a str,
}

/// Renders the files of a new application named `name` with the given options.
///
/// # Errors
///
/// Returns an error if the options select an integration that cannot be generated, or
/// if rendering or formatting a file fails.
pub fn generate(name: &PackageName, options: &ProjectOptions) -> Result<ProjectPlan, String> {
    check_supported(options)?;

    let mut manifest = Manifest::new(name.clone());
    manifest.dependency(
        "tokio",
        Dependency::new(TOKIO_VERSION).features(["macros", "rt-multi-thread"]),
    )?;
    manifest.dependency(
        "topcoat",
        Dependency::new(TOPCOAT_VERSION)
            .no_default_features()
            .features(["asset", "discover", "router", "serve", "view"]),
    )?;

    // A package name has no characters that need escaping, so its debug form is a
    // valid string literal.
    let title = format!("{:?}", name.as_str());
    let main = render(&MainRs { title: &title })?;

    let mut plan = ProjectPlan::default();
    plan.add("Cargo.toml", manifest.render())?;
    plan.add("src/main.rs", format_rust("src/main.rs", &main)?)?;
    plan.add("styles.css", include_str!("templates/styles.css"))?;
    plan.add(".gitignore", include_str!("templates/gitignore"))?;
    plan.add(
        "README.md",
        render(&Readme {
            name: name.as_str(),
        })?,
    )?;
    Ok(plan)
}

/// Rejects options selecting integrations the generator does not support yet.
fn check_supported(options: &ProjectOptions) -> Result<(), String> {
    let unsupported: Vec<&str> = [
        (options.database != DatabaseSetup::None, "--database toasty"),
        (options.interaction != Interaction::None, "--interaction"),
        (options.tailwind, "--tailwind"),
        (options.icons != IconSetup::None, "--icons"),
        (options.font != FontSetup::None, "--font"),
        (options.ui, "--ui"),
    ]
    .into_iter()
    .filter_map(|(selected, flag)| selected.then_some(flag))
    .collect();
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "not supported yet: {}; use --minimal",
            unsupported.join(", ")
        ))
    }
}

fn render(template: &impl Template) -> Result<String, String> {
    template
        .render()
        .map_err(|error| format!("failed to render a template: {error}"))
}

/// Formats the Topcoat macro bodies in `source`.
fn format_rust(path: &str, source: &str) -> Result<String, String> {
    pretty_print_str(&format::registry(), source).map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        format!("failed to format {path}: {}", errors.join("; "))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::new::choice::DatabaseBackend;

    fn minimal() -> ProjectOptions {
        ProjectOptions {
            database: DatabaseSetup::None,
            interaction: Interaction::None,
            tailwind: false,
            icons: IconSetup::None,
            font: FontSetup::None,
            ui: false,
        }
    }

    fn file<'a>(plan: &'a ProjectPlan, path: &str) -> Option<&'a str> {
        plan.files()
            .find(|(file, _)| *file == Path::new(path))
            .map(|(_, contents)| contents)
    }

    #[test]
    fn minimal_application_depends_only_on_the_features_it_uses() {
        let name = PackageName::new("my-app").unwrap();
        let plan = generate(&name, &minimal()).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert_eq!(manifest["package"]["name"].as_str(), Some("my-app"));
        let topcoat = &manifest["dependencies"]["topcoat"];
        assert_eq!(topcoat["version"].as_str(), Some(TOPCOAT_VERSION));
        assert_eq!(topcoat["default-features"].as_bool(), Some(false));
        let features = topcoat["features"].as_array().unwrap();
        assert!(
            !features
                .iter()
                .any(|feature| feature.as_str() == Some("runtime"))
        );

        let main = file(&plan, "src/main.rs").unwrap();
        assert!(main.contains("\"my-app\""));
        assert!(file(&plan, "build.rs").is_none());
    }

    #[test]
    fn rejects_integrations_that_cannot_be_generated_yet() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            database: DatabaseSetup::Toasty {
                backend: DatabaseBackend::Sqlite,
            },
            ..minimal()
        };
        assert!(generate(&name, &options).is_err());
    }

    #[test]
    fn tokio_version_matches_the_workspace() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
        let workspace: toml::Table = std::fs::read_to_string(path).unwrap().parse().unwrap();
        let tokio = &workspace["workspace"]["dependencies"]["tokio"];
        let version = tokio.as_str().or_else(|| tokio["version"].as_str());
        assert_eq!(version, Some(TOKIO_VERSION));
    }
}
