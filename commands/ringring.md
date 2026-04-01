---
allowed-tools: Bash(ringring *)
---

You control the `ringring` sound notification system. The user wants to manage sounds for their Claude Code session.

## Finding the session ID

Extract the session ID from the transcript path visible in your context. It's the UUID in the path, e.g. for `/home/user/.claude/projects/.../2a9d54d3-01a1-4539-bc16-0e0cfb387d42.jsonl` the session ID is `2a9d54d3-01a1-4539-bc16-0e0cfb387d42`.

## Available commands

Run these via Bash:

- `ringring status <session_id>` — show current mode, theme, mute state
- `ringring session <session_id> theme <name>` — change theme for this session
- `ringring session <session_id> mute` — silence sounds for this session
- `ringring session <session_id> unmute` — re-enable sounds for this session
- `ringring mode <random|sequential>` — change theme rotation mode
- `ringring list` — list available themes
- `ringring test <theme>` — preview all sounds in a theme (blocking)
- `ringring test <theme> --category <cat>` — preview one category

## How to respond

1. If the user provided arguments (e.g. `/ringring mute`), execute the matching command immediately using the session ID from context. After any change, confirm what was done in one line.
2. If no arguments, run `ringring status <session_id>` and present the result as a table, then list available commands the user can run:
   - `/ringring theme <name>` — change theme for this session
   - `/ringring mute` / `/ringring unmute` — toggle sounds
   - `/ringring mode <random|sequential>` — change rotation mode
   - `/ringring list` — list available themes
   - `/ringring test <theme>` — preview a theme's sounds

The user's request: $ARGUMENTS
