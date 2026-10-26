//! Canonical Context/Transcript chronology contract (P20-CONTEXT closure
//! test, task 20.6 / DEL-03, AIR-008/011/020, V-06/V-31 SDK-active
//! sub-boundary).
//!
//! Pinned reference: `.repo/pi-1.3.0/packages/ai/src/utils/transcript.ts`
//! (Pi AI 1.3.0). This battery asserts the public seam end to end: the
//! convenience shorthand folds once into a leading system entry, replay
//! applies ordered content/section/tool deltas, projection is
//! native-or-collapse (never rewriting the canonical value), strict
//! projection fails before any request is built, declaration installation
//! fails closed, and real adapters consume only the normalized transcript.
//! Host-impact clauses (implementation revisions under AIR-009,
//! tool_search/loadout/resume) are deliberately absent.

use opi_ai::message::{InputContent, Message, ToolDef, UserMessage};
use opi_ai::transcript::{
    Context, SystemMessage, SystemSection, ToolReference, TranscriptContext, TranscriptError,
    TranscriptProjection, collapse_system_messages, get_current_system_message,
    get_current_system_prompt, get_current_tools, get_declared_tools, normalize_context,
    render_system_message_update, resolve_transcript, system_message_text,
    tool_state_change_message,
};

#[path = "transcript_contract/provider_projection.rs"]
mod provider_projection;

fn tool(name: &str, description: &str) -> ToolDef {
    ToolDef {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema: serde_json::json!({"type": "object"}),
        constrained_sampling: None,
    }
}

fn user(text: &str) -> Message {
    Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: text.to_owned(),
        }],
        timestamp_ms: 1,
    })
}

fn change() -> SystemMessage {
    SystemMessage {
        content: String::new(),
        sections: Vec::new(),
        tools_added: Vec::new(),
        tools_removed: Vec::new(),
        timestamp_ms: 10,
    }
}

#[test]
fn serde_cannot_install_invalid_declaration_batches() {
    let mut empty = tool("", "empty");
    empty.input_schema = serde_json::json!({"type":"object"});
    let mut broken = tool("broken", "invalid pattern");
    broken.input_schema = serde_json::json!({"type":"string", "pattern":"["});
    for tools in [
        vec![empty],
        vec![tool("same", "first"), tool("same", "second")],
        vec![broken],
    ] {
        let messages = vec![Message::System(SystemMessage {
            tools_added: tools,
            ..change()
        })];
        let raw = serde_json::to_string(&messages).unwrap();
        assert!(
            serde_json::from_str::<TranscriptContext>(&raw).is_err(),
            "{raw}"
        );
    }
}

#[test]
fn initial_and_embedded_declarations_use_the_same_validation() {
    let mut broken = tool("broken", "invalid pattern");
    broken.input_schema = serde_json::json!({"type":"string", "pattern":"["});
    for tools in [
        vec![tool("", "empty")],
        vec![tool("same", "a"), tool("same", "b")],
        vec![broken],
    ] {
        assert!(
            normalize_context(Context {
                tools: tools.clone(),
                ..Context::default()
            })
            .is_err()
        );
        assert!(
            normalize_context(Context {
                messages: vec![Message::System(SystemMessage {
                    tools_added: tools,
                    ..change()
                })],
                ..Context::default()
            })
            .is_err()
        );
    }
}

/// V-06 SDK-active: the public shorthand folds into exactly one leading
/// transcript entry; an already normalized transcript re-enters unchanged.
#[test]
fn initial_shorthand_folds_once_into_a_leading_entry() {
    let context = Context {
        system_prompt: Some("base prompt".to_owned()),
        messages: vec![user("hello")],
        tools: vec![tool("read", "Read things")],
    };
    let normalized = normalize_context(context).expect("valid context declarations");
    assert_eq!(normalized.messages().len(), 2);
    let head = normalized
        .messages()
        .first()
        .expect("leading system entry exists");
    assert!(matches!(head, Message::System(_)));
    // Replay of the folded state is the identity: one unchanged initial
    // state is lossless under collapse (AIR-011).
    assert_eq!(collapse_system_messages(&normalized), normalized);
}

