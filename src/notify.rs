use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

static ICON_BYTES: &[u8] = include_bytes!("../clawd.png");

static NOTIFICATION_ID: AtomicU32 = AtomicU32::new(0);

const APP_ID: &str = "claude-code";
const DESKTOP_ENTRY: &str = "[Desktop Entry]
Name=Claude Code
Icon=claude-code
Type=Application
NoDisplay=true
";

fn icon_path() -> &'static PathBuf {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let path = PathBuf::from("/tmp/.claude-ringring-icon.png");
        let _ = std::fs::write(&path, ICON_BYTES);
        path
    })
}

/// Ensure the .desktop file exists so GNOME can identify our app for stacking.
fn ensure_desktop_entry() {
    static DONE: OnceLock<()> = OnceLock::new();
    DONE.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            return;
        }

        let apps_dir = PathBuf::from(&home).join(".local/share/applications");
        let _ = std::fs::create_dir_all(&apps_dir);
        let desktop_path = apps_dir.join("claude-code.desktop");
        let _ = std::fs::write(&desktop_path, DESKTOP_ENTRY);

        // Also install the icon so GNOME can find it
        let icons_dir = PathBuf::from(&home).join(".local/share/icons/hicolor/128x128/apps");
        let _ = std::fs::create_dir_all(&icons_dir);
        let icon_dest = icons_dir.join("claude-code.png");
        if !icon_dest.exists() {
            let _ = std::fs::write(&icon_dest, ICON_BYTES);
        }
    });
}

/// Send a desktop notification via org.gtk.Notifications (stacks in GNOME).
/// Falls back to freedesktop notifications if gdbus fails.
pub fn send_notification(title: &str, body: &str) {
    ensure_desktop_entry();
    let icon = icon_path().to_string_lossy();

    let notif_id = NOTIFICATION_ID.fetch_add(1, Ordering::Relaxed);
    let id = format!("ringring-{}-{}", std::process::id(), notif_id);

    // Try GTK notification path first (groups by app ID, enables stacking)
    let variant = format!(
        "{{'title': <'{}'>, 'body': <'{}'>, 'icon': <('file-icon', <'{}'>)>}}",
        escape_gvariant(title),
        escape_gvariant(body),
        icon,
    );

    let result = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest", "org.gtk.Notifications",
            "--object-path", "/org/gtk/Notifications",
            "--method", "org.gtk.Notifications.AddNotification",
            APP_ID,
            &id,
            &variant,
        ])
        .output();

    if result.is_ok_and(|o| o.status.success()) {
        return;
    }

    // Fallback to freedesktop notifications
    let _ = notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon(&icon)
        .image_path(&icon)
        .appname("Claude Code")
        .hint(notify_rust::Hint::DesktopEntry(APP_ID.to_string()))
        .show();
}

fn escape_gvariant(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "'\\''")
}
