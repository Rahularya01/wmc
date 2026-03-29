use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

fn default_media_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME env var not set");
    PathBuf::from(home)
        .join("Library/Group Containers/group.net.whatsapp.WhatsApp.shared/Message/Media")
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.2} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

const MEDIA_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "heic", "heif", "tiff", "tif", "mp4", "mov", "avi",
    "mkv", "wmv", "flv", "3gp", "m4v", "webm", "mp3", "aac", "m4a", "ogg", "wav", "flac", "opus",
    "amr",
];

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| MEDIA_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn collect_files(dir: &Path, files: &mut Vec<(PathBuf, u64)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            collect_files(&path, files)?;
        } else if meta.is_file() && is_media_file(&path) {
            files.push((path, meta.len()));
        }
    }
    Ok(())
}

fn get_media_cache_plist_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(
        "Library/Containers/net.whatsapp.WhatsApp/Data/tmp/MediaCache/diskcacherepository.plist",
    ))
}

fn get_db_path(media_path: &Path) -> Option<PathBuf> {
    let message_dir = media_path.parent()?;
    let container_dir = message_dir.parent()?;
    Some(container_dir.join("ChatStorage.sqlite"))
}

fn relative_db_path(media_path: &Path, file_path: &Path) -> Option<String> {
    let message_dir = media_path.parent()?;
    file_path
        .strip_prefix(message_dir)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

fn remove_empty_dirs(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.metadata()?.is_dir() {
            remove_empty_dirs(&path)?;
            let _ = fs::remove_dir(&path);
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut dry_run = false;
    let mut skip_confirm = false;
    let mut target_path: Option<PathBuf> = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" | "-n" => dry_run = true,
            "--yes" | "-y" => skip_confirm = true,
            "--path" => {
                i += 1;
                if i < args.len() {
                    target_path = Some(PathBuf::from(&args[i]));
                } else {
                    eprintln!("Error: --path requires an argument");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!(
                    "wmc - WhatsApp Media Cleaner\n"
                );
                println!("USAGE:");
                println!("  wmc [OPTIONS]\n");
                println!("OPTIONS:");
                println!("  -n, --dry-run      Show what would be deleted without deleting");
                println!("  -y, --yes          Skip confirmation prompt");
                println!("  --path <DIR>       Override target directory");
                println!("  -h, --help         Show this help message");
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {}", other);
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

    println!("Scanning: {}", target.display());

    let mut files: Vec<(PathBuf, u64)> = Vec::new();
    if let Err(e) = collect_files(&target, &mut files) {
        eprintln!("Error scanning directory: {}", e);
        std::process::exit(1);
    }

    if files.is_empty() {
        println!("No files found. Nothing to do.");
        return;
    }

    let total_size: u64 = files.iter().map(|(_, s)| s).sum();
    println!(
        "Found {} file(s) totalling {}",
        files.len(),
        format_bytes(total_size)
    );

    if dry_run {
        println!("\n[dry-run] Files that would be deleted:");
        for (path, size) in &files {
            println!("  {} ({})", path.display(), format_bytes(*size));
        }
        println!("\n[dry-run] No files were deleted.");
        return;
    }

    if !skip_confirm {
        print!(
            "\nDelete all {} file(s) ({})? [y/N] ",
            files.len(),
            format_bytes(total_size)
        );
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return;
        }
    }

    // Open WhatsApp database so we can clear local path references, which
    // lets WhatsApp show "tap to re-download" instead of "corrupted".
    // Open WhatsApp database so we can clear local path references, which
    // lets WhatsApp show "tap to re-download" instead of "corrupted".
    let conn: Option<Connection> = match get_db_path(&target) {
        Some(db_path) if db_path.exists() => match Connection::open(&db_path) {
            Ok(c) => match c.execute_batch("BEGIN") {
                Ok(_) => Some(c),
                Err(e) => {
                    eprintln!(
                        "Warning: could not start database transaction ({}). \
                         Close WhatsApp and retry, or deleted files may appear corrupted.",
                        e
                    );
                    None
                }
            },
            Err(e) => {
                eprintln!(
                    "Warning: could not open WhatsApp database ({}). \
                     Close WhatsApp and retry, or deleted files may appear corrupted.",
                    e
                );
                None
            }
        },
        _ => None,
    };

    let db_updated = if let Some(ref c) = conn {
        let message_dir = target.parent().unwrap();
        if let Ok(mut stmt) = c.prepare(
            "SELECT Z_PK, ZMEDIALOCALPATH FROM ZWAMEDIAITEM WHERE ZMEDIALOCALPATH IS NOT NULL",
        ) {
            let orphans: Vec<(i64, String)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map(|rows| {
                    rows.filter_map(|r| r.ok())
                        .filter(|(_, rel)| !message_dir.join(rel).exists())
                        .collect()
                })
                .unwrap_or_default();
            for (pk, _) in &orphans {
                let _ = c.execute(
                    "UPDATE ZWAMEDIAITEM SET ZMEDIALOCALPATH = NULL WHERE Z_PK = ?1",
                    params![pk],
                );
            }
            if !orphans.is_empty() {
                println!("Repaired {} orphaned database record(s)", orphans.len());
            }
        }

        for (path, _) in &files {
            if let Some(rel) = relative_db_path(&target, path) {
                let _ = c.execute(
                    "UPDATE ZWAMEDIAITEM SET ZMEDIALOCALPATH = NULL WHERE ZMEDIALOCALPATH = ?1",
                    params![rel],
                );
            }
        }
        c.execute_batch("COMMIT").is_ok()
    } else {
        false
    };

    let mut freed = 0u64;
    let mut errors = 0usize;

    for (path, size) in &files {
        match fs::remove_file(path) {
            Ok(_) => freed += size,
            Err(e) => {
                eprintln!("  Failed to delete {}: {}", path.display(), e);
                errors += 1;
            }
        }
    }

    if let Err(e) = remove_empty_dirs(&target) {
        eprintln!("Warning: error removing empty directories: {}", e);
    }

    println!(
        "\nDone. Deleted {}/{} file(s), freed {}{}",
        files.len() - errors,
        files.len(),
        format_bytes(freed),
        if errors > 0 {
            format!(" ({} error(s))", errors)
        } else {
            String::new()
        }
    );

    if let Some(p) = get_media_cache_plist_path() {
        if p.exists() {
            let _ = fs::remove_file(&p);
        }
    }

    if db_updated {
        restart_whatsapp();
    }
}

fn restart_whatsapp() {
    print!("Restarting WhatsApp...");
    io::stdout().flush().unwrap();

    std::process::Command::new("pkill")
        .args(["-x", "WhatsApp"])
        .status()
        .ok();

    std::thread::sleep(std::time::Duration::from_secs(2));

    match std::process::Command::new("open")
        .args(["-a", "WhatsApp"])
        .status()
    {
        Ok(s) if s.success() => println!(" done."),
        _ => println!("\nCould not relaunch WhatsApp automatically — please open it manually."),
    }
}