/// AIR-008: ordered content appends; same-name section replacement keeps
/// position; null deletes; delete-then-readd appends; body and sections stay
/// distinguishable.
#[test]
fn content_appends_and_sections_patch_in_order() {
    let mut transcript = normalize_context(Context {
        system_prompt: Some("base".to_owned()),
        messages: vec![user("q")],
        ..Context::default()
    })
    .expect("valid context declarations");
    transcript
        .append_system_change(SystemMessage {
            content: "add-one".to_owned(),
            sections: vec![
                SystemSection {
                    name: "alpha".to_owned(),
                    value: Some("a1".to_owned()),
                },
                SystemSection {
                    name: "beta".to_owned(),
                    value: Some("b1".to_owned()),
                },
            ],
            ..change()
        })
        .unwrap();
    // Replace alpha (keeps position), delete beta, re-add beta (appends).
    transcript
        .append_system_change(SystemMessage {
            sections: vec![SystemSection {
                name: "alpha".to_owned(),
                value: Some("a2".to_owned()),
            }],
            ..change()
        })
        .unwrap();
    transcript
        .append_system_change(SystemMessage {
            sections: vec![SystemSection {
                name: "beta".to_owned(),
                value: None,
            }],
            ..change()
        })
        .unwrap();
    transcript
        .append_system_change(SystemMessage {
            content: "add-two".to_owned(),
            sections: vec![SystemSection {
                name: "beta".to_owned(),
                value: Some("b2".to_owned()),
            }],
            ..change()
        })
        .unwrap();

    let head = get_current_system_message(transcript.messages()).unwrap();
    assert_eq!(head.content, "base\n\nadd-one\n\nadd-two");
    let names: Vec<&str> = head.sections.iter().map(|s| s.name.as_str()).collect();
    // alpha kept its position; beta moved to the end after delete+readd.
    assert_eq!(names, ["alpha", "beta"]);
    assert_eq!(
        head.sections
            .iter()
            .map(|s| s.value.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("a2"), Some("b2")]
    );
    // Body and sections are distinguishable in the rendered prompt.
    let rendered = get_current_system_prompt(transcript.messages());
    assert_eq!(rendered, "base\n\nadd-one\n\nadd-two\n\na2\n\nb2");
}

/// AIR-008: tool declarations are by value; a redefinition is a removal
/// followed by an addition; equal states record no redundant change.
#[test]
fn tool_declarations_are_by_value_with_real_deltas_only() {
    let mut transcript = normalize_context(Context {
        tools: vec![tool("a", "one"), tool("b", "two")],
        ..Context::default()
    })
    .expect("valid context declarations");
    // Redefine `a` and add `c` through the state-change message; equal
    // state produces None (dedup).
    assert!(tool_state_change_message(&[tool("a", "one")], &[tool("a", "one")], 1).is_none());
    let redefine = tool_state_change_message(
        &[tool("a", "one"), tool("b", "two")],
        &[tool("a", "one-new"), tool("c", "three")],
        20,
    )
    .unwrap();
    transcript.append_system_change(redefine).unwrap();

    let tools = get_current_tools(transcript.messages());
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["a", "c"]);
    assert_eq!(tools[0].description, "one-new");
    // First-declaration order keeps the original position of `a`.
    let declared = get_declared_tools(transcript.messages());
    let declared_names: Vec<&str> = declared.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(declared_names, ["a", "b", "c"]);
}

/// AIR-011: default projection is native-or-collapse; collapse returns a
/// fresh value and never rewrites the canonical chronology; explicit Strict
/// rejects an unplaceable transcript before any request is built.
#[test]
fn projection_is_native_or_collapse_and_never_rewrites_canonical() {
    let mut transcript = normalize_context(Context {
        system_prompt: Some("base".to_owned()),
        messages: vec![user("q")],
        ..Context::default()
    })
    .expect("valid context declarations");
    transcript
        .append_system_change(SystemMessage {
            content: "later".to_owned(),
            ..change()
        })
        .unwrap();
    let canonical_len = transcript.messages().len();

    // Native capability keeps the canonical value by reference.
    let native =
        resolve_transcript(&transcript, true, TranscriptProjection::NativeOrCollapse).unwrap();
    assert!(matches!(native, std::borrow::Cow::Borrowed(_)));

    // Collapse rebuilds a single head and drops later system messages,
    // leaving the canonical transcript untouched.
    let collapsed = collapse_system_messages(&transcript);
    assert_eq!(collapsed.messages().len(), 2); // head + user
    assert_eq!(transcript.messages().len(), canonical_len);
    assert_eq!(
        get_current_system_prompt(collapsed.messages()),
        "base\n\nlater"
    );

    // Strict rejects the temporal placement a collapsing transport cannot
    // honor — before any request is built (before auth/network).
    let error = resolve_transcript(&transcript, false, TranscriptProjection::Strict).unwrap_err();
    assert_eq!(
        error,
        TranscriptError::UnsupportedTemporalFidelity {
            mid_convo_system_messages: 1
        }
    );
    // Strict on a placement-capable transport, or on a transcript with no
    // mid-conversation changes, succeeds.
    resolve_transcript(&transcript, true, TranscriptProjection::Strict).unwrap();
    let fresh = normalize_context(Context {
        system_prompt: Some("only head".to_owned()),
        ..Context::default()
    })
    .expect("valid context declarations");
    resolve_transcript(&fresh, false, TranscriptProjection::Strict).unwrap();
}

