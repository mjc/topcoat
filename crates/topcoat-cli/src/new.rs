mod choice;
mod input;
mod options;

use std::path::PathBuf;

use clap::Args;
use console::style;

use self::input::{ChoiceArgs, Input};

#[derive(Args)]
pub struct NewCommand {
    /// Directory to create the application in
    path: Option<PathBuf>,
    /// Cargo package name (defaults to the directory name)
    #[arg(long)]
    name: Option<String>,
    /// Never prompt; fail if a choice is not given as a flag or by a preset
    #[arg(long)]
    no_interactive: bool,
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
        let input = Input::new(self.choices);
        let resolution = input.resolve()?;

        for note in &resolution.notes {
            println!("{} {note}", style("!").yellow());
        }
        println!(
            "{}",
            style(format!(
                "topcoat new {} {}",
                self.path.unwrap_or_default().display(),
                resolution.options.to_args().join(" ")
            ))
            .dim()
        );
        Ok(())
    }
}
