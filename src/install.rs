use std::path::Path;

/// Copy the running binary to `dest_dir/ringring` with executable permissions.
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

/// Merge ringring hook entries into the Claude Code settings.json at `settings_path`.
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
