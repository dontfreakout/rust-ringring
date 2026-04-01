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
    if let Ok(base) = std::env::var("XDG_CONFIG_HOME")
        && !base.is_empty()
    {
        return std::path::PathBuf::from(base).join("ringring");
    }
    platform_config_fallback().join("ringring")
}

pub fn data_dir() -> std::path::PathBuf {
    if let Ok(base) = std::env::var("XDG_DATA_HOME")
        && !base.is_empty()
    {
        let xdg = std::path::PathBuf::from(base).join("ringring");
        if xdg.join("config.json").exists() || has_themes(&xdg) {
            return xdg;
        }
    }

    let xdg = platform_data_fallback().join("ringring");
    if xdg.join("config.json").exists() || has_themes(&xdg) {
        return xdg;
    }

    // Legacy path — where data lived before XDG migration
    let legacy = home_dir().join(".claude/sounds");
    if legacy.exists() {
        return legacy;
    }

    xdg
}

fn has_themes(dir: &std::path::Path) -> bool {
    dir.read_dir()
        .map(|mut rd| rd.any(|e| e.is_ok_and(|e| e.path().join("manifest.json").exists())))
        .unwrap_or(false)
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
    fn data_dir_prefers_xdg_with_data() {
        let tmp = tempfile::tempdir().unwrap();
        let xdg_data = tmp.path().join("xdg");
        let ring_dir = xdg_data.join("ringring");
        std::fs::create_dir_all(&ring_dir).unwrap();
        std::fs::write(ring_dir.join("config.json"), "{}").unwrap();
        unsafe { std::env::set_var("XDG_DATA_HOME", xdg_data.to_str().unwrap()) };
        let result = data_dir();
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
        assert_eq!(result, ring_dir);
    }

    #[test]
    fn data_dir_falls_back_to_legacy_when_xdg_empty() {
        // When XDG path has no data and ~/.claude/sounds exists, use legacy
        let home = std::env::var("HOME").unwrap();
        let legacy = std::path::PathBuf::from(&home).join(".claude/sounds");
        if legacy.exists() {
            unsafe { std::env::set_var("XDG_DATA_HOME", "/nonexistent/xdg") };
            let result = data_dir();
            unsafe { std::env::remove_var("XDG_DATA_HOME") };
            assert_eq!(result, legacy);
        }
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
    #[cfg(target_os = "macos")]
    fn config_dir_macos_fallback() {
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        let result = config_dir();
        let home = std::env::var("HOME").unwrap();
        assert_eq!(result, std::path::PathBuf::from(format!("{home}/Library/Application Support/ringring")));
    }
}
