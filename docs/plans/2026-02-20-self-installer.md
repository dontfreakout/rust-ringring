# Self-Installer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ringring install` (binary placement + Claude Code hook registration) and `ringring theme install <path|url>` (zip-based theme installation), while migrating all hardcoded `~/.claude/sounds/` paths to XDG-compliant directories.

**Architecture:** New `paths.rs` module centralises XDG/macOS path resolution; new `install.rs` module handles all install logic; existing modules (`config.rs`, `main.rs`) are updated to call `paths::data_dir()` instead of constructing `~/.claude/sounds/` directly.

**Tech Stack:** Rust 2024, `serde_json` (already present), `ureq = "2"` (HTTP download), `zip = "2"` (zip extraction).

---

### Task 1: Add `ureq` and `zip` dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add the two crates**

In `Cargo.toml`, under `[dependencies]`, add:

```toml
ureq = "2"
zip = "2"
```

**Step 2: Verify the build**

```bash
cargo build --quiet
```

Expected: compiles without errors. (No new code yet.)

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add ureq and zip dependencies"
```

---

### Task 2: Create `paths.rs` — XDG-compliant directory resolution

**Files:**
- Create: `src/paths.rs`
- Modify: `src/main.rs` (add `mod paths;`)

**Step 1: Write the failing tests**

Create `src/paths.rs` with tests only:

```rust
fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}

#[cfg(target_os = "macos")]
fn platform_config_fallback() -> std::path::PathBuf {
    home_dir().join("Library/Application Support")
}

#[cfg(not(target_os = "macos"))]
fn platform_config_fallback() -> std::path::PathBuf {
    home_dir().join(".config")
}

#[cfg(target_os = "macos")]
fn platform_data_fallback() -> std::path::PathBuf {
    home_dir().join("Library/Application Support")
}

#[cfg(not(target_os = "macos"))]
fn platform_data_fallback() -> std::path::PathBuf {
    home_dir().join(".local/share")
}

pub fn config_dir() -> std::path::PathBuf {
    todo!()
}

