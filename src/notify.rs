use std::path::PathBuf;
use std::sync::OnceLock;

static ICON_BYTES: &[u8] = include_bytes!("../clawd.png");

fn icon_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = PathBuf::from("/tmp/.claude-ringring-icon.png");
        let _ = std::fs::write(&path, ICON_BYTES);
        path
    })
}

/// Send a desktop notification. Silent failure on error.
pub fn send_notification(title: &str, body: &str) {
    let icon_str = icon_path().to_string_lossy();
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .image_path(&icon_str)
        .appname("Claude Code")
        .show();
}
