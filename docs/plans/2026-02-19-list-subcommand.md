# List Subcommand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `ringring list [--debug]` to enumerate all installed sound themes with optional verbose manifest output.

**Architecture:** Add `List { debug: bool }` to the existing `Cmd` enum, extend `parse_args` to route `args[1] == "list"`, add `run_list` in `main.rs`. Theme discovery scans `~/.claude/sounds/` for subdirectories with a valid `manifest.json`. All logic stays in `main.rs`, no new files or dependencies.

**Tech Stack:** Rust 2024, `std::fs::read_dir` for directory scanning, existing `manifest::Manifest` for parsing.

---

### Task 1: Extend `Cmd` and `parse_args` for `list`

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the failing tests**

Add to `#[cfg(test)] mod tests` in `src/main.rs`:

```rust
#[test]
fn parse_list_no_flags() {
    let args = vec!["ringring".to_string(), "list".to_string()];
    let cmd = parse_args(&args);
    assert!(matches!(cmd, Cmd::List { debug: false }));
}

#[test]
fn parse_list_with_debug() {
    let args = vec!["ringring".to_string(), "list".to_string(), "--debug".to_string()];
    let cmd = parse_args(&args);
    assert!(matches!(cmd, Cmd::List { debug: true }));
}
```

**Step 2: Run to verify they fail**

```bash
cargo test parse_list 2>&1
```

Expected: compile error — `Cmd::List` not defined.

**Step 3: Add `List` variant to `Cmd` and extend `parse_args`**

Change the `Cmd` enum (around line 10):

```rust
enum Cmd {
    Hook,
    Test { theme: String, category: Option<String> },
    List { debug: bool },
}
```

In `parse_args` (around line 15), add a branch before the `else`:

```rust
fn parse_args(args: &[String]) -> Cmd {
    match args.get(1).map(|s| s.as_str()) {
        Some("test") => {
            let theme = args.get(2).cloned().unwrap_or_default();
            let category = args.get(3..).unwrap_or(&[])
                .windows(2)
                .find(|w| w[0] == "--category")
                .map(|w| w[1].clone());
            Cmd::Test { theme, category }
        }
        Some("list") => {
            let debug = args.get(2..).unwrap_or(&[]).contains(&"--debug".to_string());
            Cmd::List { debug }
        }
        _ => Cmd::Hook,
    }
}
```

Add dispatch in `main()` (around line 30):

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
        Cmd::List { debug } => {
            run_list(debug);
        }
        Cmd::Hook => {
            let _ = run();
        }
    }
}
```

Add stub `run_list` after `run_test`:

```rust
fn run_list(_debug: bool) {
    // TODO: implement
}
```

**Step 4: Run tests**

```bash
cargo test parse_list 2>&1
```

Expected: both tests PASS.

```bash
cargo test 2>&1
```

Expected: all 23 existing + 2 new = 25 tests PASS.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add List variant to Cmd and parse_args dispatch"
```

---

### Task 2: Implement `run_list` default output

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the failing test**

There is no unit-testable logic here (depends on `$HOME` and filesystem). Skip to step 3. Manual smoke testing in Task 4 covers this.

**Step 2: (skipped)**

**Step 3: Replace the `run_list` stub**

```rust
fn run_list(debug: bool) {
    let Ok(home) = std::env::var("HOME") else { return };
    let sounds_dir = PathBuf::from(&home).join(".claude/sounds");

    let Ok(entries) = std::fs::read_dir(&sounds_dir) else { return };

    let mut themes: Vec<(String, manifest::Manifest)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let m = manifest::Manifest::load(&e.path())?;
            Some((name, m))
        })
        .collect();

    themes.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (name, manifest) in &themes {
        if debug {
            print_theme_debug(name, manifest);
        } else {
            println!("{}\t{}", name, manifest.display_name);
        }
    }
}
```

**Step 4: Build**

```bash
cargo build 2>&1
```

Expected: compiles clean.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: implement run_list default output"
```

---

### Task 3: Implement `--debug` output via `print_theme_debug`

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the failing test**

No unit test needed (pure formatting, covered by smoke test). Skip to step 3.

**Step 2: (skipped)**

**Step 3: Add `print_theme_debug` after `run_list`**

```rust
fn print_theme_debug(name: &str, manifest: &manifest::Manifest) {
    println!("=== {} ({}) ===", name, manifest.display_name);
    let mut categories: Vec<(&str, &manifest::Category)> =
        manifest.categories.iter().map(|(k, v)| (k.as_str(), v)).collect();
    categories.sort_by_key(|(k, _)| *k);
    for (cat_name, cat) in categories {
        println!("  {}:", cat_name);
        for sound in &cat.sounds {
            if let Some(ref line) = sound.line {
                println!("    - {}  \"{}\"", sound.file, line);
            } else {
                println!("    - {}", sound.file);
            }
        }
    }
}
```

**Step 4: Build and run all tests**

```bash
cargo build 2>&1
cargo test 2>&1
```

Expected: compiles clean, 25 tests PASS.

**Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --debug output for ringring list"
```

---

### Task 4: Release build and final verification

**Step 1: Release build**

```bash
cargo build --release 2>&1
```

Expected: clean, no warnings.

**Step 2: All tests**

```bash
cargo test 2>&1
```

Expected: 25 passed, 0 failed.

**Step 3: Smoke test**

```bash
./target/release/ringring list
# Expected: one line per installed theme: "<name>\t<display_name>"
# If no themes installed: no output, exit 0

./target/release/ringring list --debug
# Expected: === <name> (<display_name>) === headers with category/sound detail

./target/release/ringring list --unknown-flag
# Expected: same as `ringring list` (unknown flags silently ignored — debug stays false)
```

**Step 4: Verify hook mode unchanged**

```bash
echo '{"session_id":"test123","hook_event_name":"Stop","transcript_path":"/tmp/x"}' \
  | ./target/release/ringring 2>&1; echo "exit=$?"
```

Expected: exit=0.

**Step 5: Commit only if something changed**

```bash
git status
# If clean, skip commit.
```
