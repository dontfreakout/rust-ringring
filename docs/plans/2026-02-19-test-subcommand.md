# Test Subcommand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ringring test <theme> [--category <cat>]` subcommand that plays all sounds in a theme sequentially without needing Claude Code hook events.

**Architecture:** Dispatch on `args[1] == "test"` before the stdin-hook path in `main.rs`. `run_test` loads the manifest, collects target categories (all or one), and plays each sound file in order using the existing `audio::play_sound`. No new files or dependencies.

**Tech Stack:** Rust 2024, rodio (via `audio::play_sound`), `std::env::args()` for arg parsing, existing `manifest` and `config` modules.

---

### Task 1: Arg dispatch — route `test` subcommand before stdin read

**Files:**
- Modify: `src/main.rs:10-13`

**Step 1: Write the failing test**

Add to `src/main.rs` inside a `#[cfg(test)]` block at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test_args_theme_only() {
        let args = vec!["ringring".to_string(), "test".to_string(), "peon".to_string()];
        let cmd = parse_args(&args);
        assert!(matches!(cmd, Cmd::Test { theme, category: None } if theme == "peon"));
    }

    #[test]
    fn parse_test_args_with_category() {
        let args = vec![
            "ringring".to_string(),
            "test".to_string(),
            "peon".to_string(),
            "--category".to_string(),
            "greeting".to_string(),
        ];
        let cmd = parse_args(&args);
        assert!(matches!(cmd, Cmd::Test { theme, category: Some(cat) } if theme == "peon" && cat == "greeting"));
    }

    #[test]
    fn parse_hook_mode_when_no_subcommand() {
        let args = vec!["ringring".to_string()];
        let cmd = parse_args(&args);
        assert!(matches!(cmd, Cmd::Hook));
    }
}
```

**Step 2: Run to verify it fails**

```bash
cargo test parse_test_args
```

Expected: compile error — `parse_args` and `Cmd` not defined yet.

**Step 3: Implement `Cmd` enum and `parse_args`**

Add to `src/main.rs` (before `fn main`):

```rust
enum Cmd {
    Hook,
    Test { theme: String, category: Option<String> },
}

fn parse_args(args: &[String]) -> Cmd {
    if args.get(1).map(|s| s.as_str()) == Some("test") {
        let theme = args.get(2).cloned().unwrap_or_default();
        let category = args
            .windows(2)
            .find(|w| w[0] == "--category")
            .map(|w| w[1].clone());
        Cmd::Test { theme, category }
    } else {
        Cmd::Hook
    }
}
```

Update `fn main()` to dispatch:

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args) {
        Cmd::Test { theme, category } => {
            if let Err(e) = run_test(&theme, category.as_deref()) {
                eprintln!("ringring test: {e}");
                std::process::exit(1);
            }
        }
        Cmd::Hook => {
            let _ = run();
        }
    }
}
```

Add stub so it compiles:

```rust
fn run_test(_theme: &str, _category: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test parse_test_args
```

Expected: all 3 tests PASS.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add Cmd enum and parse_args for test subcommand dispatch"
```

---

### Task 2: Implement `run_test` body

**Files:**
- Modify: `src/main.rs` — replace stub `run_test`

**Step 1: Write the failing test**

This logic depends on the filesystem, so test it manually after implementation (no unit test needed here — the manifest loading is already tested in `manifest.rs`). Skip to step 3.

**Step 2: (skipped — not unit-testable in isolation)**

**Step 3: Implement `run_test`**

Replace the stub:

```rust
fn run_test(theme: &str, category: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if theme.is_empty() {
        return Err("usage: ringring test <theme> [--category <cat>]".into());
    }

    let home = std::env::var("HOME")?;
    let sounds_dir = PathBuf::from(&home).join(".claude/sounds");
    let theme_dir = config::theme_dir(&sounds_dir, theme);

    let manifest = manifest::Manifest::load(&theme_dir)
        .ok_or_else(|| format!("no manifest found for theme '{theme}'"))?;

    let mut categories: Vec<&str> = if let Some(cat) = category {
        if !manifest.categories.contains_key(cat) {
            return Err(format!("category '{cat}' not found in theme '{theme}'").into());
        }
        vec![cat]
    } else {
        let mut keys: Vec<&str> = manifest.categories.keys().map(|s| s.as_str()).collect();
        keys.sort();
        keys
    };

    for cat_name in &categories {
        let Some(cat) = manifest.categories.get(*cat_name) else {
            continue;
        };
        for sound in &cat.sounds {
            println!("[{cat_name}] {}", sound.file);
            let sound_path = theme_dir.join("sounds").join(&sound.file);
            let _ = audio::play_sound(&sound_path);
        }
    }

    Ok(())
}
```

**Step 4: Build and smoke-test manually**

```bash
cargo build
# If you have a peon theme installed:
~/.claude/ringring test peon
~/.claude/ringring test peon --category greeting
# Expected: prints [category] filename lines, plays each sound
```

If no theme is installed, verify the error path:

```bash
~/.claude/ringring test nonexistent
# Expected: prints "ringring test: no manifest found for theme 'nonexistent'" to stderr, exits 1
```

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement run_test to play all theme sounds sequentially"
```

---

### Task 3: Missing theme argument error

**Files:**
- Modify: `src/main.rs` — `parse_args` and `main` dispatch

**Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn parse_test_args_missing_theme() {
    let args = vec!["ringring".to_string(), "test".to_string()];
    let cmd = parse_args(&args);
    // theme will be empty string — run_test handles the error
    assert!(matches!(cmd, Cmd::Test { theme, .. } if theme.is_empty()));
}
```

**Step 2: Run to verify it passes already**

```bash
cargo test parse_test_args_missing_theme
```

Expected: PASS (empty string is what `unwrap_or_default` returns).

**Step 3: Verify error message at runtime**

```bash
cargo build && ./target/debug/ringring test
# Expected stderr: "ringring test: usage: ringring test <theme> [--category <cat>]"
# Expected exit code: 1
echo $?
```

**Step 4: (Already passing — no code change needed)**

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "test: add parse_test_args_missing_theme coverage"
```

---

### Task 4: Release build and final verification

**Step 1: Build release**

```bash
cargo build --release
```

Expected: compiles clean, no warnings.

**Step 2: Run all tests**

```bash
cargo test
```

Expected: all tests PASS.

**Step 3: Smoke test the binary**

```bash
./target/release/ringring test peon
./target/release/ringring test peon --category greeting
./target/release/ringring test badtheme
./target/release/ringring test peon --category badcat
```

**Step 4: Commit**

```bash
git add -p  # stage only if anything changed
git commit -m "chore: verify release build for test subcommand"
```

Or skip commit if nothing changed.
