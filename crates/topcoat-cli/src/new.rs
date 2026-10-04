mod choice;
mod generate;
mod input;
mod manifest;
mod name;
mod options;
mod plan;
mod publish;

use std::path::{Path, PathBuf};

use clap::Args;
use console::style;

use self::{
    input::{ChoiceArgs, Input},
    name::PackageName,
    options::DatabaseSetup,
};

#[derive(Args)]
pub struct NewCommand {
    /// Directory to create the application in
    path: PathBuf,
    /// Cargo package name (defaults to the directory name)
    #[arg(long)]
    name: Option<String>,
    #[command(flatten)]
    choices: ChoiceArgs,
}

impl NewCommand {
    pub fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let name = match &self.name {
            Some(name) => name.as_str(),
            None => self
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    format!(
                        "cannot derive a package name from {}; pass --name",
                        self.path.display()
                    )
                })?,
        };
        let name = PackageName::new(name)?;

        let resolution = Input::new(self.choices).resolve()?;
        let plan = generate::generate(&name, &resolution.options)?;
        publish::publish(&plan, &self.path)?;

        println!(
            "{} created {} {}",
            style("+").green(),
            style(&name).bold(),
            style(format!("({})", self.path.display())).dim(),
        );
        for note in &resolution.notes {
            println!("{} {note}", style("!").yellow());
        }

        let path = shell_quote(&self.path);
        let mut command = vec!["topcoat".to_string(), "new".to_string(), path.clone()];
        if self.name.is_some() {
            command.extend(["--name".to_string(), name.to_string()]);
        }
        command.extend(resolution.options.to_args());
        println!();
        println!(
            "Equivalent command for topcoat-cli {}:",
            env!("CARGO_PKG_VERSION")
        );
        println!("  {}", style(command.join(" ")).dim());

        println!();
        let mut steps = vec![format!("cd {path}")];
        if let DatabaseSetup::Toasty { .. } = resolution.options.database {
            println!("Create the database tables, then start the development server:");
            steps.push("cargo run -- toasty migration generate".to_string());
            steps.push("cargo run -- toasty migration apply".to_string());
        } else {
            println!("Start the development server:");
        }
        steps.push("topcoat dev".to_string());
        for step in steps {
            println!("  {}", style(step).bold());
        }
        Ok(())
    }
}

/// Quotes `path` for a POSIX shell if it contains characters other than ASCII letters,
/// digits, and `_-./`.
fn shell_quote(path: &Path) -> String {
    let path = path.to_string_lossy();
    let plain = !path.is_empty()
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-./".contains(&byte));
    if plain {
        path.into_owned()
    } else {
        format!("'{}'", path.replace('\'', r"'\''"))
    }
}
