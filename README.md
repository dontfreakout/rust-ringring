# rust-ringring

Sound and notification companion for [Claude Code](https://claude.ai/code). Hooks into Claude Code events to play themed audio cues and send desktop notifications, so you know what's happening without watching the terminal.

## Features

- Plays sounds on Claude Code events (session start, task complete, permission requests, notifications)
- Desktop notifications with icon support (GTK notification stacking on GNOME, freedesktop fallback)
- Themed sound packs with per-category sounds and random selection
- Theme resolution chain: env var, workspace pin, session cache, random pool, config, legacy file, fallback
- Deferred startup sound with resume cancellation
- Silent failures — never blocks Claude Code
- Single static binary, size-optimized with LTO

## Installation

### Build from source

Requires Rust 1.85+ (2024 edition).

```bash
git clone https://github.com/YOUR_USER/rust-ringring.git
cd rust-ringring
make install
```

This builds a release binary and copies it to `~/.claude/rust-ringring`.

### Configure the hook

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": [{ "type": "command", "command": "echo '$HOOK_EVENT' | ~/.claude/rust-ringring" }],
    "Notification": [{ "type": "command", "command": "echo '$HOOK_EVENT' | ~/.claude/rust-ringring" }],
    "SessionStart": [{ "type": "command", "command": "echo '$HOOK_EVENT' | ~/.claude/rust-ringring" }]
  }
}
```

## Sound Themes

Themes live in `~/.claude/sounds/<theme-name>/` with this structure:

```
~/.claude/sounds/
├── config.json
└── peon/
    ├── manifest.json
    └── sounds/
        ├── greeting.wav
        ├── complete.wav
        └── ...
```

### config.json

```json
{
  "theme": "peon",
  "mode": "random",
  "random_pool": ["peon", "aoe2", "icq"],
  "workspaces": {
    "/home/user/serious-project": "office"
  }
}
```

| Field | Description |
|-------|-------------|
| `theme` | Default theme name |
| `mode` | Set to `"random"` to pick from `random_pool` each session |
| `random_pool` | List of theme names for random selection |
| `workspaces` | Map of directory path to theme name (workspace pinning) |

### manifest.json

```json
{
  "name": "peon",
  "display_name": "Warcraft Peon",
  "categories": {
    "greeting": {
      "title": "Ready to work",
      "sounds": [
        { "file": "ready.wav", "line": "Ready to work!" },
        { "file": "work.wav", "line": "Work work." }
      ]
    },
    "complete": {
      "title": "Job's done",
      "body": "Something need doing?",
      "sounds": [
        { "file": "jobsdone.wav" }
      ]
    }
  }
}
```

**Categories** used by hook events: `greeting`, `complete`, `permission`, `annoyed`, `acknowledge`, `resource_limit`.

### Theme resolution priority

1. `CLAUDE_SOUND_THEME` environment variable
2. Workspace pin from `config.json` `workspaces` map
3. Session cache (`/tmp/.claude-theme-{session_id}`)
4. Random from `random_pool` (if `mode` is `"random"`)
5. `config.json` `theme` field
6. Legacy `~/.claude/sounds/theme` file
7. Fallback: `"peon"`

## Hook Events

| Hook Event | Category | Notification |
|------------|----------|--------------|
| `SessionStart` (startup) | `greeting` | No (sound only, 1s delay) |
| `SessionStart` (resume) | — | No (cancels pending startup sound) |
| `Stop` | `complete` | Yes |
| `PermissionRequest` | `permission` | No (sound only) |
| `Notification` (permission_prompt) | `permission` | Yes |
| `Notification` (idle_prompt) | `annoyed` | Yes |
| `Notification` (auth_success) | `acknowledge` | Yes |
| `Notification` (elicitation_dialog) | `permission` | Yes |

## Building

```bash
make              # release build
make test         # run tests
make dist         # build and package for current platform
make cross TARGET=aarch64-unknown-linux-gnu  # cross-compile
```

## License

MIT
