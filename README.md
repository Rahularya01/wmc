# wmc

`wmc` stands for WhatsApp Media Cleaner. It is a small Rust CLI for cleaning downloaded WhatsApp media on macOS.

It scans the local WhatsApp media directory, deletes media files, clears stale media paths in the local WhatsApp SQLite database, and removes the local media cache so WhatsApp can re-download files when needed.

## Install

### With Homebrew (recommended)

```bash
brew install Rahularya01/tap/wmc
```

This installs a pre-built binary — no Rust required.

### From source

Requires Rust (`cargo`):

```bash
cargo install --git https://github.com/Rahularya01/wmc --tag v0.1.0
```

## Usage

```bash
wmc --dry-run
wmc --yes
wmc --path ~/Library/Group\ Containers/group.net.whatsapp.WhatsApp.shared/Message/Media
```

## Release Flow

1. Bump `version` in `Cargo.toml`.
2. Commit and push to `main`.
3. Tag and push the release:
   ```bash
   git tag v0.x.0 && git push origin v0.x.0
   ```
4. GitHub Actions builds the macOS binaries, creates the release, and updates the Homebrew formula automatically.

## Notes

- This tool is macOS-specific because it relies on WhatsApp Desktop container paths and uses `open` and `pkill`.
- If WhatsApp is open, database access may fail. Closing WhatsApp before running the tool gives the best results.
