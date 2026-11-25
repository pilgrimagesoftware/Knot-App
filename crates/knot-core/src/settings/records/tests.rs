use super::*;

fn id() -> Uuid {
    Uuid::new_v4()
}

#[test]
fn persona_enum_round_trips_lowercase() {
    assert_eq!(serde_json::to_string(&PersonaType::System).unwrap(),
               "\"system\"");
    assert_eq!(serde_json::to_string(&PersonaState::Deleted).unwrap(),
               "\"deleted\"");
    let parsed: PersonaState = serde_json::from_str("\"enabled\"").unwrap();
    assert_eq!(parsed, PersonaState::Enabled);
}

#[test]
fn legacy_persona_defaults_type_and_state() {
    let json = r#"{"id":"a1000001-0000-0000-0000-000000000001","name":"X","instructions":"do X"}"#;
    let persona: Persona = serde_json::from_str(json).unwrap();
    assert_eq!(persona.persona_type, PersonaType::User);
    assert_eq!(persona.state, PersonaState::Enabled);
}

#[test]
fn legacy_saved_agent_defaults_added_fields() {
    let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                       id());
    let agent: SavedAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(agent.agent_type, "claude");
    assert_eq!(agent.created_by, None);
    assert!(!agent.is_companion);
    assert_eq!(agent.persona_id, None);
    assert_eq!(agent.session_id, None);
    assert!(agent.session_config.is_empty());
}

#[test]
fn saved_agent_session_config_round_trips() {
    let mut agent = SavedAgent::new(id(), "A", None, "/tmp");
    agent.session_config
         .insert("model".to_string(), "sonnet".to_string());
    agent.session_config
         .insert("permission_mode".to_string(), "plan".to_string());
    agent.session_config
         .insert("reasoning-effort".to_string(), "high".to_string());

    let json = serde_json::to_string(&agent).unwrap();
    assert!(json.contains("sessionConfig"), "camelCase key: {json}");

    let decoded: SavedAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.session_config, agent.session_config);
}

#[test]
fn legacy_saved_agent_without_activation_mode_loads_active() {
    let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                       id());
    let agent: SavedAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(agent.activation_mode, ActivationMode::Active);
}

#[test]
fn activation_mode_round_trips_in_lowercase() {
    assert_eq!(serde_json::to_string(&ActivationMode::Passive).unwrap(),
               "\"passive\"");
    let mut a = SavedAgent::new(id(), "A", None, "/tmp");
    a.activation_mode = ActivationMode::Passive;
    let json = serde_json::to_string(&a).unwrap();
    assert!(json.contains("\"activationMode\":\"passive\""), "{json}");
    let back: SavedAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, a);
}

#[test]
fn saved_agent_session_id_round_trips() {
    let mut a = SavedAgent::new(id(), "A", None, "/tmp");
    a.session_id = Some("s7".to_string());
    let json = serde_json::to_string(&a).unwrap();
    let back: SavedAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(back.session_id, Some("s7".to_string()));
}

#[test]
fn saved_agent_new_defaults_avatar() {
    let a = SavedAgent::new(id(), "A", None, "/tmp");
    assert_eq!(a.avatar, DEFAULT_AVATAR);
    let b = SavedAgent::new(id(), "B", Some(String::new()), "/tmp");
    assert_eq!(b.avatar, DEFAULT_AVATAR);
    let c = SavedAgent::new(id(), "C", Some("🦀".to_string()), "/tmp");
    assert_eq!(c.avatar, "🦀");
}

#[test]
fn legacy_bench_agent_defaults() {
    let json = format!(r#"{{"id":"{}","name":"A","avatar":"x","folder":"/tmp"}}"#,
                       id());
    let bench: BenchAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(bench.agent_type, "claude");
    assert_eq!(bench.persona_id, None);
}

#[test]
fn workspace_round_trips() {
    let ws = Workspace { id:        id(),
                         name:      "Main".to_string(),
                         color_hex: "#1B4FB2".to_string(),
                         agent_ids: vec![id()], };
    let json = serde_json::to_string(&ws).unwrap();
    let back: Workspace = serde_json::from_str(&json).unwrap();
    assert_eq!(ws, back);
}

#[test]
fn workspace_ui_state_round_trips() {
    let ui = WorkspaceUiState { layout_mode:           LayoutMode::GridFourPane,
                                active_agent_ids:      vec![id()],
                                focused_pane_index:    1,
                                split_ratio:           0.5,
                                split_ratio_secondary: Some(0.3),
                                show_dashboard:        Some(false),
                                is_detached:           Some(true),
                                window_bounds:         Some(SavedWindowBounds { x:      12.,
                                                                                y:      34.,
                                                                                width:  960.,
                                                                                height: 640., }),
                                open_at_quit:          true,
                                schedule_runs_handled: BTreeMap::new(), };
    let json = serde_json::to_string(&ui).unwrap();
    let back: WorkspaceUiState = serde_json::from_str(&json).unwrap();
    assert_eq!(ui, back);
}

/// The wire keys are what an existing `workspaces.json` was written with,
/// so the split can only find them under these exact names.
#[test]
fn workspace_ui_state_keeps_the_camel_case_keys_it_was_stored_under() {
    let json = serde_json::to_value(WorkspaceUiState::default()).unwrap();
    let object = json.as_object().unwrap();
    for key in ["layoutMode",
                "activeAgentIds",
                "focusedPaneIndex",
                "splitRatio",
                "splitRatioSecondary",
                "showDashboard",
                "isDetached",
                "windowBounds",
                "openAtQuit"]
    {
        assert!(object.contains_key(key), "UI state missing key {key}");
    }
}