/// AIR-008: invalid or ambiguous names and schema compile failures fail
/// before installation; a keyword that is inert under the pinned structural
/// guard is not a metaschema admission error.
#[test]
fn invalid_declarations_fail_before_installation() {
    let inert = ToolDef {
        name: "inert".to_owned(),
        description: "unknown keywords stay inert".to_owned(),
        input_schema: serde_json::json!({"type": "object", "unknownKeyword": {"x": 1}}),
        constrained_sampling: None,
    };
    let mut transcript = normalize_context(Context::default()).expect("valid context declarations");
    transcript
        .append_system_change(SystemMessage {
            tools_added: vec![inert],
            ..change()
        })
        .unwrap(); // inert keywords are not admission errors

    let bad_regex = ToolDef {
        name: "bad".to_owned(),
        description: String::new(),
        input_schema: serde_json::json!({"pattern": "["}),
        constrained_sampling: None,
    };
    let before = transcript.messages().len();
    assert!(
        transcript
            .append_system_change(SystemMessage {
                tools_added: vec![bad_regex],
                ..change()
            })
            .is_err()
    );
    let dup = ToolDef {
        name: String::new(),
        description: String::new(),
        input_schema: serde_json::json!({"type": "object"}),
        constrained_sampling: None,
    };
    assert!(
        transcript
            .append_system_change(SystemMessage {
                tools_added: vec![dup.clone(), dup],
                ..change()
            })
            .is_err()
    );
    assert_eq!(transcript.messages().len(), before); // only the inert batch landed
}

/// AIR-020: adapters consume only the canonical normalized input — the
/// request carries the transcript, not independent system/tools fields, and
/// the Anthropic wire renders the replayed prompt and current tools at the
/// head exactly as the pre-cutover fields did.
#[test]
fn adapters_consume_only_the_normalized_transcript() {
    use opi_ai::anthropic::AnthropicProvider;
    use opi_ai::http::HttpClient;
    use std::sync::Arc;

    let provider = AnthropicProvider::with_client(
        Some("https://api.anthropic.com".into()),
        Arc::new(HttpClient::new()),
    );
    let request = opi_ai::provider::Request {
        transport: Default::default(),
        model: "anthropic:claude-sonnet-4-5-20250514".into(),
        context: normalize_context(Context {
            system_prompt: Some("system text".to_owned()),
            messages: vec![user("hello")],
            tools: vec![tool("read", "Read things")],
        })
        .expect("valid context declarations"),
        max_tokens: Some(64),
        temperature: None,
        sampling_params: Default::default(),
        thinking: opi_ai::provider::ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: tokio_util::sync::CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: opi_ai::provider::CacheRetention::None,
        session_id: None,
        callbacks: Default::default(),
    };
    let body = provider.build_request_body(&request).unwrap();
    // The built-in model enables cache control, so the prompt rides a
    // single-content-block array carrying the ephemeral marker.
    assert_eq!(body["system"][0]["text"], serde_json::json!("system text"));
    assert_eq!(body["tools"][0]["name"], serde_json::json!("read"));
    // The conversation body carries the user message, not the system entry.
    let body_messages = body["messages"].as_array().unwrap();
    assert_eq!(body_messages.len(), 1);
    assert_eq!(body_messages[0]["role"], serde_json::json!("user"));

    // A mid-conversation change collapses onto the head for this wire.
    let mut request = request.clone();
    request
        .context
        .append_system_change(SystemMessage {
            content: "later instruction".to_owned(),
            tools_added: vec![tool("write", "Write things")],
            ..change()
        })
        .unwrap();
    let body = provider.build_request_body(&request).unwrap();
    assert_eq!(
        body["system"][0]["text"],
        serde_json::json!("system text\n\nlater instruction")
    );
    let tool_names: Vec<&str> = body["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(tool_names, ["read", "write"]);
}

/// AIR-020: stored transcripts re-enter through the transparent message
/// list; pre-cutover message chronologies (no system entries) deserialize
/// unchanged — session v1 readers keep working.
#[test]
fn stored_chronologies_round_trip_and_pre_cutover_lists_still_parse() {
    let mut transcript = normalize_context(Context {
        system_prompt: Some("p".to_owned()),
        tools: vec![tool("t", "d")],
        ..Context::default()
    })
    .expect("valid context declarations");
    transcript
        .append_system_change(SystemMessage {
            sections: vec![SystemSection {
                name: "s".to_owned(),
                value: Some("v".to_owned()),
            }],
            tools_removed: vec![ToolReference {
                name: "t".to_owned(),
            }],
            ..change()
        })
        .unwrap();
    let json = serde_json::to_string(&transcript).unwrap();
    let back: TranscriptContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back, transcript);

    // A pre-cutover conversation list (no system entries) deserializes into
    // the canonical holder unchanged.
    let legacy = serde_json::to_string(&vec![user("old")]).unwrap();
    let revived: TranscriptContext = serde_json::from_str(&legacy).unwrap();
    assert_eq!(revived.messages().len(), 1);
    assert!(get_current_tools(revived.messages()).is_empty());
}

