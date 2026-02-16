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
    pub session_start_type: Option<String>,
}

pub fn map_event(input: &HookInput) -> EventAction {
    match input.hook_event_name.as_str() {
        "SessionStart" => {
            let source_type = input.source.as_deref().unwrap_or("unknown");
            let session_start_type = Some(source_type.to_string());
            match source_type {
                "startup" | "resume" => EventAction {
                    category: Some("greeting".to_string()),
                    title: String::new(),
                    body: String::new(),
                    skip_notify: true,
                    session_start_type,
                },
                _ => EventAction {
                    category: None,
                    title: String::new(),
                    body: String::new(),
                    skip_notify: true,
                    session_start_type,
                },
            }
        }
        "PermissionRequest" => EventAction {
            category: Some("permission".to_string()),
            title: "Potřebuju povolení".to_string(),
            body: "Something need doing?".to_string(),
            skip_notify: true,
            session_start_type: None,
        },
        "Stop" => EventAction {
            category: Some("complete".to_string()),
            title: "Hotovo".to_string(),
            body: "Okie dokie.".to_string(),
            skip_notify: false,
            session_start_type: None,
        },
        "Notification" => {
            let nt = input.notification_type.as_deref().unwrap_or("unknown");
            match nt {
                "permission_prompt" => EventAction {
                    category: Some("permission".to_string()),
                    title: "Chtěl bych trochu pozornosti".to_string(),
                    body: "Hmm?".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                "idle_prompt" => EventAction {
                    category: Some("annoyed".to_string()),
                    title: "Čekám na tebe".to_string(),
                    body: "Nudím se, pojď makat.".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                "auth_success" => EventAction {
                    category: Some("acknowledge".to_string()),
                    title: "Přihlášení úspěšné".to_string(),
                    body: "Be happy to.".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                "elicitation_dialog" => EventAction {
                    category: Some("permission".to_string()),
                    title: "Mám otázku".to_string(),
                    body: "What you want?".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
                _ => EventAction {
                    category: Some("greeting".to_string()),
                    title: "Chtěl bych trochu pozornosti".to_string(),
                    body: "Yes?".to_string(),
                    skip_notify: false,
                    session_start_type: None,
                },
            }
        }
        _ => EventAction {
            category: Some("resource_limit".to_string()),
            title: "Neznámá událost".to_string(),
            body: "Why not?".to_string(),
            skip_notify: false,
            session_start_type: None,
        },
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