// --- SavedPullRequest --------------------------------------------------

#[test]
fn saved_pull_request_round_trips_in_camel_case() {
    let record = SavedPullRequest { url:
                                        "https://github.com/acme/widget/pull/42".to_string(),
                                    agent_id:     id(),
                                    workspace_id: id(),
                                    first_seen:   1_758_566_400, };

    let json = serde_json::to_string(&record).unwrap();
    assert!(json.contains("\"agentId\""), "camelCase key: {json}");
    assert!(json.contains("\"workspaceId\""), "camelCase key: {json}");
    assert!(json.contains("\"firstSeen\""), "camelCase key: {json}");

    let back: SavedPullRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(back, record);
}

/// The record is evidence of a sighting, not a copy of the pull request.
/// A status persisted here would be shown as current after a restart
/// when it is not.
#[test]
fn saved_pull_request_carries_no_fetched_state() {
    let json = serde_json::to_string(&SavedPullRequest::new("https://github.com/a/b/pull/1",
                                                            id(),
                                                            id())).unwrap();

    for absent in ["title", "number", "state", "status", "isDraft"] {
        assert!(!json.contains(absent),
                "{absent} must not be persisted: {json}");
    }
}

#[test]
fn a_new_saved_pull_request_is_stamped_with_the_current_time() {
    let record = SavedPullRequest::new("https://github.com/a/b/pull/1", id(), id());

    // 2026-01-01, comfortably in the past whenever this runs.
    assert!(record.first_seen > 1_767_225_600, "{}", record.first_seen);
}

/// Idempotence is per agent: the same URL from a second agent is a second
/// record, because who opened it is part of what is recorded.
#[test]
fn a_sighting_matches_only_the_same_url_from_the_same_agent() {
    let agent = id();
    let other_agent = id();
    let record = SavedPullRequest::new("https://github.com/a/b/pull/1", agent, id());

    assert!(record.is_same_sighting("https://github.com/a/b/pull/1", agent));
    assert!(!record.is_same_sighting("https://github.com/a/b/pull/2", agent));
    assert!(!record.is_same_sighting("https://github.com/a/b/pull/1", other_agent));
}

/// The defaults are what every record written before the split already
/// holds. Deriving `Default` would give `0.0` for the ratio instead, which
/// is a divider at the far edge - a change
/// to what the state means, not just to where it is stored.
#[test]
fn ui_state_defaults_match_what_existing_records_hold() {
    let ui = WorkspaceUiState::default();

    assert_eq!(ui.layout_mode, LayoutMode::Single);
    assert_eq!(ui.split_ratio, 0.5);

    // And an entry that names neither reads the same way, rather than
    // taking serde's zero values.
    let decoded: WorkspaceUiState = serde_json::from_str("{}").unwrap();
    assert_eq!(decoded, ui);
}

/// UI state saved before window frames were remembered must still load -
/// the field is absent from every entry written before it existed.
#[test]
fn ui_state_without_saved_window_bounds_still_loads() {
    let ui: WorkspaceUiState = serde_json::from_str(r#"{"layoutMode":"single"}"#).unwrap();

    assert_eq!(ui.layout_mode, LayoutMode::Single);
    assert_eq!(ui.window_bounds, None);
}

/// A workspace record still carrying the UI keys - every existing
/// `workspaces.json` - must load as a workspace, because the split reads
/// the document as JSON and decodes each record after lifting them out.
#[test]
fn a_combined_workspace_record_still_decodes_as_a_workspace() {
    let json = r##"{"id":"00000000-0000-0000-0000-000000000001","name":"Main",
                    "colorHex":"#1B4FB2","layoutMode":"gridFourPane","splitRatio":0.25}"##;

    let ws: Workspace = serde_json::from_str(json).unwrap();

    assert_eq!(ws.name, "Main");
    assert!(ws.agent_ids.is_empty());
}

/// Every mode is stored as the Swift app's `LayoutMode` raw value, so a
/// record written by either app reads back as the same mode.
#[test]
fn each_layout_mode_round_trips_under_its_swift_name() {
    let names = [(LayoutMode::Single, "single"),
                 (LayoutMode::SplitVertical, "splitVertical"),
                 (LayoutMode::SplitHorizontal, "splitHorizontal"),
                 (LayoutMode::ThreePane, "threePane"),
                 (LayoutMode::GridFourPane, "gridFourPane")];
    assert_eq!(names.len(), LayoutMode::ALL.len());
    for (mode, name) in names {
        let ui = WorkspaceUiState { layout_mode: mode,
                                    ..WorkspaceUiState::default() };
        let json = serde_json::to_value(&ui).unwrap();
        assert_eq!(json["layoutMode"], name);
        let back: WorkspaceUiState = serde_json::from_value(json).unwrap();
        assert_eq!(back.layout_mode, mode);
    }
}

/// A mode this build does not know - a misspelling, or one a later version
/// added - reads as single rather than failing the whole document.
#[test]
fn an_unknown_layout_mode_loads_as_single() {
    let ui: WorkspaceUiState =
        serde_json::from_str(r#"{"layoutMode":"grid","splitRatio":0.3}"#).unwrap();

    assert_eq!(ui.layout_mode, LayoutMode::Single);
    assert_eq!(ui.split_ratio, 0.3, "the rest of the entry still loads");
}
