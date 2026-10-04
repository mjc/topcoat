use std::fmt::Write as _;

use askama::Template;
use topcoat_core_grammar::pretty::pretty_print_str;
use topcoat_font::fontsource::{Family, Style};

use super::{
    choice::{Interaction, Routing},
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
/// The `serde` version generated applications depend on.
const SERDE_VERSION: &str = "1";
/// The Iconify set the home page renders an example icon from.
const EXAMPLE_ICON_SET: &str = "lucide";

/// `src/main.rs`: the module declarations and entry point.
#[derive(Template)]
#[template(path = "main.rs.askama", escape = "none")]
struct MainRs {
    /// Whether the application has a module of hand-written icons.
    custom_icons: bool,
}

/// `src/app.rs`: the router, root layout, and home page.
#[derive(Template)]
#[template(path = "app.rs.askama", escape = "none")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool switches a template section"
)]
struct AppRs<'a> {
    /// The page title as a Rust string literal.
    title: &'a str,
    routing: Routing,
    paths: Paths,
    classes: Classes,
    interaction: Interaction,
    /// Whether the application has the counter example in `app::counter`.
    counter: bool,
    tailwind: bool,
    /// Whether the home page shows an icon from the default Iconify set.
    iconify_example: bool,
    /// The Iconify set to explain in a comment when it has no example icon.
    icon_set_hint: Option<&'a str>,
    /// Whether the application has a module of hand-written icons.
    custom_icons: bool,
    font: Option<FontInfo>,
}

/// The Fontsource family a generated application loads.
struct FontInfo {
    /// The name of the family's constant, e.g. `INTER`.
    ident: &'static str,
    /// The family's display name, e.g. `Inter`.
    name: &'static str,
    /// The arguments of `fontsource_font!` after the family, each preceded by `, `.
    args: String,
    /// The generic CSS family used until the font loads or if it fails to.
    generic: &'static str,
}

impl FontInfo {
    /// The weights the starter pages use: regular text and bold headings.
    const WEIGHTS: [u16; 2] = [400, 700];

    /// Describes `family`, limited to the starter's weights in the normal style where the
    /// family offers them, since every included face is preloaded.
    fn new(family: &Family) -> Self {
        let weights: Vec<String> = Self::WEIGHTS
            .iter()
            .filter(|weight| family.has_weight(**weight))
            .map(ToString::to_string)
            .collect();
        let mut args = String::new();
        if !weights.is_empty() {
            write!(args, ", weight: [{}]", weights.join(", "))
                .expect("writing to a String cannot fail");
        }
        if family.has_style(Style::Normal) {
            args.push_str(", style: Normal");
        }

        Self {
            ident: family.ident,
            name: family.name,
            args,
            generic: match family.category {
                "serif" => "serif",
                "monospace" => "monospace",
                _ => "sans-serif",
            },
        }
    }
}

/// Renders the arguments of handler attributes for the selected routing style.
#[derive(Clone, Copy)]
struct Paths(Routing);

impl Paths {
    /// The arguments of a `#[page]` or `#[layout]` attribute for a handler at `path`:
    /// empty with module routing, where paths come from modules.
    fn page(self, path: &str) -> String {
        match self.0 {
            Routing::Module => String::new(),
            Routing::Discover | Routing::Manual => format!("({path:?})"),
        }
    }

    /// The arguments of a `#[route]` attribute for a `method` handler at `path`.
    fn route(self, method: &str, path: &str) -> String {
        match self.0 {
            Routing::Module => format!("({method})"),
            Routing::Discover | Routing::Manual => format!("({method} {path:?})"),
        }
    }
}

/// `class` attributes for the starter's elements, each with a leading space: Tailwind
/// utility classes, or nothing with the plain stylesheet.
#[derive(Clone, Copy)]
struct Classes {
    main: &'static str,
    heading: &'static str,
    paragraph: &'static str,
    link: &'static str,
    button: &'static str,
}

impl Classes {
    fn new(tailwind: bool) -> Self {
        if tailwind {
            Self {
                main: r#" class="mx-auto max-w-2xl px-4 py-16""#,
                heading: r#" class="text-3xl font-bold""#,
                paragraph: r#" class="mt-4""#,
                link: r#" class="text-blue-600 underline""#,
                button: r#" class="rounded border px-3 py-1""#,
            }
        } else {
            Self {
                main: "",
                heading: "",
                paragraph: "",
                link: "",
                button: "",
            }
        }
    }
}