/// Native placement renders later system messages as framed updates
/// (request-time only); the canonical entries are untouched.
#[test]
fn native_placement_renders_framed_updates() {
    let mut transcript = normalize_context(Context {
        system_prompt: Some("base".to_owned()),
        ..Context::default()
    })
    .expect("valid context declarations");
    let mut later = change();
    later.content = "new note".to_owned();
    later.sections = vec![SystemSection {
        name: "sec".to_owned(),
        value: Some("value".to_owned()),
    }];
    transcript.append_system_change(later).unwrap();
    let rendered = render_system_message_update(
        transcript
            .messages()
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::System(system) => Some(system),
                _ => None,
            })
            .unwrap(),
    );
    assert_eq!(
        rendered,
        "new note\n\nUpdated system prompt section \"sec\":\n\nvalue"
    );
    // The rendered update and the replayed head agree on live section text.
    let head = get_current_system_message(transcript.messages()).unwrap();
    assert_eq!(system_message_text(&head), "base\n\nnew note\n\nvalue");
}

// --- task 20.11: constrained-sampling declarations on tools ---

mod constrained_tool_serde_tests {
    use opi_ai::constrained_sampling::ConstrainedSampling;
    use opi_ai::constrained_sampling::StrictPreference;
    use opi_ai::message::ToolDef;
    use serde_json::json;

    #[test]
    fn tool_def_constrained_sampling_round_trips() {
        let mut tool = ToolDef {
            name: "t".into(),
            description: "d".into(),
            input_schema: json!({"type": "object"}),
            constrained_sampling: None,
        };
        let legacy = serde_json::to_value(&tool).unwrap();
        assert!(
            legacy.get("constrainedSampling").is_none() || legacy["constrainedSampling"].is_null(),
            "absent config must not serialize a key"
        );
        let back: ToolDef = serde_json::from_value(legacy).unwrap();
        assert_eq!(back, tool);

        tool.constrained_sampling = Some(ConstrainedSampling::JsonSchema {
            strict: StrictPreference::Require,
        });
        let wire = serde_json::to_value(&tool).unwrap();
        assert_eq!(wire["constrained_sampling"]["type"], json!("json_schema"));
        assert_eq!(wire["constrained_sampling"]["strict"], json!("require"));
        let back: ToolDef = serde_json::from_value(wire).unwrap();
        assert_eq!(back, tool);

        tool.constrained_sampling = Some(ConstrainedSampling::Grammar {
            openai_lark: Some("start: A".into()),
            openai_regex: None,
        });
        let wire = serde_json::to_value(&tool).unwrap();
        assert_eq!(wire["constrained_sampling"]["type"], json!("grammar"));
        assert_eq!(
            wire["constrained_sampling"]["openaiLark"],
            json!("start: A")
        );
        let back: ToolDef = serde_json::from_value(wire).unwrap();
        assert_eq!(back, tool);
    }
}
