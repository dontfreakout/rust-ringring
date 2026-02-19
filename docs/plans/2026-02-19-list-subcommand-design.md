# Design: `ringring list` Subcommand

**Date:** 2026-02-19

## Goal

Add `ringring list [--debug]` to enumerate all installed sound themes without needing Claude Code.

## CLI Interface

```
ringring list             # display_name + name per theme
ringring list --debug     # full manifest details per theme
```

- No arguments required
- `--debug` flag enables verbose output
- No new dependencies; no new files

## Entry Point Dispatch

Add `List { debug: bool }` variant to the `Cmd` enum. `parse_args` routes `args[1] == "list"` to it with `debug = args[2..].contains("--debug")`.

## `run_list` Logic

1. Resolve `sounds_dir` from `$HOME/.claude/sounds/`
2. Read directory entries; for each subdirectory containing a valid `manifest.json`:
   - Load and parse manifest (skip silently on parse failure)
   - Collect into `Vec`, sorted alphabetically by directory name
3. Output per theme:
   - **Default:** `<name>\t<display_name>` (tab-separated)
   - **`--debug`:** header `=== <name> (<display_name>) ===`, then each category with its sound files and optional `line` values
4. If `sounds_dir` is missing or empty: print nothing, exit 0

## Default Output Example

```
aoe2    Age of Empires II
icq     ICQ Classic
peon    Warcraft Peon
```

## `--debug` Output Example

```
=== peon (Warcraft Peon) ===
  greeting:
    - hello.wav  "Ready to work!"
  complete:
    - work_complete.wav
```

## Constraints

- No new files — all logic in `main.rs`
- No new dependencies
- Silent on missing/unreadable themes (consistent with project design)
- `run_list` exits 0 in all cases (no user-facing errors)