/// `src/app/counter.rs` with Topcoat's browser runtime.
#[derive(Template)]
#[template(path = "counter/topcoat.rs.askama", escape = "none")]
struct TopcoatCounter {
    paths: Paths,
    classes: Classes,
}

/// `src/app/counter.rs` with htmx.
#[derive(Template)]
#[template(path = "counter/htmx.rs.askama", escape = "none")]
struct HtmxCounter {
    paths: Paths,
    classes: Classes,
}

/// `src/app/counter.rs` with Datastar.
#[derive(Template)]
#[template(path = "counter/datastar.rs.askama", escape = "none")]
struct DatastarCounter {
    paths: Paths,
    classes: Classes,
}

/// `src/app/counter.rs` with Alpine AJAX.
#[derive(Template)]
#[template(path = "counter/alpine_ajax.rs.askama", escape = "none")]
struct AlpineAjaxCounter {
    paths: Paths,
    classes: Classes,
}

/// `build.rs`: build steps of the selected integrations.
#[derive(Template)]
#[template(path = "build.rs.askama", escape = "none")]
struct BuildRs<'a> {
    tailwind: bool,
    icon_sets: Vec<&'a str>,
}

impl BuildRs<'_> {
    /// Whether any build step is selected. Without one, no build script is generated.
    fn is_needed(&self) -> bool {
        self.tailwind || !self.icon_sets.is_empty()
    }
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
        topcoat().features(["asset", "router", "serve", "view"]),
    )?;
    if options.routing != Routing::Manual {
        manifest.dependency("topcoat", topcoat().features(["discover"]))?;
    }
    if options.tailwind {
        manifest.dependency("topcoat", topcoat().features(["tailwind"]))?;
        manifest.build_dependency("topcoat", topcoat().features(["tailwind"]))?;
    }
    match &options.icons {
        IconSetup::None => {}
        IconSetup::Custom => manifest.dependency("topcoat", topcoat().features(["icon"]))?,
        IconSetup::Iconify { .. } => {
            manifest.dependency("topcoat", topcoat().features(["icon-iconify"]))?;
            manifest.build_dependency("topcoat", topcoat().features(["icon-iconify"]))?;
        }
    }

    match options.interaction {
        Interaction::None => {}
        Interaction::Topcoat => manifest.dependency("topcoat", topcoat().features(["runtime"]))?,
        Interaction::Htmx => manifest.dependency("topcoat", topcoat().features(["htmx"]))?,
        Interaction::Datastar => {
            manifest.dependency("topcoat", topcoat().features(["datastar"]))?;
            manifest.dependency("serde", Dependency::new(SERDE_VERSION).features(["derive"]))?;
        }
        Interaction::AlpineAjax => {
            manifest.dependency("topcoat", topcoat().features(["alpine-ajax"]))?;
        }
    }

    let font = match &options.font {
        FontSetup::None => None,
        FontSetup::Fontsource { family } => {
            manifest.dependency("topcoat", topcoat().features(["font-fontsource"]))?;
            let family = Family::by_id(family)
                .ok_or_else(|| format!("unknown Fontsource family `{family}`"))?;
            Some(FontInfo::new(family))
        }
    };

    let mut plan = ProjectPlan::default();

    // Only the default set has an icon name known to exist for the example.
    let iconify_set = match &options.icons {
        IconSetup::Iconify { set } => Some(set.as_str()),
        IconSetup::None | IconSetup::Custom => None,
    };
    let iconify_example = iconify_set == Some(EXAMPLE_ICON_SET);
    let custom_icons = options.icons == IconSetup::Custom;
    if custom_icons {
        plan.add("src/icons.rs", include_str!("templates/icons.rs"))?;
    }

    // A package name has no characters that need escaping, so its debug form is a
    // valid string literal.
    let title = format!("{:?}", name.as_str());
    let paths = Paths(options.routing);
    let classes = Classes::new(options.tailwind);

    let counter = match options.interaction {
        Interaction::None => None,
        Interaction::Topcoat => Some(render(&TopcoatCounter { paths, classes })?),
        Interaction::Htmx => Some(render(&HtmxCounter { paths, classes })?),
        Interaction::Datastar => Some(render(&DatastarCounter { paths, classes })?),
        Interaction::AlpineAjax => Some(render(&AlpineAjaxCounter { paths, classes })?),
    };
    if let Some(counter) = &counter {
        plan.add(
            "src/app/counter.rs",
            format_rust("src/app/counter.rs", counter)?,
        )?;
    }

    let app = render(&AppRs {
        title: &title,
        routing: options.routing,
        paths,
        classes,
        interaction: options.interaction,
        counter: counter.is_some(),
        tailwind: options.tailwind,
        iconify_example,
        icon_set_hint: iconify_set.filter(|_| !iconify_example),
        custom_icons,
        font,
    })?;
    plan.add("src/app.rs", format_rust("src/app.rs", &app)?)?;
    let main = render(&MainRs { custom_icons })?;
    plan.add("src/main.rs", format_rust("src/main.rs", &main)?)?;

    let build = BuildRs {
        tailwind: options.tailwind,
        icon_sets: options.icon_sets(),
    };
    if build.is_needed() {
        plan.add("build.rs", format_rust("build.rs", &render(&build)?)?)?;
    }

    // With Tailwind, the stylesheet is the Tailwind input.
    let styles = if options.tailwind {
        include_str!("templates/tailwind.css")
    } else {
        include_str!("templates/styles.css")
    };
    plan.add("styles.css", styles)?;

    plan.add("Cargo.toml", manifest.render())?;
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

