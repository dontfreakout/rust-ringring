# rust-ringring Design

Drop-in Rust replacement for the `~/.claude/ringring` bash hook script.

## Decisions

- **Architecture**: Single binary monolith. No daemon, no lib split.
- **Audio**: rodio crate (in-process decoding, WAV/MP3/OGG/FLAC, cross-platform)
- **Notifications**: notify-rust crate (D-Bus on Linux, native on macOS)
- **Compatibility**: Drop-in. Same stdin JSON protocol, same config.json and manifest.json schemas. Existing themes work unchanged.

## Project Structure

```
rust-ringring/
  Cargo.toml
  src/
    main.rs          # Entry point: read stdin, dispatch
    config.rs        # Config/theme resolution
    manifest.rs      # Manifest parsing, sound picking
    event.rs         # Hook event types and mapping to categories
    audio.rs         # rodio-based playback
    notify.rs        # Desktop notifications
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| serde + serde_json | JSON parsing (stdin, config, manifests) |
| rodio | Cross-platform audio playback |
| notify-rust | Cross-platform desktop notifications |
| rand | Random sound/theme selection |

## Flow

1. Read all of stdin into a string
2. Parse JSON: extract hook_event_name, session_id, source, notification_type
3. Resolve theme (env var > workspace pin > session random > config > legacy file > fallback "peon")
4. Load manifest from ~/.claude/sounds/{theme}/manifest.json
5. Map event to category (same logic as bash case statements)
6. Pick random sound from category
7. Resolve notification title/body (manifest overrides > hardcoded defaults)
8. Play sound via rodio
9. Send notification via notify-rust (unless skip_notify)

## SessionStart Deferred Startup

- On startup: write flag file /tmp/.claude-ringring-{session_id}, spawn thread that sleeps 1s then checks flag and plays if present
- On resume: delete the flag file to cancel pending startup sound
- Process waits for background thread before exiting

## Session Theme Persistence

On startup, write resolved theme to /tmp/.claude-theme-{session_id}. On subsequent invocations, read it back during theme resolution.

## Platform Abstraction

rodio and notify-rust handle platform differences internally:
- Audio: ALSA/PulseAudio/PipeWire on Linux, CoreAudio on macOS
- Notifications: D-Bus on Linux, native APIs on macOS

## Error Handling

Silent failures. If manifest missing, config broken, or audio fails: exit 0. A hook must never block Claude Code.
