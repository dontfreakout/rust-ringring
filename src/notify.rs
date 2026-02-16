use std::path::Path;

/// Send a desktop notification. Silent failure on error.
pub fn send_notification(title: &str, body: &str, icon: &Path) {
    let icon_str = icon.to_string_lossy();
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon(&icon_str)
        .appname("Claude Code")
        .show();
}