/// The `topcoat` dependency without default features. Features are added per
/// integration and merged by the manifest.
fn topcoat() -> Dependency {
    Dependency::new(TOPCOAT_VERSION).no_default_features()
}

fn render(template: &impl Template) -> Result<String, String> {
    let mut output = template
        .render()
        .map_err(|error| format!("failed to render a template: {error}"))?;
    // Askama drops the final newline of every template file; restore it.
    output.push('\n');
    Ok(output)
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
            routing: Routing::Module,
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

    fn features(dependency: &toml::Value) -> Vec<&str> {
        dependency["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|feature| feature.as_str().unwrap())
            .collect()
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
        assert!(!features(topcoat).contains(&"runtime"));
        assert!(!features(topcoat).contains(&"tailwind"));

        assert!(file(&plan, "src/app.rs").unwrap().contains("\"my-app\""));
        assert!(file(&plan, "build.rs").is_none());
        assert!(file(&plan, "src/icons.rs").is_none());
    }

    #[test]
    fn manual_routing_registers_handlers_without_discovery() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            routing: Routing::Manual,
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(!features(&manifest["dependencies"]["topcoat"]).contains(&"discover"));
        let app = file(&plan, "src/app.rs").unwrap();
        assert!(app.contains(".page(home)"));
        assert!(app.contains("#[page(\"/\")]"));
        assert!(!app.contains("discover"));
    }

    #[test]
    fn iconify_stages_the_chosen_set() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            icons: IconSetup::Iconify {
                set: "tabler".to_string(),
            },
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"icon-iconify"));
        assert!(features(&manifest["build-dependencies"]["topcoat"]).contains(&"icon-iconify"));
        let build_script = file(&plan, "build.rs").unwrap();
        assert!(build_script.contains(".icon_set(\"tabler\")"));
        assert!(!build_script.contains("lucide"));
    }

    #[test]
    fn fontsource_fonts_are_registered_unless_discovered() {
        let name = PackageName::new("my-app").unwrap();
        for routing in [Routing::Module, Routing::Discover, Routing::Manual] {
            let options = ProjectOptions {
                routing,
                font: FontSetup::Fontsource {
                    family: "roboto".to_string(),
                },
                ..minimal()
            };
            let plan = generate(&name, &options).unwrap();

            let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
            assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"font-fontsource"));
            let app = file(&plan, "src/app.rs").unwrap();
            assert!(app.contains("fontsource_font!(ROBOTO, weight: [400, 700], style: Normal)"));
            assert!(app.contains("font::link(font: ROBOTO)"));
            let registered = app.contains(".font(ROBOTO)");
            assert_eq!(registered, routing != Routing::Discover, "{routing:?}");
        }
    }

    #[test]
    fn each_interaction_adds_its_feature_and_counter() {
        let name = PackageName::new("my-app").unwrap();
        for (interaction, feature) in [
            (Interaction::Topcoat, "runtime"),
            (Interaction::Htmx, "htmx"),
            (Interaction::Datastar, "datastar"),
            (Interaction::AlpineAjax, "alpine-ajax"),
        ] {
            let options = ProjectOptions {
                routing: Routing::Manual,
                interaction,
                ..minimal()
            };
            let plan = generate(&name, &options).unwrap();

            let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
            let dependencies = &manifest["dependencies"];
            assert!(
                features(&dependencies["topcoat"]).contains(&feature),
                "{feature}"
            );
            assert_eq!(
                dependencies.get("serde").is_some(),
                interaction == Interaction::Datastar,
                "{feature}"
            );

            assert!(file(&plan, "src/app/counter.rs").is_some(), "{feature}");
            let app = file(&plan, "src/app.rs").unwrap();
            assert!(app.contains("mod counter;"), "{feature}");
            assert!(app.contains(".page(counter::page)"), "{feature}");
            // Only server-side counters have a route to register.
            assert_eq!(
                app.contains(".route(counter::increment)"),
                interaction != Interaction::Topcoat,
                "{feature}"
            );
        }

        let plan = generate(&name, &minimal()).unwrap();
        assert!(file(&plan, "src/app/counter.rs").is_none());
    }

    #[test]
    fn browser_scripts_match_the_examples() {
        let name = PackageName::new("my-app").unwrap();
        for (interaction, example) in [
            (Interaction::Htmx, "htmx"),
            (Interaction::Datastar, "datastar"),
            (Interaction::AlpineAjax, "alpine-ajax"),
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples")
                .join(example)
                .join("src/main.rs");
            let source = std::fs::read_to_string(path).unwrap();
            let urls: Vec<&str> = source
                .split('"')
                .filter(|part| part.starts_with("https://cdn.jsdelivr.net/"))
                .collect();
            assert!(!urls.is_empty(), "{example}");

            let options = ProjectOptions {
                interaction,
                ..minimal()
            };
            let plan = generate(&name, &options).unwrap();
            let app = file(&plan, "src/app.rs").unwrap();
            for url in urls {
                assert!(app.contains(url), "{example}: {url}");
            }
        }
    }

    #[test]
    fn custom_icons_live_in_their_own_module() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            icons: IconSetup::Custom,
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        assert!(file(&plan, "src/icons.rs").is_some());
        assert!(file(&plan, "src/main.rs").unwrap().contains("mod icons;"));
        assert!(file(&plan, "build.rs").is_none());
    }

    #[test]
    fn tailwind_builds_the_stylesheet_from_styles_css() {
        let name = PackageName::new("my-app").unwrap();
        let options = ProjectOptions {
            tailwind: true,
            ..minimal()
        };
        let plan = generate(&name, &options).unwrap();

        let manifest: toml::Table = file(&plan, "Cargo.toml").unwrap().parse().unwrap();
        assert!(features(&manifest["dependencies"]["topcoat"]).contains(&"tailwind"));
        let build = &manifest["build-dependencies"]["topcoat"];
        assert_eq!(build["default-features"].as_bool(), Some(false));
        assert!(features(build).contains(&"tailwind"));

        let build_script = file(&plan, "build.rs").unwrap();
        assert!(build_script.contains("\"styles.css\""));
        assert!(file(&plan, "styles.css").unwrap().contains("tailwindcss"));
        assert!(
            file(&plan, "src/app.rs")
                .unwrap()
                .contains("tailwind::stylesheet!")
        );
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
    fn dependency_versions_match_the_workspace() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
        let workspace: toml::Table = std::fs::read_to_string(path).unwrap().parse().unwrap();
        for (name, expected) in [("tokio", TOKIO_VERSION), ("serde", SERDE_VERSION)] {
            let dependency = &workspace["workspace"]["dependencies"][name];
            let version = dependency
                .as_str()
                .or_else(|| dependency["version"].as_str());
            assert_eq!(version, Some(expected), "{name}");
        }
    }
}
