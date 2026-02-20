# Self-Installer Design

**Date:** 2026-02-20

## Overview

Add `ringring install` and `ringring theme install` subcommands, and migrate all
hardcoded `~/.claude/sounds/` paths to XDG-compliant directories configurable via
environment variables.

## Directory Layout

| Purpose | Path |
|---------|------|
| Binary  | `~/.local/bin/ringring` |
| Config  | `config_dir()/config.json` |
| Data (themes) | `data_dir()/<theme-name>/` |

## Path Resolution (`paths.rs`)

Two public functions resolve directories at runtime:

```
config_dir() -> PathBuf
  1. $XDG_CONFIG_HOME/ringring      (all platforms, if set)
  2. ~/Library/Application Support/ringring  (macOS)
  3. ~/.config/ringring             (Linux / other)

data_dir() -> PathBuf
  1. $XDG_DATA_HOME/ringring        (all platforms, if set)
  2. ~/Library/Application Support/ringring  (macOS)
  3. ~/.local/share/ringring        (Linux / other)
```

All existing references to `~/.claude/sounds/` are replaced by `data_dir()`.
`Config::load` and `ThemeResolver` are updated to accept the result of `data_dir()`.

## New Subcommands

### `ringring install`

1. Create `config_dir()` and `data_dir()` if they don't exist.
2. Copy the running binary (`std::env::current_exe()`) to `~/.local/bin/ringring`.
3. Read `~/.claude/settings.json` (or start from `{}`), merge hook entries for
   `SessionStart`, `Stop`, `Notification`, and `PermissionRequest`, each pointing
   to `ringring`. Existing unrelated fields are preserved.
4. Write settings back atomically (write to `.tmp`, rename).
5. Print a summary of actions taken.
6. Exit non-zero on any failure.

Hook entry shape added per event:
```json
{"hooks": [{"type": "command", "command": "ringring"}]}
```

A ringring entry is only appended if no entry with `"command": "ringring"` already
exists for that event.

### `ringring theme install [--force] <path|url>`

1. If the argument starts with `http://` or `https://`, download zip to a temp file
   via `ureq`.
2. Open zip, assert exactly one top-level directory — its name becomes the theme name.
3. Validate `manifest.json` exists inside the theme directory.
4. Copy the extracted tree to `data_dir()/<theme-name>/`.
5. Error if destination already exists, unless `--force` is passed.
6. Exit non-zero on any failure.

## Subcommand Surface

```
ringring                                  # hook mode (stdin JSON)
ringring install                          # self-install binary + hooks
ringring theme install <path|url>         # install theme from local or remote zip
ringring theme install --force <path|url> # overwrite existing theme
ringring list [--debug]                   # list installed themes
ringring test <theme> [--category c]      # play sounds from a theme
```

## New Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ureq` | `2` | Synchronous HTTP download for remote zips |
| `zip`  | `2` | Zip extraction |

## Error Handling

- `ringring install` and `ringring theme install`: print to stderr, exit non-zero on failure.
- All existing hook-mode commands (`Hook`, `List`, `Test`): unchanged silent-failure contract.

## Modules Affected

| Module | Change |
|--------|--------|
| `paths.rs` | New — XDG/macOS path resolution |
| `install.rs` | New — install and theme-install logic |
| `config.rs` | Replace `sounds_dir` references with `data_dir()` |
| `main.rs` | Add `Install` and `ThemeInstall` `Cmd` variants; pass `data_dir()` and `config_dir()` |
| `audio.rs`, `event.rs`, `manifest.rs`, `notify.rs` | No changes |
