# rust-ringring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Drop-in Rust replacement for ~/.claude/ringring bash hook script with cross-platform audio and notifications.

**Architecture:** Single binary reads JSON from stdin, resolves sound theme, picks a random sound, plays it via rodio, and sends desktop notifications via notify-rust. Same config/manifest format as the bash version.

**Tech Stack:** Rust, serde_json, rodio 0.21, notify-rust 4, rand

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "rust-ringring"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rodio = "0.21"
notify-rust = "4"
rand = "0.9"

[profile.release]
strip = true
lto = true
opt-level = "s"
```

**Step 2: Create minimal src/main.rs**

```rust
fn main() {
    if let Err(_) = run() {
        // Silent failure — hooks must never block Claude Code
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully, downloads dependencies.

**Step 4: Commit**

```bash
git add Cargo.toml src/main.rs
git commit -m "feat: project scaffolding with dependencies"
```

---

### Task 2: Event Types (event.rs)

**Files:**
- Create: `src/event.rs`
- Modify: `src/main.rs` (add mod)

**Step 1: Write tests for event parsing and category mapping**

Create `src/event.rs` with tests at the bottom:

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HookInput {
    #[serde(default = "default_unknown")]
    pub hook_event_name: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
}

fn default_unknown() -> String {
    "unknown".to_string()
}

/// Result of mapping a hook event to display/sound parameters.
pub struct EventAction {
    pub category: Option<String>,
    pub title: String,
    pub body: String,
    pub skip_notify: bool,
    /// For SessionStart: "startup", "resume", or other
    pub session_start_type: Option<String>,
}

pub fn map_event(input: &HookInput) -> EventAction {
    match input.hook_event_name.as_str() {
        "SessionStart" => {
            let source_type = input.source.as_deref().unwrap_or("unknown");
            let session_start_type = Some(source_type.to_string());
            match source_type {
                "startup" | "resume" => EventAction {
                    category: Some("greeting".to_string()),
                    title: String::new(),
                    body: String::new(),
                    skip_notify: true,
                    session_start_type,
                },
                _ => EventAction {
                    category: None,
                    title: String::new(),
                    body: String::new(),
                    skip_notify: true,
                    session_start_type,
                },
            }
        }
        "PermissionRequest" => EventAction {
            category: Some("permission".to_string()),
            title: "Potřebuju povolení".to_string(),
            body: "Something need doing?".to_string(),
            skip_notify: true,
            session_start_type: None,
        },
        "Stop" => EventAction {
            category: Some("complete".to_string()),
            title: "Hotovo".to_string(),
            body: "Okie dokie.".to_string(),
            skip_notify: false,
            session_start_type: None,
        },
        "Notification" => {
            let nt = input.notification_type.as_deref().unwrap_or("unknown");
            match nt {
                "permission_prompt" => EventAction {
                    category: Some("permission".to_string()),
                    title: "Chtěl bych trochu pozornosti".to_string(),
                    body: "Hmm?".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                "idle_prompt" => EventAction {
                    category: Some("annoyed".to_string()),
                    title: "Čekám na tebe".to_string(),
                    body: "Nudím se, pojď makat.".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                "auth_success" => EventAction {
                    category: Some("acknowledge".to_string()),
                    title: "Přihlášení úspěšné".to_string(),
                    body: "Be happy to.".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                "elicitation_dialog" => EventAction {
                    category: Some("permission".to_string()),
                    title: "Mám otázku".to_string(),
                    body: "What you want?".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                _ => EventAction {
                    category: Some("greeting".to_string()),
                    title: "Chtěl bych trochu pozornosti".to_string(),
                    body: "Yes?".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
            }
        }
        _ => EventAction {
            category: Some("resource_limit".to_string()),
            title: "Neznámá událost".to_string(),
            body: "Why not?".to_string(),
            skip_notify: false,
            session_start_type: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> HookInput {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn stop_maps_to_complete() {
        let input = parse(r#"{"hook_event_name": "Stop", "session_id": "abc"}"#);
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("complete"));
        assert!(!action.skip_notify);
    }

    #[test]
    fn permission_request_skips_notify() {
        let input = parse(r#"{"hook_event_name": "PermissionRequest"}"#);
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("permission"));
        assert!(action.skip_notify);
    }

    #[test]
    fn session_start_startup() {
        let input = parse(r#"{"hook_event_name": "SessionStart", "source": "startup"}"#);
        let action = map_event(&input);
        assert_eq!(action.session_start_type.as_deref(), Some("startup"));
        assert!(action.skip_notify);
    }

    #[test]
    fn notification_idle_prompt() {
        let input = parse(
            r#"{"hook_event_name": "Notification", "notification_type": "idle_prompt"}"#,
        );
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("annoyed"));
    }

    #[test]
    fn notification_unknown_type_defaults_to_greeting() {
        let input = parse(
            r#"{"hook_event_name": "Notification", "notification_type": "some_new_thing"}"#,
        );
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("greeting"));
    }

    #[test]
    fn unknown_event_maps_to_resource_limit() {
        let input = parse(r#"{"hook_event_name": "SomeFutureEvent"}"#);
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("resource_limit"));
    }
}
```

**Step 2: Add module to main.rs**

Add to `src/main.rs`:
```rust
mod event;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All 6 tests pass.

**Step 4: Commit**

```bash
git add src/event.rs src/main.rs
git commit -m "feat: event parsing and category mapping with tests"
```

---

### Task 3: Config and Theme Resolution (config.rs)

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add mod)

**Step 1: Write config.rs with theme resolution and tests**

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub random_pool: Vec<String>,
    #[serde(default)]
    pub workspaces: HashMap<String, String>,
}

impl Config {
    pub fn load(sounds_dir: &Path) -> Self {
        let path = sounds_dir.join("config.json");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
}

pub struct ThemeResolver<'a> {
    pub sounds_dir: &'a Path,
    pub config: &'a Config,
    pub session_id: &'a str,
    pub cwd: String,
}

impl<'a> ThemeResolver<'a> {
    /// Resolve theme using the priority chain:
    /// 1. CLAUDE_SOUND_THEME env var
    /// 2. Workspace pin (config.json workspaces map)
    /// 3. Session cache (/tmp/.claude-theme-{session_id})
    /// 4. Random from pool (if mode=random)
    /// 5. config.json "theme" field
    /// 6. Legacy ~/.claude/sounds/theme file
    /// 7. Fallback "peon"
    pub fn resolve(&self) -> String {
        // 1. Env var
        if let Ok(theme) = std::env::var("CLAUDE_SOUND_THEME") {
            if !theme.is_empty() {
                return theme;
            }
        }

        // 2. Workspace pin
        if let Some(theme) = self.config.workspaces.get(&self.cwd) {
            if !theme.is_empty() {
                return theme.clone();
            }
        }

        // 3. Session cache
        if !self.session_id.is_empty() {
            let session_file = self.session_theme_file();
            if let Ok(cached) = fs::read_to_string(&session_file) {
                let cached = cached.trim().to_string();
                if !cached.is_empty() {
                    return cached;
                }
            }
        }

        // 3b. Random from pool
        if self.config.mode.as_deref() == Some("random") && !self.config.random_pool.is_empty() {
            use rand::Rng;
            let idx = rand::rng().random_range(0..self.config.random_pool.len());
            return self.config.random_pool[idx].clone();
        }

        // 4. Config theme
        if let Some(ref theme) = self.config.theme {
            if !theme.is_empty() {
                return theme.clone();
            }
        }

        // 5. Legacy theme file
        let legacy = self.sounds_dir.join("theme");
        if let Ok(content) = fs::read_to_string(&legacy) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }

        // 6. Fallback
        "peon".to_string()
    }

    pub fn session_theme_file(&self) -> PathBuf {
        PathBuf::from(format!("/tmp/.claude-theme-{}", self.session_id))
    }

    /// Persist resolved theme for this session.
    pub fn persist_session_theme(&self, theme: &str) {
        if !self.session_id.is_empty() {
            let _ = fs::write(self.session_theme_file(), theme);
        }
    }
}

pub fn theme_dir(sounds_dir: &Path, theme: &str) -> PathBuf {
    sounds_dir.join(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_sounds_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn fallback_to_peon() {
        let dir = temp_sounds_dir();
        let config = Config::default();
        let resolver = ThemeResolver {
            sounds_dir: dir.path(),
            config: &config,
            session_id: "",
            cwd: "/tmp".to_string(),
        };
        assert_eq!(resolver.resolve(), "peon");
    }

    #[test]
    fn config_theme_field() {
        let dir = temp_sounds_dir();
        let config = Config {
            theme: Some("aoe2".to_string()),
            ..Default::default()
        };
        let resolver = ThemeResolver {
            sounds_dir: dir.path(),
            config: &config,
            session_id: "",
            cwd: "/tmp".to_string(),
        };
        assert_eq!(resolver.resolve(), "aoe2");
    }

    #[test]
    fn legacy_theme_file() {
        let dir = temp_sounds_dir();
        fs::write(dir.path().join("theme"), "icq\n").unwrap();
        let config = Config::default();
        let resolver = ThemeResolver {
            sounds_dir: dir.path(),
            config: &config,
            session_id: "",
            cwd: "/tmp".to_string(),
        };
        assert_eq!(resolver.resolve(), "icq");
    }

    #[test]
    fn workspace_pin_overrides_config() {
        let dir = temp_sounds_dir();
        let mut workspaces = HashMap::new();
        workspaces.insert("/home/user/project".to_string(), "aoe3".to_string());
        let config = Config {
            theme: Some("peon".to_string()),
            workspaces,
            ..Default::default()
        };
        let resolver = ThemeResolver {
            sounds_dir: dir.path(),
            config: &config,
            session_id: "",
            cwd: "/home/user/project".to_string(),
        };
        assert_eq!(resolver.resolve(), "aoe3");
    }

    #[test]
    fn env_var_highest_priority() {
        let dir = temp_sounds_dir();
        let config = Config {
            theme: Some("peon".to_string()),
            ..Default::default()
        };
        std::env::set_var("CLAUDE_SOUND_THEME", "icq");
        let resolver = ThemeResolver {
            sounds_dir: dir.path(),
            config: &config,
            session_id: "",
            cwd: "/tmp".to_string(),
        };
        assert_eq!(resolver.resolve(), "icq");
        std::env::remove_var("CLAUDE_SOUND_THEME");
    }

    #[test]
    fn load_config_from_file() {
        let dir = temp_sounds_dir();
        fs::write(
            dir.path().join("config.json"),
            r#"{"mode": "random", "theme": "peon", "random_pool": ["peon", "aoe2"]}"#,
        )
        .unwrap();
        let config = Config::load(dir.path());
        assert_eq!(config.mode.as_deref(), Some("random"));
        assert_eq!(config.random_pool.len(), 2);
    }
}
```

**Step 2: Add tempfile as dev dependency in Cargo.toml**

Add under `[dev-dependencies]`:
```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Add module to main.rs**

```rust
mod config;
```

**Step 4: Run tests**

Run: `cargo test`
Expected: All tests pass (event + config).

**Step 5: Commit**

```bash
git add src/config.rs src/main.rs Cargo.toml
git commit -m "feat: config loading and theme resolution with tests"
```

---

### Task 4: Manifest Parsing and Sound Picking (manifest.rs)

**Files:**
- Create: `src/manifest.rs`
- Modify: `src/main.rs` (add mod)

**Step 1: Write manifest.rs with parsing and sound picking**

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub display_name: String,
    pub categories: HashMap<String, Category>,
}

#[derive(Debug, Deserialize)]
pub struct Category {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub sounds: Vec<Sound>,
}

#[derive(Debug, Deserialize)]
pub struct Sound {
    pub file: String,
    #[serde(default)]
    pub line: Option<String>,
}

impl Manifest {
    pub fn load(theme_dir: &Path) -> Option<Self> {
        let path = theme_dir.join("manifest.json");
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }
}

pub struct SoundPick {
    pub file: String,
    pub line: Option<String>,
}

/// Pick a random sound from a category. Returns None if category missing or empty.
pub fn pick_sound(manifest: &Manifest, category: &str) -> Option<SoundPick> {
    let cat = manifest.categories.get(category)?;
    if cat.sounds.is_empty() {
        return None;
    }
    use rand::Rng;
    let idx = rand::rng().random_range(0..cat.sounds.len());
    let sound = &cat.sounds[idx];
    Some(SoundPick {
        file: sound.file.clone(),
        line: sound.line.clone(),
    })
}

/// Get category-level title and body from manifest.
pub fn category_text(manifest: &Manifest, category: &str) -> (Option<String>, Option<String>) {
    match manifest.categories.get(category) {
        Some(cat) => (cat.title.clone(), cat.body.clone()),
        None => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> Manifest {
        serde_json::from_str(
            r#"{
                "name": "test",
                "display_name": "Test Theme",
                "categories": {
                    "greeting": {
                        "title": "Hello",
                        "sounds": [
                            {"file": "hello.wav", "line": "Hello there!"},
                            {"file": "hi.wav"}
                        ]
                    },
                    "empty": {
                        "title": "Empty",
                        "sounds": []
                    }
                }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn pick_from_valid_category() {
        let manifest = sample_manifest();
        let pick = pick_sound(&manifest, "greeting");
        assert!(pick.is_some());
        let pick = pick.unwrap();
        assert!(pick.file == "hello.wav" || pick.file == "hi.wav");
    }

    #[test]
    fn pick_from_empty_category_returns_none() {
        let manifest = sample_manifest();
        assert!(pick_sound(&manifest, "empty").is_none());
    }

    #[test]
    fn pick_from_missing_category_returns_none() {
        let manifest = sample_manifest();
        assert!(pick_sound(&manifest, "nonexistent").is_none());
    }

    #[test]
    fn category_text_returns_title() {
        let manifest = sample_manifest();
        let (title, body) = category_text(&manifest, "greeting");
        assert_eq!(title.as_deref(), Some("Hello"));
        assert!(body.is_none());
    }

    #[test]
    fn load_manifest_from_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("manifest.json"),
            r#"{"name":"t","display_name":"T","categories":{}}"#,
        )
        .unwrap();
        let m = Manifest::load(dir.path());
        assert!(m.is_some());
        assert_eq!(m.unwrap().name, "t");
    }

    #[test]
    fn load_missing_manifest_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Manifest::load(dir.path()).is_none());
    }
}
```

**Step 2: Add module to main.rs**

```rust
mod manifest;
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass.

**Step 4: Commit**

```bash
git add src/manifest.rs src/main.rs
git commit -m "feat: manifest parsing and random sound picking with tests"
```

---

### Task 5: Audio Playback (audio.rs)

**Files:**
- Create: `src/audio.rs`
- Modify: `src/main.rs` (add mod)

**Step 1: Write audio.rs**

```rust
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Play a sound file to completion. Blocks until done.
/// Returns Ok(()) on success, Err on any failure.
pub fn play_sound(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let source = Decoder::try_from(reader)?;

    let stream = OutputStream::try_default()?;
    let sink = Sink::connect_new(&stream.mixer());
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
```

Note: Audio playback is a side effect — no unit tests. Integration tested in Task 8.

**Step 2: Add module to main.rs**

```rust
mod audio;
```

**Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles. (May need alsa-lib-devel on Fedora for rodio/cpal.)

If build fails with ALSA errors, run: `sudo dnf install alsa-lib-devel`

**Step 4: Commit**

```bash
git add src/audio.rs src/main.rs
git commit -m "feat: rodio-based cross-platform audio playback"
```

---

### Task 6: Notifications (notify.rs)

**Files:**
- Create: `src/notify.rs`
- Modify: `src/main.rs` (add mod)

**Step 1: Write notify.rs**

```rust
use std::path::Path;

/// Send a desktop notification. Silent failure on error.
pub fn send_notification(title: &str, body: &str, icon: &Path) {
    let icon_str = icon.to_string_lossy();
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon(&icon_str)
        .appname("Claude Code")
        .show();
}
```

**Step 2: Add module to main.rs**

```rust
mod notify;
```

**Step 3: Verify compilation**

Run: `cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add src/notify.rs src/main.rs
git commit -m "feat: cross-platform desktop notifications via notify-rust"
```

---

### Task 7: Main Integration (main.rs)

**Files:**
- Modify: `src/main.rs`

**Step 1: Wire everything together in main.rs**

```rust
mod audio;
mod config;
mod event;
mod manifest;
mod notify;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    if let Err(_) = run() {
        // Silent failure — hooks must never block Claude Code
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Read stdin
    let mut input_str = String::new();
    std::io::stdin().read_to_string(&mut input_str)?;

    let hook_input: event::HookInput = serde_json::from_str(&input_str)?;

    let home = std::env::var("HOME")?;
    let sounds_dir = PathBuf::from(&home).join(".claude/sounds");
    let icon_path = PathBuf::from(&home).join(".claude/clawd.png");

    // Resolve theme
    let cfg = config::Config::load(&sounds_dir);
    let resolver = config::ThemeResolver {
        sounds_dir: &sounds_dir,
        config: &cfg,
        session_id: &hook_input.session_id,
        cwd: std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    };
    let theme = resolver.resolve();
    let theme_dir = config::theme_dir(&sounds_dir, &theme);
    let manifest_path = theme_dir.join("manifest.json");

    // If no manifest, nothing to do
    if !manifest_path.exists() {
        return Ok(());
    }

    let manifest = match manifest::Manifest::load(&theme_dir) {
        Some(m) => m,
        None => return Ok(()),
    };

    // Map event to action
    let action = event::map_event(&hook_input);

    // Handle SessionStart special logic
    if hook_input.hook_event_name == "SessionStart" {
        return handle_session_start(
            &hook_input,
            &resolver,
            &theme,
            &theme_dir,
            &manifest,
        );
    }

    // Normal event handling
    if let Some(ref category) = action.category {
        let pick = manifest::pick_sound(&manifest, category);
        let (cat_title, cat_body) = manifest::category_text(&manifest, category);

        // Resolve title: manifest > hardcoded
        let title = cat_title.unwrap_or(action.title);

        // Resolve body: sound line > manifest body > hardcoded
        let body = pick
            .as_ref()
            .and_then(|p| p.line.clone())
            .or(cat_body)
            .unwrap_or(action.body);

        if !action.skip_notify {
            notify::send_notification(&title, &body, &icon_path);
        }

        if let Some(ref pick) = pick {
            let sound_path = theme_dir.join("sounds").join(&pick.file);
            let _ = audio::play_sound(&sound_path);
        }
    } else if !action.skip_notify {
        notify::send_notification(&action.title, &action.body, &icon_path);
    }

    Ok(())
}

fn handle_session_start(
    hook_input: &event::HookInput,
    resolver: &config::ThemeResolver,
    theme: &str,
    theme_dir: &std::path::Path,
    manifest: &manifest::Manifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_type = hook_input.source.as_deref().unwrap_or("unknown");
    let startup_flag = PathBuf::from(format!(
        "/tmp/.claude-ringring-{}",
        if hook_input.session_id.is_empty() {
            "unknown"
        } else {
            &hook_input.session_id
        }
    ));

    match source_type {
        "startup" => {
            // Persist theme for session
            resolver.persist_session_theme(theme);

            // Write startup flag
            fs::write(&startup_flag, "startup")?;

            // Deferred startup sound: sleep, then play if flag still exists
            let theme_dir = theme_dir.to_path_buf();
            let manifest_json = serde_json::to_string(manifest)?;
            let flag = startup_flag.clone();

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if flag.exists() {
                    if let Ok(m) = serde_json::from_str::<manifest::Manifest>(&manifest_json) {
                        if let Some(pick) = manifest::pick_sound(&m, "greeting") {
                            let sound_path = theme_dir.join("sounds").join(&pick.file);
                            let _ = audio::play_sound(&sound_path);
                        }
                    }
                    let _ = fs::remove_file(&flag);
                }
            })
            .join()
            .ok();
        }
        "resume" => {
            // Cancel pending startup sound
            let _ = fs::remove_file(&startup_flag);
        }
        _ => {}
    }

    Ok(())
}
```

**Step 2: Run all tests to make sure nothing broke**

Run: `cargo test`
Expected: All tests pass.

**Step 3: Build release binary**

Run: `cargo build --release`
Expected: Binary at `target/release/rust-ringring`.

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire all modules together in main entry point"
```

---

### Task 8: Manual Integration Test

**Step 1: Test with a real hook event**

Run:
```bash
echo '{"hook_event_name": "Stop", "session_id": "test123"}' | cargo run --release
```
Expected: Hear a sound, see a notification.

**Step 2: Test SessionStart**

Run:
```bash
echo '{"hook_event_name": "SessionStart", "source": "startup", "session_id": "test456"}' | cargo run --release
```
Expected: After ~1 second, hear a greeting sound. No notification.

**Step 3: Test with missing manifest (should exit cleanly)**

Run:
```bash
CLAUDE_SOUND_THEME=nonexistent echo '{"hook_event_name": "Stop"}' | cargo run --release
```
Expected: Exits 0, no sound, no error.

---

### Task 9: Copy Themes and Deploy

**Step 1: No theme copying needed**

The binary reads themes from `~/.claude/sounds/` which already has all 4 themes (peon, aoe2, aoe3, icq). The Rust binary uses the same paths as the bash script. Nothing to copy.

**Step 2: Install binary**

Run:
```bash
cp target/release/rust-ringring ~/.claude/rust-ringring
chmod +x ~/.claude/rust-ringring
```

**Step 3: Update settings.json to use new binary**

In `~/.claude/settings.json`, replace all `~/.claude/ringring` with `~/.claude/rust-ringring`.

(Keep the old bash script as backup until verified.)

**Step 4: Commit**

```bash
git add -A
git commit -m "feat: complete rust-ringring, ready for deployment"
```
