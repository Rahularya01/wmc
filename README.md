<p align="center">
  <img src="assets/logo.png" alt="WMC Logo" width="150">
</p>

# wmc — WhatsApp Media Cleaner

`wmc` is a macOS CLI tool that reclaims storage taken up by WhatsApp media. It deletes downloaded files, clears stale references from WhatsApp's database, and removes the local media cache — so WhatsApp shows "tap to re-download" instead of broken media.

---

## Install

### With Homebrew (recommended)

```bash
brew install Rahularya01/tap/wmc
```

No Rust required. Works on Apple Silicon and Intel Macs.

### From source

Requires [Rust](https://rustup.rs):

```bash
cargo install --git https://github.com/Rahularya01/wmc --tag v0.3.0
```

---

## Commands

### `wmc`

Open the interactive terminal UI built with `tuirealm`. This is now the default when you run `wmc` with no subcommand.

```bash
wmc
```

Inside the UI you can:

- review total usage by media type
- inspect the heaviest contacts and largest files
- preview a clean before deleting anything
- rescan after WhatsApp downloads new media

---

### `wmc analyze`

See how much storage WhatsApp media is taking up in a plain non-interactive report.

```bash
wmc analyze
```

Example output:

```
Scanning: ~/Library/Group Containers/.../Message/Media

  Images      42 file(s)   18.30 MB
  Videos       8 file(s)   412.10 MB
  Audio       15 file(s)   6.20 MB
  ──────────────────────────────────────
  Total       65 file(s)   436.60 MB

Run `wmc` for the interactive UI or `wmc clean` to free up this space.
```

---

### `wmc clean`

Delete all WhatsApp media files. You'll be asked to confirm before anything is deleted.

```bash
wmc clean
```

Skip the confirmation prompt:

```bash
wmc clean --yes
```

Preview what would be deleted without actually deleting anything:

```bash
wmc clean --dry-run
```

---

### Global options

| Option | Description |
|---|---|
| `--path <DIR>` | Use a custom media directory instead of the default |
| `-h, --help` | Show help |

---

## Notes

- **Close WhatsApp before running `wmc clean`** — if WhatsApp is open, the database may be locked and deleted files could appear as corrupted instead of re-downloadable.
- After cleaning, `wmc` will restart WhatsApp automatically if it was able to update the database.
- This tool is macOS-only — it relies on WhatsApp Desktop's container paths and macOS system commands.

---

## Release Flow

1. Bump `version` in `Cargo.toml`.
2. Commit and push to `main`.
3. Tag and push the release:
   ```bash
   git tag v0.x.0 && git push origin v0.x.0
   ```
4. GitHub Actions builds the macOS binaries, creates the release, and updates the Homebrew formula automatically.
