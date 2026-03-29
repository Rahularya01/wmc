use std::io::{self, Write};
use std::path::Path;

use crate::media::types::{ContactBreakdown, ScanReport};
use crate::media::{clean_media, scan_media};
use crate::utils::format_bytes;

// ── Help text ─────────────────────────────────────────────────────────────────

pub fn print_help() {
    println!("wmc - WhatsApp Media Cleaner\n");
    println!("USAGE:");
    println!("  wmc [ui] [OPTIONS]");
    println!("  wmc <COMMAND> [OPTIONS]\n");
    println!("COMMANDS:");
    println!("  ui               Open the interactive terminal UI");
    println!("  analyze          Show how much storage WhatsApp media is using");
    println!("  clean            Delete WhatsApp media and free up storage");
    println!("  clean --dry-run  Preview what would be deleted without deleting\n");
    println!("OPTIONS:");
    println!("  -y, --yes        Skip confirmation prompt (use with clean)");
    println!("  --path <DIR>     Override the target media directory");
    println!("  -h, --help       Show this help message");
}

// ── Shared report printing ────────────────────────────────────────────────────

pub fn print_contact_breakdown(breakdown: &[ContactBreakdown]) {
    if breakdown.is_empty() {
        return;
    }

    let max_label = breakdown
        .iter()
        .map(|item| item.label.len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!("\nPer-contact breakdown:\n");
    println!(
        "  {:<width$}  {:>6}   {:>10}",
        "Contact",
        "Files",
        "Size",
        width = max_label
    );
    println!("  {}", "-".repeat(max_label + 22));
    for item in breakdown {
        println!(
            "  {:<width$}  {:>6}   {:>10}",
            item.label,
            item.file_count,
            format_bytes(item.total_size),
            width = max_label
        );
    }
}

pub fn print_report(target: &Path, report: &ScanReport) {
    println!("Scanning: {}\n", target.display());

    if report.total_files == 0 {
        println!("No media files found.");
        return;
    }

    for category in &report.categories {
        println!(
            "  {:<6} {:>6} file(s)   {}",
            category.label,
            category.file_count,
            format_bytes(category.total_size)
        );
    }
    println!("  {}", "-".repeat(38));
    println!(
        "  Total  {:>6} file(s)   {}",
        report.total_files,
        format_bytes(report.total_size)
    );
    println!("\nRun `wmc` for the interactive UI or `wmc clean` to free up this space.");
}

// ── Subcommand handlers ───────────────────────────────────────────────────────

pub fn cmd_analyze(target: &Path) {
    match scan_media(target) {
        Ok(report) => {
            print_report(target, &report);
            print_contact_breakdown(&report.contact_breakdown);
        }
        Err(error) => {
            eprintln!("Error scanning directory: {}", error);
            std::process::exit(1);
        }
    }
}

pub fn cmd_clean(target: &Path, skip_confirm: bool, dry_run: bool) {
    let report = match scan_media(target) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("Error scanning directory: {}", error);
            std::process::exit(1);
        }
    };

    println!("Scanning: {}", target.display());

    if report.total_files == 0 {
        println!("No media files found. Nothing to do.");
        return;
    }

    println!(
        "Found {} file(s) totalling {}",
        report.total_files,
        format_bytes(report.total_size)
    );
    print_contact_breakdown(&report.contact_breakdown);

    if dry_run {
        println!("\n[dry-run] Files that would be deleted:");
        for entry in &report.files {
            println!("  {} ({})", entry.path.display(), format_bytes(entry.size));
        }
        println!("\n[dry-run] No files were deleted.");
        return;
    }

    if !skip_confirm {
        print!(
            "\nDelete all {} file(s) ({})? [y/N] ",
            report.total_files,
            format_bytes(report.total_size)
        );
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return;
        }
    }

    let outcome = clean_media(target, &report.files);
    println!(
        "\nDone. Deleted {}/{} file(s), freed {}{}",
        outcome.deleted_files,
        outcome.total_files,
        format_bytes(outcome.freed_bytes),
        if outcome.errors > 0 {
            format!(" ({} error(s))", outcome.errors)
        } else {
            String::new()
        }
    );
    if outcome.repaired_orphans > 0 {
        println!(
            "Repaired {} orphaned database record(s).",
            outcome.repaired_orphans
        );
    }
    if !outcome.db_updated {
        println!("Database was not updated. Close WhatsApp and retry if media appears corrupted.");
    }
}
