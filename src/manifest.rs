use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub display_name: String,
    pub categories: HashMap<String, Category>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Category {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub sounds: Vec<Sound>,
}

#[derive(Debug, Deserialize, Serialize)]
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
