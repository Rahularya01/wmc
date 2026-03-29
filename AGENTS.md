# AGENTS.md

Guidance for AI agents working in the wmc (WhatsApp Media Cleaner) repository.

## Commands

```bash
# Build
cargo build                  # Debug build
cargo build --release        # Release build (optimized)

# Run
cargo run -- [args]          # Run in debug mode (e.g., cargo run -- analyze)

# Code quality
cargo fmt                    # Format all code
cargo clippy                 # Run linter
cargo clippy -- -D warnings  # Fail on warnings (CI)

# Install
cargo install --path .       # Install locally from source

# Testing
# NOTE: There are no tests in this codebase yet.
# When tests are added, use:
#   cargo test                 # Run all tests
#   cargo test <name>          # Run specific test
```

## Code Style

### Formatting
- Use `cargo fmt` before committing
- Max line length: ~100 characters (rustfmt default)
- 4 spaces for indentation
- Trailing commas in multi-line structs/arrays

### Imports
Order: std → external crates → internal modules. Separate groups with blank lines.

```rust
use std::fs;
use std::path::Path;

use rusqlite::Connection;

use crate::config::media_cache_plist_path;
use crate::db;
```

### Naming
- Types (structs, enums, traits): `PascalCase` (e.g., `MediaEntry`, `CleanOutcome`)
- Functions, variables, modules: `snake_case` (e.g., `clean_media`, `total_size`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_CONTACTS`, `IMAGE_EXTENSIONS`)
- Generic parameters: single uppercase letters (e.g., `T`, `K`, `V`)

### Types
- Prefer explicit types on public struct fields
- Use `u64` for byte sizes, `usize` for counts
- Use `&'static str` for string constants
- Return `Option<T>` for fallible lookups, `Result<T, E>` for operations that can fail

### Error Handling
- Use `?` operator in functions returning `Result`
- Use `let _ = expr;` to explicitly ignore errors where appropriate
- Handle SQLite operations gracefully (DB may be locked/missing)
- Fail fast on critical errors (e.g., `expect("HOME env var not set")`)

### Documentation
- Public items must have `///` doc comments
- Document panics and error conditions
- Use backticks for code references in docs

```rust
/// Deletes files and NULLs corresponding database entries.
/// Restarts WhatsApp if the database was successfully updated.
pub fn clean_media(target: &Path, files: &[MediaEntry]) -> CleanOutcome {
```

### Architecture Patterns
- Module per feature: `cli/`, `media/`, `db/`, `tui/`
- Public API in `lib.rs`, binary entry in `main.rs`
- Types in `types.rs` within each module
- Config constants in `config.rs`

### Database
- Always wrap DB writes in transactions (`BEGIN`/`COMMIT`)
- Close read-only connections before opening read-write
- Use `params![]` macro for SQL parameter binding

### Unsafe Code
- No unsafe code allowed

## Release Process

Releases are automated via GitHub Actions on version tag push:

```bash
git tag v0.2.0
git push origin v0.2.0
```

Binaries are built for `aarch64-apple-darwin` and `x86_64-apple-darwin`.
