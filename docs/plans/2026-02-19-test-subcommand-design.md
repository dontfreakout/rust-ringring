# Design: `ringring test` Subcommand

**Date:** 2026-02-19

## Goal

Add a `test` subcommand so users can play through all sounds in a theme without needing Claude Code to generate hook events.

## CLI Interface

```
ringring test <theme> [--category <cat>]
```

- `<theme>` — required; theme name resolved to `~/.claude/sounds/<theme>/`
- `--category <cat>` — optional; restrict playback to one category
- No external arg-parsing crate; use `std::env::args()` directly

## Entry Point Dispatch

In `main.rs`, before the stdin-hook path:

```
args[1] == "test"  →  run_test(args)
otherwise          →  existing run() reading from stdin
```

`run_test` returns `Result` like `run`; outer `main` swallows errors identically.

## Test Execution (`run_test`)

1. Resolve `sounds_dir` from `$HOME/.claude/sounds/`
2. Load `manifest.json` from theme dir; exit cleanly if missing
3. Collect target categories: all manifest keys, or the one specified by `--category`
4. For each category (sorted for determinism), for each sound in declaration order:
   - Print `[category] filename` to stdout
   - Play via `audio::play_sound` (blocking)
5. If `--category` names a missing category: print error to stderr, exit 1

## Constraints

- No new files — all logic in `main.rs`
- No new dependencies
- Silent failure behavior preserved for hook mode; test mode may print errors
