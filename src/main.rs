mod audio;
mod config;
mod event;
mod manifest;
mod notify;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    if let Err(_) = run() {
        // Silent failure — hooks must never block Claude Code
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Read stdin
    let mut input_str = String::new();
    std::io::stdin().read_to_string(&mut input_str)?;

    let hook_input: event::HookInput = serde_json::from_str(&input_str)?;

    let home = std::env::var("HOME")?;
    let sounds_dir = PathBuf::from(&home).join(".claude/sounds");

    // Resolve theme
    let cfg = config::Config::load(&sounds_dir);
    let resolver = config::ThemeResolver {
        sounds_dir: &sounds_dir,
        config: &cfg,
        session_id: &hook_input.session_id,
        cwd: std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    };
    let theme = resolver.resolve();
    let theme_dir = config::theme_dir(&sounds_dir, &theme);
    let manifest_path = theme_dir.join("manifest.json");

    // If no manifest, nothing to do
    if !manifest_path.exists() {
        return Ok(());
    }

    let manifest = match manifest::Manifest::load(&theme_dir) {
        Some(m) => m,
        None => return Ok(()),
    };

    // Map event to action
    let action = event::map_event(&hook_input);

    // Handle SessionStart special logic
    if hook_input.hook_event_name == "SessionStart" {
        return handle_session_start(
            &hook_input,
            &resolver,
            &theme,
            &theme_dir,
            &manifest,
        );
    }

    // Normal event handling
    if let Some(ref category) = action.category {
        let pick = manifest::pick_sound(&manifest, category);
        let (cat_title, cat_body) = manifest::category_text(&manifest, category);

        // Resolve title: manifest > hardcoded
        let title = cat_title.unwrap_or(action.title);

        // Resolve body: sound line > manifest body > hardcoded
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

fn handle_session_start(
    hook_input: &event::HookInput,
    resolver: &config::ThemeResolver,
    theme: &str,
    theme_dir: &std::path::Path,
    manifest: &manifest::Manifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let source_type = hook_input.source.as_deref().unwrap_or("unknown");
    let startup_flag = PathBuf::from(format!(
        "/tmp/.claude-ringring-{}",
        if hook_input.session_id.is_empty() {
            "unknown"
        } else {
            &hook_input.session_id
        }
    ));

    match source_type {
        "startup" => {
            // Persist theme for session
            resolver.persist_session_theme(theme);

            // Write startup flag
            fs::write(&startup_flag, "startup")?;

            // Deferred startup sound: sleep, then play if flag still exists
            let theme_dir = theme_dir.to_path_buf();
            let manifest_json = serde_json::to_string(manifest)?;
            let flag = startup_flag.clone();

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                if flag.exists() {
                    if let Ok(m) = serde_json::from_str::<manifest::Manifest>(&manifest_json) {
                        if let Some(pick) = manifest::pick_sound(&m, "greeting") {
                            let sound_path = theme_dir.join("sounds").join(&pick.file);
                            let _ = audio::play_sound(&sound_path);
                        }
                    }
                    let _ = fs::remove_file(&flag);
                }
            })
            .join()
            .ok();
        }
        "resume" => {
            // Cancel pending startup sound
            let _ = fs::remove_file(&startup_flag);
        }
        _ => {}
    }

    Ok(())
}
