# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Drop-in Rust replacement for the `~/.claude/ringring` bash hook script. Reads Claude Code hook events from stdin as JSON, resolves a sound theme, plays audio via rodio, and sends desktop notifications via notify-rust. Uses Rust 2024 edition.

## Build & Test Commands

```bash
cargo build                    # debug build
cargo build --release          # release build (stripped, LTO, size-optimized)
cargo test                     # run all tests
cargo test -- event::tests     # run tests in a specific module
cargo test test_name           # run a single test by name
```

No linter or formatter config exists yet; use `cargo clippy` and `cargo fmt`.

## Architecture

Single binary, no library crate. Stdin JSON → event mapping → theme resolution → manifest lookup → sound playback + notification.

**Modules:**
- `event.rs` — Deserializes `HookInput` from stdin JSON, maps hook events (`Stop`, `PermissionRequest`, `Notification`, `SessionStart`) to `EventAction` (category, title, body, skip_notify)
- `config.rs` — `Config` (from `~/.claude/sounds/config.json`) and `ThemeResolver` with priority chain: env var `CLAUDE_SOUND_THEME` → workspace pin → session cache (`/tmp/.claude-theme-{session_id}`) → random pool → config theme → legacy theme file → fallback "peon"
- `manifest.rs` — `Manifest` (from `{theme_dir}/manifest.json`) with categories containing sounds; `pick_sound` selects randomly; `category_text` extracts title/body overrides
- `audio.rs` — Thin rodio wrapper, `play_sound` blocks until playback completes
- `notify.rs` — Thin notify-rust wrapper, silent failure
- `main.rs` — Orchestration + `SessionStart` deferred startup logic (flag file + 1s delay thread to allow resume cancellation)

## Key Design Constraints

- **Silent failures everywhere.** A hook must never block Claude Code. All errors are swallowed; the binary always exits 0.
- **Drop-in compatibility.** Same stdin JSON protocol, same config.json/manifest.json schemas as the bash original. Existing themes work unchanged.
- **Runtime data lives in `~/.claude/sounds/`** — config.json, theme directories with manifest.json + sounds/ subdirs, optional icon at `~/.claude/clawd.png`.
- **Temp files** for session state: `/tmp/.claude-ringring-{session_id}` (startup flag), `/tmp/.claude-theme-{session_id}` (session theme cache).
