mod audio;
mod config;
mod event;
mod manifest;
mod notify;
mod paths;

use std::fs;
use std::path::PathBuf;

enum Cmd {
    Hook,
    Test { theme: String, category: Option<String> },
    List { debug: bool },
}

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
            let debug = args.get(2..).unwrap_or(&[]).iter().any(|a| a == "--debug");
            Cmd::List { debug }
        }
        _ => Cmd::Hook,
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
        Cmd::List { debug } => {
            run_list(debug);
        }
        Cmd::Hook => {
            let _ = run();
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let input_str = std::io::read_to_string(std::io::stdin())?;
    let hook_input: event::HookInput = serde_json::from_str(&input_str)?;

    let sounds_dir = paths::data_dir();

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

fn run_list(debug: bool) {
    let sounds_dir = paths::data_dir();

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

fn run_test(theme: &str, category: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if theme.is_empty() {
        return Err("usage: ringring test <theme> [--category <cat>]".into());
    }

    let sounds_dir = paths::data_dir();
    let theme_dir = config::theme_dir(&sounds_dir, theme);

    let manifest = manifest::Manifest::load(&theme_dir)
        .ok_or_else(|| format!("no manifest found for theme '{theme}'"))?;

    let categories: Vec<(&str, &manifest::Category)> = if let Some(cat) = category {
        let entry = manifest.categories.get(cat)
            .ok_or_else(|| format!("category '{cat}' not found in theme '{theme}'"))?;
        vec![(cat, entry)]
    } else {
        let mut pairs: Vec<(&str, &manifest::Category)> =
            manifest.categories.iter().map(|(k, v)| (k.as_str(), v)).collect();
        pairs.sort_by_key(|(k, _)| *k);
        pairs
    };

    for (cat_name, cat) in &categories {
        // Preview mode: play every sound in the category, not a random pick.
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

    #[test]
    fn parse_test_args_missing_theme() {
        let args = vec!["ringring".to_string(), "test".to_string()];
        let cmd = parse_args(&args);
        // theme will be empty string — run_test handles the error
        assert!(matches!(cmd, Cmd::Test { ref theme, .. } if theme.is_empty()));
    }

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
