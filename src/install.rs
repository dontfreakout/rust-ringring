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
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = settings_path.with_extension("json.tmp");
    let serialized = serde_json::to_string_pretty(&root)?;
    std::fs::write(&tmp_path, serialized)?;
    std::fs::rename(&tmp_path, settings_path)?;

    Ok(())
}

/// Find the single top-level directory name in a zip archive.
fn zip_theme_name(archive: &mut zip::ZipArchive<std::fs::File>) -> Result<String, Box<dyn std::error::Error>> {
    let mut top_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        if let Some(rel) = entry.enclosed_name() {
            if let Some(first) = rel.components().next() {
                let name = first.as_os_str().to_string_lossy().into_owned();
                if !name.is_empty() {
                    top_dirs.insert(name);
                }
            }
        }
    }
    if top_dirs.is_empty() {
        return Err("zip is empty or contains no files".into());
    }
    if top_dirs.len() > 1 {
        return Err(format!(
            "zip must contain exactly one top-level directory, found {}: {}",
            top_dirs.len(),
            {
                let mut names: Vec<_> = top_dirs.into_iter().collect();
                names.sort();
                names.join(", ")
            }
        ).into());
    }
    Ok(top_dirs.into_iter().next().unwrap())
}

/// Extract a zip archive into `dest_parent`. All entries placed relative to `dest_parent`.
fn extract_zip(archive: &mut zip::ZipArchive<std::fs::File>, dest_parent: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(rel_path) = entry.enclosed_name() else { continue };
        let out_path = dest_parent.join(&rel_path);
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

/// Install a theme from a local zip path or an http(s):// URL.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

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
        assert!(!data_dir.join("nomanifest").exists());
    }
}
