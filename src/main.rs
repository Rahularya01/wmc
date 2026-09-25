use clap::Parser;
use std::io::ErrorKind;
use std::path::Path;
use wmc::cli::{CliArgs, Subcommand, cmd_analyze, cmd_clean};
use wmc::config::default_media_path;
use wmc::tui::run_tui;

fn main() {
    let args = CliArgs::parse();
    let target = args.path.unwrap_or_else(default_media_path);

    // `Path::exists()` returns false on any error, which hides macOS privacy
    // (TCC) denials behind a misleading "does not exist". Probe with read_dir
    // so permission failures are reported as such.
    if let Err(error) = std::fs::read_dir(&target) {
        match error.kind() {
            ErrorKind::NotFound => eprintln!(
                "Error: target directory does not exist: {}",
                target.display()
            ),
            ErrorKind::PermissionDenied => print_permission_help(&target),
            _ => eprintln!("Error: cannot read {}: {}", target.display(), error),
        }
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

/// Explains how to grant the terminal macOS's "access data from other apps"
/// permission, which is narrower than Full Disk Access. TCC attributes a CLI's
/// file access to the terminal that launched it, so the grant lands there.
fn print_permission_help(target: &Path) {
    let terminal = std::env::var("__CFBundleIdentifier").ok();
    let reset_cmd = match &terminal {
        Some(bundle_id) => format!("tccutil reset SystemPolicyAppData {bundle_id}"),
        None => "tccutil reset SystemPolicyAppData <terminal-bundle-id>".to_string(),
    };

    eprintln!(
        "Error: permission denied reading {}\n\n\
         macOS protects WhatsApp's data. wmc only needs your terminal to be allowed to\n\
         \"access data from other apps\" — Full Disk Access is not required.\n\n\
         - If macOS just showed that prompt, click Allow and run wmc again.\n\
         - If you denied it earlier, macOS won't ask again. Reset it, then rerun wmc:\n\n\
         \x20   {reset_cmd}",
        target.display()
    );
}
