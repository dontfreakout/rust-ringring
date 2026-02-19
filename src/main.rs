mod audio;
mod config;
mod event;
mod manifest;
mod notify;

use std::fs;
use std::path::PathBuf;

enum Cmd {
    Hook,
    Test { theme: String, category: Option<String> },
}

fn parse_args(args: &[String]) -> Cmd {
    if args.get(1).map(|s| s.as_str()) == Some("test") {
        let theme = args.get(2).cloned().unwrap_or_default();
        let category = args.get(3..).unwrap_or(&[])
            .windows(2)
            .find(|w| w[0] == "--category")
            .map(|w| w[1].clone());
        Cmd::Test { theme, category }
    } else {
        Cmd::Hook
    }
}

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

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let input_str = std::io::read_to_string(std::io::stdin())?;
    let hook_input: event::HookInput = serde_json::from_str(&input_str)?;

    let home = std::env::var("HOME")?;
    let sounds_dir = PathBuf::from(&home).join(".claude/sounds");

    let cfg = config::Config::load(&sounds_dir);
    let cwd = std::env::current_dir().unwrap_or_default();
    let resolver = config::ThemeResolver {
        sounds_dir: &sounds_dir,
        config: &cfg,
        session_id: &hook_input.session_id,
        cwd: cwd.to_string_lossy().into_owned(),
    };
    let theme = resolver.resolve();
    let theme_dir = config::theme_dir(&sounds_dir, &theme);

    let Some(manifest) = manifest::Manifest::load(&theme_dir) else {
        return Ok(());
    };

    if hook_input.hook_event_name == "SessionStart" {
        return handle_session_start(&hook_input, &resolver, &theme, &theme_dir, &manifest);
    }

    let action = event::map_event(&hook_input);

    if let Some(ref category) = action.category {
        let pick = manifest::pick_sound(&manifest, category);
        let (cat_title, cat_body) = manifest::category_text(&manifest, category);

        let title = cat_title.unwrap_or(action.title);
        let body = pick
            .as_ref()
            .and_then(|p| p.line.clone())
            .or(cat_body)
            .unwrap_or(action.body);

        if !action.skip_notify {
            notify::send_notification(&title, &body);
        }

        if let Some(ref pick) = pick {
            let sound_path = theme_dir.join("sounds").join(&pick.file);
            let _ = audio::play_sound(&sound_path);
        }
    } else if !action.skip_notify {
        notify::send_notification(&action.title, &action.body);
    }

    Ok(())
}

fn run_test(theme: &str, category: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if theme.is_empty() {
        return Err("usage: ringring test <theme> [--category <cat>]".into());
    }

    let home = std::env::var("HOME")?;
    let sounds_dir = PathBuf::from(&home).join(".claude/sounds");
    let theme_dir = config::theme_dir(&sounds_dir, theme);

    let manifest = manifest::Manifest::load(&theme_dir)
        .ok_or_else(|| format!("no manifest found for theme '{theme}'"))?;

    let categories: Vec<&str> = if let Some(cat) = category {
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
    fn parse_test_args_category_flag_not_confused_with_theme() {
        // No theme given; --category should NOT be picked up as a category value
        let args = vec![
            "ringring".to_string(),
            "test".to_string(),
            "--category".to_string(),
            "greeting".to_string(),
        ];
        let cmd = parse_args(&args);
        // theme is "--category" (args[2]), no --category flag in args[3..]
        assert!(matches!(cmd, Cmd::Test { ref theme, category: None } if theme == "--category"));
    }

    #[test]
    fn parse_hook_mode_when_no_subcommand() {
        let args = vec!["ringring".to_string()];
        let cmd = parse_args(&args);
        assert!(matches!(cmd, Cmd::Hook));
    }
}

fn handle_session_start(
    hook_input: &event::HookInput,
    resolver: &config::ThemeResolver,
    theme: &str,
    theme_dir: &std::path::Path,
    manifest: &manifest::Manifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_type = hook_input.source.as_deref().unwrap_or("unknown");
    let session_id = if hook_input.session_id.is_empty() {
        "unknown"
    } else {
        &hook_input.session_id
    };
    let startup_flag = PathBuf::from(format!("/tmp/.claude-ringring-{session_id}"));

    match source_type {
        "startup" => {
            resolver.persist_session_theme(theme);
            fs::write(&startup_flag, "startup")?;

            // Deferred startup sound: sleep, then play if flag still exists
            let theme_dir = theme_dir.to_path_buf();
            let flag = startup_flag;

            // Pick sound now, move only what we need into the thread
            let pick = manifest::pick_sound(manifest, "greeting");

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if flag.exists() {
                    if let Some(pick) = pick {
                        let sound_path = theme_dir.join("sounds").join(&pick.file);
                        let _ = audio::play_sound(&sound_path);
                    }
                    let _ = fs::remove_file(&flag);
                }
            })
            .join()
            .ok();
        }
        "resume" => {
            let _ = fs::remove_file(&startup_flag);
        }
        _ => {}
    }

    Ok(())
}
