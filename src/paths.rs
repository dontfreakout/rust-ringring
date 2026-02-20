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
