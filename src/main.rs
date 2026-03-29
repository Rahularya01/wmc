use wmc::cli::{CliArgs, Subcommand, cmd_analyze, cmd_clean};
use wmc::tui::run_tui;

fn main() {
    let args = CliArgs::parse();
    match args.subcommand {
        Subcommand::Analyze => cmd_analyze(&args.target),
        Subcommand::Clean {
            skip_confirm,
            dry_run,
        } => cmd_clean(&args.target, skip_confirm, dry_run),
        Subcommand::Ui => run_tui(&args.target),
    }
}