pub fn data_dir() -> std::path::PathBuf {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_uses_xdg_when_set() {
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/custom/config") };
        let result = config_dir();
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        assert_eq!(result, std::path::PathBuf::from("/custom/config/ringring"));
    }

    #[test]
    fn data_dir_uses_xdg_when_set() {
        unsafe { std::env::set_var("XDG_DATA_HOME", "/custom/data") };
        let result = data_dir();
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
        assert_eq!(result, std::path::PathBuf::from("/custom/data/ringring"));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn config_dir_linux_fallback() {
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        let result = config_dir();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(result, std::path::PathBuf::from(format!("{home}/.config/ringring")));
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn data_dir_linux_fallback() {
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
        let result = data_dir();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(result, std::path::PathBuf::from(format!("{home}/.local/share/ringring")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn config_dir_macos_fallback() {
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        let result = config_dir();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(result, std::path::PathBuf::from(format!("{home}/Library/Application Support/ringring")));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn data_dir_macos_fallback() {
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
        let result = data_dir();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(result, std::path::PathBuf::from(format!("{home}/Library/Application Support/ringring")));
    }
}
```

**Step 2: Add `mod paths;` to `src/main.rs`**

At the top of `src/main.rs`, after the existing `mod` declarations:

```rust
mod paths;
```

**Step 3: Run tests to verify they fail**

```bash
cargo test -- paths::tests 2>&1 | head -20
```

Expected: FAIL — `todo!()` panics.

**Step 4: Implement `config_dir` and `data_dir`**

Replace the `todo!()` bodies in `src/paths.rs`:

```rust
pub fn config_dir() -> std::path::PathBuf {
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME") {
        if !base.is_empty() {
            return std::path::PathBuf::from(base).join("ringring");
        }
    }
    platform_config_fallback().join("ringring")
}

pub fn data_dir() -> std::path::PathBuf {
    if let Ok(base) = std::env::var("XDG_DATA_HOME") {
        if !base.is_empty() {
            return std::path::PathBuf::from(base).join("ringring");
        }
    }
    platform_data_fallback().join("ringring")
}
```

**Step 5: Run tests to verify they pass**

```bash
cargo test -- paths::tests
```

Expected: all pass.

**Step 6: Commit**

```bash
git add src/paths.rs src/main.rs
git commit -m "feat: add paths module with XDG/macOS directory resolution"
```

---

### Task 3: Migrate hardcoded `~/.claude/sounds/` to `paths::data_dir()`

**Files:**
- Modify: `src/main.rs` (three functions: `run`, `run_list`, `run_test`)

**Step 1: Run existing tests to establish baseline**

```bash
cargo test
```

Expected: all pass.

**Step 2: Replace `sounds_dir` construction in `run()`**

In `src/main.rs`, find `run()`. Replace:

```rust
let home = std::env::var("HOME")?;
let sounds_dir = PathBuf::from(&home).join(".claude/sounds");
```

With:

```rust
let sounds_dir = paths::data_dir();
```

Remove the unused `home` variable. The `use std::path::PathBuf;` import at the top may no longer be needed in `run()` — leave it for now (it's used elsewhere or will be used in install).

**Step 3: Replace `sounds_dir` construction in `run_list()`**

Find `run_list()`. Replace:

```rust
let Ok(home) = std::env::var("HOME") else { return };
let sounds_dir = PathBuf::from(&home).join(".claude/sounds");
```

With:

```rust
let sounds_dir = paths::data_dir();
```

**Step 4: Replace `sounds_dir` construction in `run_test()`**

Find `run_test()`. Replace:

```rust
let home = std::env::var("HOME")?;
let sounds_dir = PathBuf::from(&home).join(".claude/sounds");
```

With:

```rust
let sounds_dir = paths::data_dir();
```

**Step 5: Run all tests**

```bash
cargo test
```

Expected: all pass. (Behaviour unchanged; path resolution now goes through `paths::data_dir()`.)

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "refactor: migrate sounds_dir to paths::data_dir()"
```

---

### Task 4: Create `install.rs` — binary installation and hook registration

**Files:**
- Create: `src/install.rs`
- Modify: `src/main.rs` (add `mod install;`)

**Step 1: Write failing tests for `install_binary`**

Create `src/install.rs`:

```rust
use std::path::{Path, PathBuf};

/// Copy the running binary to `dest_dir/ringring` with executable permissions.
pub fn install_binary(dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}

/// Merge ringring hook entries into `~/.claude/settings.json`.
pub fn register_hooks(settings_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn install_binary_copies_and_makes_executable() {
        let dest = tempfile::tempdir().unwrap();
        install_binary(dest.path()).unwrap();
        let bin = dest.path().join("ringring");
        assert!(bin.exists(), "binary was not copied");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&bin).unwrap().permissions().mode();
            assert!(mode & 0o111 != 0, "binary is not executable");
        }
    }

    #[test]
    fn install_binary_creates_dest_dir_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("nested/bin");
        install_binary(&dest).unwrap();
        assert!(dest.join("ringring").exists());
    }

    #[test]
    fn register_hooks_creates_settings_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        register_hooks(&settings).unwrap();
        let content = fs::read_to_string(&settings).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        for event in ["SessionStart", "Stop", "Notification", "PermissionRequest"] {
            let arr = v["hooks"][event].as_array().unwrap();
            let has_ringring = arr.iter().any(|entry| {
                entry["hooks"].as_array()
                    .map(|hooks| hooks.iter().any(|h| h["command"] == "ringring"))
                    .unwrap_or(false)
            });
            assert!(has_ringring, "missing ringring hook for {event}");
        }
    }

    #[test]
    fn register_hooks_preserves_existing_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        fs::write(&settings, r#"{"hooks":{"PostToolUse":[{"matcher":"Edit","hooks":[{"type":"command","command":"cargo check"}]}]},"otherField":42}"#).unwrap();
        register_hooks(&settings).unwrap();
        let content = fs::read_to_string(&settings).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["otherField"], 42);
        assert!(v["hooks"]["PostToolUse"].as_array().unwrap().len() >= 1);
    }

    #[test]
    fn register_hooks_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        register_hooks(&settings).unwrap();
        register_hooks(&settings).unwrap();
        let content = fs::read_to_string(&settings).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        for event in ["SessionStart", "Stop", "Notification", "PermissionRequest"] {
            let count = v["hooks"][event].as_array().unwrap().iter()
                .filter(|entry| {
                    entry["hooks"].as_array()
                        .map(|hooks| hooks.iter().any(|h| h["command"] == "ringring"))
                        .unwrap_or(false)
                })
                .count();
            assert_eq!(count, 1, "duplicate ringring entry for {event}");
        }
    }
}
```

**Step 2: Add `mod install;` to `src/main.rs`**

```rust
mod install;
```

**Step 3: Run tests to verify they fail**

```bash
cargo test -- install::tests 2>&1 | head -30
```

Expected: FAIL — `todo!()` panics.

**Step 4: Implement `install_binary`**

Replace the `todo!()` in `install_binary`:

```rust
pub fn install_binary(dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let exe = std::env::current_exe()?;
    std::fs::create_dir_all(dest_dir)?;
    let dest = dest_dir.join("ringring");
    std::fs::copy(&exe, &dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;
    }
    Ok(())
}
```

**Step 5: Implement `register_hooks`**

Replace the `todo!()` in `register_hooks`:

```rust
pub fn register_hooks(settings_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(settings_path).unwrap_or_else(|_| "{}".to_string());
    let mut root: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

    if !root.is_object() {
        root = serde_json::json!({});
    }
    if !root["hooks"].is_object() {
        root["hooks"] = serde_json::json!({});
    }

    let events = ["SessionStart", "Stop", "Notification", "PermissionRequest"];

    for event in events {
        if !root["hooks"][event].is_array() {
            root["hooks"][event] = serde_json::json!([]);
        }

        let already = root["hooks"][event]
            .as_array()
            .map(|arr| {
                arr.iter().any(|entry| {
                    entry["hooks"]
                        .as_array()
                        .map(|hooks| hooks.iter().any(|h| h["command"] == "ringring"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if !already {
            let entry = serde_json::json!({
                "matcher": "",
                "hooks": [{"type": "command", "command": "ringring"}]
            });
            root["hooks"][event].as_array_mut().unwrap().push(entry);
        }
    }

    // Atomic write: write to .tmp then rename
    let tmp_path = settings_path.with_extension("json.tmp");
    let serialized = serde_json::to_string_pretty(&root)?;
    std::fs::write(&tmp_path, serialized)?;
    std::fs::rename(&tmp_path, settings_path)?;

    Ok(())
}
```

**Step 6: Run tests**

```bash
cargo test -- install::tests
```

Expected: all pass.

**Step 7: Commit**

```bash
git add src/install.rs src/main.rs
git commit -m "feat: add install_binary and register_hooks to install module"
```

---

### Task 5: Create `install.rs` — theme installation from zip

**Files:**
- Modify: `src/install.rs` (add `theme_install` function)

**Step 1: Write failing tests**

Append to the `tests` module in `src/install.rs`:

```rust
    fn make_theme_zip(tmp: &tempfile::TempDir, theme_name: &str) -> PathBuf {
        use std::io::Write;
        let zip_path = tmp.path().join("theme.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.add_directory(format!("{theme_name}/"), opts).unwrap();
        zip.add_directory(format!("{theme_name}/sounds/"), opts).unwrap();
        zip.start_file(format!("{theme_name}/manifest.json"), opts).unwrap();
        zip.write_all(br#"{"display_name":"Test","categories":{}}"#).unwrap();
        zip.start_file(format!("{theme_name}/sounds/beep.wav"), opts).unwrap();
        zip.write_all(b"RIFF....").unwrap();
        zip.finish().unwrap();
        zip_path
    }

    #[test]
    fn theme_install_from_local_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = make_theme_zip(&tmp, "mytheme");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let name = theme_install(&zip_path.to_string_lossy(), &data_dir, false).unwrap();
        assert_eq!(name, "mytheme");
        assert!(data_dir.join("mytheme/manifest.json").exists());
        assert!(data_dir.join("mytheme/sounds/beep.wav").exists());
    }

    #[test]
    fn theme_install_rejects_existing_without_force() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = make_theme_zip(&tmp, "mytheme");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        theme_install(&zip_path.to_string_lossy(), &data_dir, false).unwrap();
        let err = theme_install(&zip_path.to_string_lossy(), &data_dir, false).unwrap_err();
        assert!(err.to_string().contains("already exists"), "expected 'already exists', got: {err}");
    }

    #[test]
    fn theme_install_force_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = make_theme_zip(&tmp, "mytheme");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        theme_install(&zip_path.to_string_lossy(), &data_dir, false).unwrap();
        theme_install(&zip_path.to_string_lossy(), &data_dir, true).unwrap();
        assert!(data_dir.join("mytheme/manifest.json").exists());
    }

    #[test]
    fn theme_install_rejects_missing_manifest() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("bad.zip");
        let file = fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.add_directory("nomanifest/", opts).unwrap();
        zip.start_file("nomanifest/sounds/beep.wav", opts).unwrap();
        zip.write_all(b"data").unwrap();
        zip.finish().unwrap();

        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&data_dir).unwrap();
        let err = theme_install(&zip_path.to_string_lossy(), &data_dir, false).unwrap_err();
        assert!(err.to_string().contains("manifest.json"));
        // Partial extraction should be cleaned up
        assert!(!data_dir.join("nomanifest").exists());
    }
```

Also add `use std::io::Write;` and `use std::path::PathBuf;` to the test module imports.

**Step 2: Run tests to verify they fail**

```bash
cargo test -- install::tests::theme 2>&1 | head -30
```

Expected: compile error (function `theme_install` not defined yet).

**Step 3: Implement helper functions and `theme_install`**

Add to `src/install.rs` (before `#[cfg(test)]`):

```rust
/// Find the single top-level directory name in a zip archive.
fn zip_theme_name(archive: &mut zip::ZipArchive<std::fs::File>) -> Result<String, Box<dyn std::error::Error>> {
    let mut top_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let first = entry.name().split('/').next().unwrap_or("").to_string();
        if !first.is_empty() {
            top_dirs.insert(first);
        }
    }
    if top_dirs.len() != 1 {
        return Err(format!(
            "zip must contain exactly one top-level directory, found {}",
            top_dirs.len()
        )
        .into());
    }
    Ok(top_dirs.into_iter().next().unwrap())
}

/// Extract a zip archive into `dest_parent`. All entries are placed relative to `dest_parent`.
fn extract_zip(archive: &mut zip::ZipArchive<std::fs::File>, dest_parent: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let out_path = dest_parent.join(entry.name());
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

/// Install a theme from a local zip path or an https:// URL.
/// Returns the theme name on success.
pub fn theme_install(source: &str, data_dir: &Path, force: bool) -> Result<String, Box<dyn std::error::Error>> {
    // Resolve to a local zip file (download if URL)
    let tmp_file;
    let zip_path: &Path = if source.starts_with("http://") || source.starts_with("https://") {
        let tmp = tempfile::NamedTempFile::new()?;
        let response = ureq::get(source).call()?;
        let mut reader = response.into_reader();
        let mut file = std::fs::File::create(tmp.path())?;
        std::io::copy(&mut reader, &mut file)?;
        tmp_file = tmp;
        tmp_file.path()
    } else {
        std::path::Path::new(source)
    };

    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let theme_name = zip_theme_name(&mut archive)?;

    let dest = data_dir.join(&theme_name);
    if dest.exists() {
        if !force {
            return Err(format!(
                "theme '{}' already exists; use --force to overwrite",
                theme_name
            )
            .into());
        }
        std::fs::remove_dir_all(&dest)?;
    }

    extract_zip(&mut archive, data_dir)?;

    // Validate manifest exists; clean up if not
    if !dest.join("manifest.json").exists() {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(format!("theme '{}' has no manifest.json", theme_name).into());
    }

    Ok(theme_name)
}
```

Note: `tempfile` is already in `[dev-dependencies]`. Since `theme_install` uses `tempfile::NamedTempFile` at runtime (not just tests), move `tempfile` to `[dependencies]`:

In `Cargo.toml`, move `tempfile = "3"` from `[dev-dependencies]` to `[dependencies]`.

**Step 4: Run tests**

```bash
cargo test -- install::tests
```

Expected: all pass.

**Step 5: Commit**

```bash
git add src/install.rs Cargo.toml Cargo.lock
git commit -m "feat: add theme_install from local/remote zip"
```

---

### Task 6: Wire up `Install` and `ThemeInstall` subcommands in `main.rs`

**Files:**
- Modify: `src/main.rs`

**Step 1: Write failing parse tests**

Append to the `tests` module in `src/main.rs`:

```rust
    #[test]
    fn parse_install() {
        let args = vec!["ringring".to_string(), "install".to_string()];
        assert!(matches!(parse_args(&args), Cmd::Install));
    }

    #[test]
    fn parse_theme_install_local() {
        let args = vec!["ringring".to_string(), "theme".to_string(), "install".to_string(), "/tmp/foo.zip".to_string()];
        assert!(matches!(parse_args(&args), Cmd::ThemeInstall { source, force: false } if source == "/tmp/foo.zip"));
    }

    #[test]
    fn parse_theme_install_force() {
        let args = vec!["ringring".to_string(), "theme".to_string(), "install".to_string(), "--force".to_string(), "https://example.com/t.zip".to_string()];
        assert!(matches!(parse_args(&args), Cmd::ThemeInstall { source, force: true } if source == "https://example.com/t.zip"));
    }

    #[test]
    fn parse_theme_install_missing_source() {
        let args = vec!["ringring".to_string(), "theme".to_string(), "install".to_string()];
        assert!(matches!(parse_args(&args), Cmd::ThemeInstall { source, .. } if source.is_empty()));
    }
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -- tests::parse_install tests::parse_theme_install 2>&1 | head -20
```

Expected: compile error — `Cmd::Install` and `Cmd::ThemeInstall` don't exist.

**Step 3: Add new `Cmd` variants**

In `src/main.rs`, extend the `Cmd` enum:

```rust
enum Cmd {
    Hook,
    Test { theme: String, category: Option<String> },
    List { debug: bool },
    Install,
    ThemeInstall { source: String, force: bool },
}
```

**Step 4: Add parsing in `parse_args`**

In `parse_args`, add new arms before `_ => Cmd::Hook`:

```rust
        Some("install") => Cmd::Install,
        Some("theme") => {
            match args.get(2).map(|s| s.as_str()) {
                Some("install") => {
                    let rest = args.get(3..).unwrap_or(&[]);
                    let force = rest.iter().any(|a| a == "--force");
                    let source = rest.iter()
                        .find(|a| *a != "--force")
                        .cloned()
                        .unwrap_or_default();
                    Cmd::ThemeInstall { source, force }
                }
                _ => Cmd::Hook,
            }
        }
```

**Step 5: Run parse tests**

```bash
cargo test -- tests::parse_install tests::parse_theme_install
```

Expected: all pass.

**Step 6: Wire up `run_install` and `run_theme_install` in `main`**

Add the two match arms to `main()`:

```rust
        Cmd::Install => {
            if let Err(e) = run_install() {
                eprintln!("ringring install: {e}");
                std::process::exit(1);
            }
        }
        Cmd::ThemeInstall { source, force } => {
            if let Err(e) = run_theme_install(&source, force) {
                eprintln!("ringring theme install: {e}");
                std::process::exit(1);
            }
        }
```

Add the two driver functions (after `run_test`):

```rust
fn run_install() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let bin_dir = PathBuf::from(&home).join(".local/bin");
    let config_dir = paths::config_dir();
    let data_dir = paths::data_dir();

    std::fs::create_dir_all(&config_dir)?;
    println!("created {}", config_dir.display());

    std::fs::create_dir_all(&data_dir)?;
    println!("created {}", data_dir.display());

    install::install_binary(&bin_dir)?;
    println!("installed binary to {}", bin_dir.join("ringring").display());

    let settings_path = PathBuf::from(&home).join(".claude/settings.json");
    install::register_hooks(&settings_path)?;
    println!("registered hooks in {}", settings_path.display());

    Ok(())
}

fn run_theme_install(source: &str, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    if source.is_empty() {
        return Err("usage: ringring theme install [--force] <path|url>".into());
    }
    let data_dir = paths::data_dir();
    let theme_name = install::theme_install(source, &data_dir, force)?;
    println!("installed theme '{theme_name}' to {}", data_dir.join(&theme_name).display());
    Ok(())
}
```

**Step 7: Run all tests**

```bash
cargo test
```

Expected: all pass.

**Step 8: Smoke-test the install subcommand**

```bash
cargo build && ./target/debug/ringring install
```

Expected: output showing paths created and binary installed, exit 0.

**Step 9: Commit**

```bash
git add src/main.rs
git commit -m "feat: add install and theme install subcommands"
```

---

### Task 7: Final verification

**Step 1: Run full test suite**

```bash
cargo test
```

Expected: all pass, zero warnings.

**Step 2: Check for clippy warnings**

```bash
cargo clippy
```

Fix any warnings.

**Step 3: Release build**

```bash
cargo build --release
```

Expected: compiles cleanly.

**Step 4: Commit any clippy fixes**

```bash
git add -p
git commit -m "fix: address clippy warnings in installer"
```
