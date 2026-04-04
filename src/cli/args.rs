use std::path::PathBuf;

use crate::config::default_media_path;

/// Which top-level command the user requested.
pub enum Subcommand {
    /// Launch the interactive TUI (default when no subcommand is given).
    Ui,
    /// Print a media usage report to stdout.
    Analyze,
    /// Delete media files from disk and update the WhatsApp database.
    Clean { skip_confirm: bool, dry_run: bool },
}

/// Parsed command-line arguments.
pub struct CliArgs {
    pub subcommand: Subcommand,
    pub target: PathBuf,
}

impl CliArgs {
    /// Parses `std::env::args()` and exits the process with an error message on
    /// invalid input.
    ///
    /// Flags (`--dry-run`, `--yes`, `--path`) may appear in any order relative
    /// to the subcommand keyword.
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();

        let mut subcommand: Option<String> = None;
        let mut dry_run = false;
        let mut skip_confirm = false;
        let mut target_path: Option<PathBuf> = None;
        let mut i = 1;

        while i < args.len() {
            match args[i].as_str() {
                "ui" | "analyze" | "clean" => subcommand = Some(args[i].clone()),
                "--dry-run" | "-n" => dry_run = true,
                "--yes" | "-y" => skip_confirm = true,
                "--path" | "-p" => {
                    i += 1;
                    if i < args.len() {
                        target_path = Some(PathBuf::from(&args[i]));
                    } else {
                        eprintln!("Error: --path requires an argument");
                        std::process::exit(1);
                    }
                }
                "--help" | "-h" => {
                    super::commands::print_help();
                    std::process::exit(0);
                }
                other => {
                    eprintln!("Unknown argument: {}\nRun `wmc --help` for usage.", other);
                    std::process::exit(1);
                }
            }
            i += 1;
        }

        let target = target_path.unwrap_or_else(default_media_path);
        if !target.exists() {
            eprintln!(
                "Error: target directory does not exist: {}",
                target.display()
            );
            std::process::exit(1);
        }

        let subcommand = match subcommand.as_deref() {
            Some("analyze") => Subcommand::Analyze,
            Some("clean") => Subcommand::Clean {
                skip_confirm,
                dry_run,
            },
            Some("ui") | None => Subcommand::Ui,
            _ => unreachable!(),
        };

        CliArgs { subcommand, target }
    }
}
