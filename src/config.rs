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
        // Use a unique env var name to avoid test interference
        // We can't safely test env vars in parallel, so this test
        // sets and immediately unsets
        // SAFETY: This test runs serially; env vars are process-global
        unsafe { std::env::set_var("CLAUDE_SOUND_THEME", "icq") };
        let resolver = ThemeResolver {
            sounds_dir: dir.path(),
            config: &config,
            session_id: "",
            cwd: "/tmp".to_string(),
        };
        let result = resolver.resolve();
        unsafe { std::env::remove_var("CLAUDE_SOUND_THEME") };
        assert_eq!(result, "icq");
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
