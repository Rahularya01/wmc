use clap::Parser;
use wmc::cli::{CliArgs, Subcommand, cmd_analyze, cmd_clean};
use wmc::config::default_media_path;
use wmc::tui::run_tui;

fn main() {
    let args = CliArgs::parse();
    let target = args.path.unwrap_or_else(default_media_path);

    if !target.exists() {
        eprintln!(
            "Error: target directory does not exist: {}",
            target.display()
        );
        std::process::exit(1);
    }

    match args.subcommand {
        Some(Subcommand::Analyze) => cmd_analyze(&target),
        Some(Subcommand::Clean(clean_args)) => {
            cmd_clean(&target, clean_args.yes, clean_args.dry_run);
        }
        Some(Subcommand::Ui) | None => run_tui(&target),
    }
}
