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
git clone https://github.com/dontfreakout/rust-ringring.git
cd rust-ringring
cargo build --release
```

### Install binary and hooks

```bash
ringring install
```

This copies the binary to `~/.local/bin/ringring`, registers hook entries in `~/.claude/settings.json`, and installs the `/ringring` slash command to `~/.claude/commands/`. The command is idempotent — safe to re-run without duplicating hooks.

## Usage

### Inside Claude Code

Use the `/ringring` slash command to control sounds from within a session:

```
/ringring                 # show current status and available commands
/ringring theme peon      # switch this session to the peon theme
/ringring mute            # silence sounds for this session
/ringring unmute          # re-enable sounds
/ringring mode sequential # change theme rotation mode
/ringring list            # list available themes
```

The slash command automatically detects your session ID and runs without permission prompts.

### CLI

#### List installed themes

```bash
ringring list            # name and display name
ringring list --debug    # full breakdown of categories and sounds
```

#### Test a theme

```bash
ringring test peon                        # play all sounds in all categories
ringring test peon --category greeting    # play only greeting sounds
```

#### Session control

```bash
ringring status <session_id>                      # show status for a session
ringring session <session_id> theme <name>        # change session theme
ringring session <session_id> mute                # mute session
ringring session <session_id> unmute              # unmute session
```

#### Mode and config

```bash
ringring mode random       # random theme per session
ringring mode sequential   # rotate through pool in order
```

#### Install a theme from zip

```bash
ringring theme install /path/to/theme.zip
ringring theme install https://example.com/mytheme.zip
ringring theme install --force /path/to/theme.zip   # overwrite existing
```

The zip must contain a single top-level directory with a `manifest.json` inside it.

## Sound Themes

Themes live in the data directory with this structure. The data directory is resolved as: `$XDG_DATA_HOME/ringring` (if it contains data), then `~/.claude/sounds/` (legacy fallback), then `~/.local/share/ringring` (default).

```
<data-dir>/
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
| `mode` | `"random"` picks randomly from pool each session; `"sequential"` rotates in order |
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
4. Pick from `random_pool` (random or sequential depending on `mode`)
5. `config.json` `theme` field
6. Legacy `~/.claude/sounds/theme` file (plain text)
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
```

## License

MIT
