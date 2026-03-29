# wmc

`wmc` stands for WhatsApp Media Cleaner. It is a small Rust CLI for cleaning downloaded WhatsApp media on macOS.

It scans the local WhatsApp media directory, deletes media files, clears stale media paths in the local WhatsApp SQLite database, and removes the local media cache so WhatsApp can re-download files when needed.

## Install

### From source

```bash
cargo install --path .
```

### From a Git tag

```bash
cargo install --git https://github.com/Rahularya01/wmc --tag v0.1.0
```

### With Homebrew

After you publish a tap:

```bash
brew install rahularya/tap/wmc
```

## Usage

```bash
wmc --dry-run
wmc --yes
wmc --path ~/Library/Group\ Containers/group.net.whatsapp.WhatsApp.shared/Message/Media
```

## Release Flow

1. Bump `version` in `Cargo.toml`.
2. Commit and tag the release, for example `v0.1.0`.
3. Push the commit and tag to GitHub.
4. Create a GitHub release for that tag.
5. Update the Homebrew formula in your tap with the new tarball SHA256.

## Homebrew Formula

A formula template is included at `packaging/homebrew/wmc.rb`.

## Notes

- This tool is macOS-specific because it relies on WhatsApp Desktop container paths and uses `open` and `pkill`.
- If WhatsApp is open, database access may fail. Closing WhatsApp before running the tool gives the best results.
