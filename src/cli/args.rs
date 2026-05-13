use std::path::PathBuf;

use clap::{Args, Parser};

use crate::config::default_media_path;

/// WhatsApp Media Cleaner
#[derive(Parser)]
#[command(name = "wmc")]
#[command(about = "CLI to clean downloaded WhatsApp media on macOS")]
#[command(version)]
pub struct CliArgs {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    /// Override the target media directory
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

impl CliArgs {
    /// Returns the effective target path (explicit `--path` or default).
    pub fn target(&self) -> PathBuf {
        self.path.clone().unwrap_or_else(default_media_path)
    }
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    /// Open the interactive terminal UI (default)
    Ui,
    /// Show how much storage WhatsApp media is using
    Analyze,
    /// Delete WhatsApp media and free up storage
    Clean(CleanArgs),
}

#[derive(Args)]
pub struct CleanArgs {
    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
    /// Preview what would be deleted without deleting
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}
