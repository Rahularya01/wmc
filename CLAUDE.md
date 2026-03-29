# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build               # Debug build
cargo build --release     # Release build
cargo run -- [args]       # Run in debug mode
cargo fmt                 # Format code
cargo clippy              # Lint
cargo install --path .    # Install locally
```

There are no tests. Releasing is handled by GitHub Actions on version tag push (e.g., `git tag v0.2.0 && git push origin v0.2.0`), producing binaries for `aarch64-apple-darwin` and `x86_64-apple-darwin`.

## Architecture

**wmc** is a macOS CLI tool that reclaims storage by deleting downloaded WhatsApp media files, updating WhatsApp's SQLite database to NULL out deleted file references, clearing the media cache plist, and optionally restarting WhatsApp.

### Modules

- **`cli/`** — Manual argument parsing (`args.rs`) and non-interactive subcommand implementations (`commands.rs`). No external CLI library.
- **`media/`** — File scanning (`scanner.rs`), deletion + DB update logic (`cleaner.rs`), and shared data types (`types.rs`).
- **`db/`** — SQLite helpers: path resolution, read-only contact lookup, and read-write deletion updates.
- **`tui/`** — Terminal UI using `tuirealm`. `app.rs` owns the event loop; `dashboard.rs` handles all rendering (477 lines) and key handling; `types.rs` defines `AppId`, `AppMsg`, `UiAction`.
- **`config.rs`** — Hardcoded file extensions, default paths, and UI limits (`MAX_CONTACTS=8`, `MAX_FILES=10`).
- **`utils.rs`** — `format_bytes()` only.

### Three entry points (all via `main.rs`)

| Subcommand | Behavior |
|---|---|
| `wmc` / `wmc ui` | Interactive TUI |
| `wmc analyze` | Print storage report, exit |
| `wmc clean` | Batch delete with `--yes`/`--dry-run` flags |

### Core data flow

1. `media::scanner::scan_media(path)` — walks filesystem, categorizes by extension, joins against `ChatStorage.sqlite` (read-only) for contact attribution → `ScanReport`
2. `media::cleaner::clean_media(files, db_path, dry_run)` — opens DB read-write, wraps in a transaction: (a) NULLs `ZWAMEDIAITEM.ZMEDIALOCALPATH` for deleted files, (b) repairs orphaned rows pointing to already-missing files; then deletes files, removes empty dirs, clears cache plist, restarts WhatsApp → `CleanOutcome`

### WhatsApp database schema (relevant tables)

```
ZWAMEDIAITEM  → ZMEDIALOCALPATH (relative to Message/ dir), ZMESSAGE FK
ZWAMESSAGE    → ZCHATSESSION FK
ZWACHATSESSION → ZPARTNERNAME, ZCONTACTJID
```

DB is at `../ChatStorage.sqlite` relative to the media directory. Media cache plist cleared: `~/Library/Containers/net.whatsapp.WhatsApp/Data/tmp/MediaCache/diskcacherepository.plist`.

### TUI event loop

Poll keyboard at 50ms, dispatch to `Dashboard` which owns all UI state. Delete requires two Enter presses (arm then confirm). Keys: `↑↓`/`jk` navigate, `r` rescan, `p` preview, `q`/Esc quit.
