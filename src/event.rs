use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HookInput {
    #[serde(default = "default_unknown")]
    pub hook_event_name: String,
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
}

fn default_unknown() -> String {
    "unknown".to_string()
}

/// Result of mapping a hook event to display/sound parameters.
pub struct EventAction {
    pub category: Option<String>,
    pub title: String,
    pub body: String,
    pub skip_notify: bool,
    /// For SessionStart: "startup", "resume", or other
    #[allow(dead_code)]
    pub session_start_type: Option<String>,
}

impl EventAction {
    fn new(category: &str, title: &str, body: &str) -> Self {
        Self {
            category: Some(category.into()),
            title: title.into(),
            body: body.into(),
            skip_notify: false,
            session_start_type: None,
        }
    }

    fn silent(category: Option<&str>) -> Self {
        Self {
            category: category.map(Into::into),
            title: String::new(),
            body: String::new(),
            skip_notify: true,
            session_start_type: None,
        }
    }
}

pub fn map_event(input: &HookInput) -> EventAction {
    match input.hook_event_name.as_str() {
        "SessionStart" => {
            let source_type = input.source.as_deref().unwrap_or("unknown");
            let category = match source_type {
                "startup" | "resume" => Some("greeting"),
                _ => None,
            };
            EventAction {
                session_start_type: Some(source_type.into()),
                ..EventAction::silent(category)
            }
        }
        "PermissionRequest" => EventAction {
            skip_notify: true,
            ..EventAction::new("permission", "Potřebuju povolení", "Something need doing?")
        },
        "Stop" => EventAction::new("complete", "Hotovo", "Okie dokie."),
        "Notification" => map_notification(input),
        _ => EventAction::new("resource_limit", "Neznámá událost", "Why not?"),
    }
}

fn map_notification(input: &HookInput) -> EventAction {
    match input.notification_type.as_deref().unwrap_or("unknown") {
        "permission_prompt" => {
            EventAction::new("permission", "Chtěl bych trochu pozornosti", "Hmm?")
        }
        "idle_prompt" => {
            EventAction::new("annoyed", "Čekám na tebe", "Nudím se, pojď makat.")
        }
        "auth_success" => {
            EventAction::new("acknowledge", "Přihlášení úspěšné", "Be happy to.")
        }
        "elicitation_dialog" => {
            EventAction::new("permission", "Mám otázku", "What you want?")
        }
        _ => EventAction::new("greeting", "Chtěl bych trochu pozornosti", "Yes?"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> HookInput {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn stop_maps_to_complete() {
        let input = parse(r#"{"hook_event_name": "Stop", "session_id": "abc"}"#);
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("complete"));
        assert!(!action.skip_notify);
    }

    #[test]
    fn permission_request_skips_notify() {
        let input = parse(r#"{"hook_event_name": "PermissionRequest"}"#);
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("permission"));
        assert!(action.skip_notify);
    }

    #[test]
    fn session_start_startup() {
        let input = parse(r#"{"hook_event_name": "SessionStart", "source": "startup"}"#);
        let action = map_event(&input);
        assert_eq!(action.session_start_type.as_deref(), Some("startup"));
        assert!(action.skip_notify);
    }

    #[test]
    fn notification_idle_prompt() {
        let input = parse(
            r#"{"hook_event_name": "Notification", "notification_type": "idle_prompt"}"#,
        );
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("annoyed"));
    }

    #[test]
    fn notification_unknown_type_defaults_to_greeting() {
        let input = parse(
            r#"{"hook_event_name": "Notification", "notification_type": "some_new_thing"}"#,
        );
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("greeting"));
    }

    #[test]
    fn unknown_event_maps_to_resource_limit() {
        let input = parse(r#"{"hook_event_name": "SomeFutureEvent"}"#);
        let action = map_event(&input);
        assert_eq!(action.category.as_deref(), Some("resource_limit"));
    }
}
