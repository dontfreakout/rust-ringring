---
allowed-tools: Bash(ringring *)
argument-hint: [status | theme <name> | mute | unmute | mode <random|sequential> | list | test <theme> [--category <cat>]]
---

You control the `ringring` sound notification system. The user wants to manage sounds for their Claude Code session.

## Finding the session ID

Run `ringring detect-session` to get the session ID for the current Claude Code instance. It auto-detects by walking the process tree to find the parent Claude Code process. Store the result in a variable for subsequent commands.

## Available commands

Run these via Bash:

- `ringring status [session_id]` — show current mode, theme, mute state (auto-detects session if omitted)
- `ringring session <session_id> theme <name>` — change theme for this session
- `ringring session <session_id> mute` — silence sounds for this session
- `ringring session <session_id> unmute` — re-enable sounds for this session
- `ringring mute` — mute current session (auto-detects)
- `ringring unmute` — unmute current session (auto-detects)
- `ringring mode <random|sequential>` — change theme rotation mode
- `ringring list` — list available themes
- `ringring test <theme>` — preview all sounds in a theme (blocking)
- `ringring test <theme> --category <cat>` — preview one category
- `ringring detect-session` — print the session ID for this Claude Code instance

## How to respond

1. If the user provided arguments (e.g. `/ringring mute`), execute the matching command immediately. After any change, confirm what was done in one line.
2. If no arguments, run `ringring status` and print its output directly (it outputs a markdown table), then list available commands:
   - `/ringring theme <name>` — change theme for this session
   - `/ringring mute` / `/ringring unmute` — toggle sounds
   - `/ringring mode <random|sequential>` — change rotation mode
   - `/ringring list` — list available themes
   - `/ringring test <theme>` — preview a theme's sounds

The user's request: $ARGUMENTS
