//! Tests for system prompt construction (task 1.11).
//!
//! DoD: "prompt includes tool defs and system layer"

use opi_ai::message::ToolDef;
use opi_coding_agent::prompt::SystemPromptBuilder;
use serde_json::json;

fn tool_def(name: &str, description: &str) -> ToolDef {
    ToolDef {
        name: name.into(),
        description: description.into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "arg": { "type": "string" }
            },
            "required": ["arg"]
        }),
        constrained_sampling: None,
    }
}

// ---------------------------------------------------------------------------
// Base prompt layer
// ---------------------------------------------------------------------------

#[test]
fn builder_always_includes_base_instructions() {
    let prompt = SystemPromptBuilder::new().build();
    assert!(!prompt.is_empty(), "system prompt should never be empty");
    assert!(
        prompt.contains("coding"),
        "base prompt should mention coding role"
    );
}

// ---------------------------------------------------------------------------
// Tool description layer
// ---------------------------------------------------------------------------

#[test]
fn tool_names_appear_in_prompt() {
    let tools = vec![
        tool_def("read", "Read a file"),
        tool_def("bash", "Run a command"),
    ];
    let prompt = SystemPromptBuilder::new().tools(tools.clone()).build();

    assert!(prompt.contains("read"), "prompt should mention tool 'read'");
    assert!(prompt.contains("bash"), "prompt should mention tool 'bash'");
}

#[test]
fn tool_descriptions_appear_in_prompt() {
    let tools = vec![tool_def("read", "Read file content from disk")];
    let prompt = SystemPromptBuilder::new().tools(tools.clone()).build();

    assert!(
        prompt.contains("Read file content from disk"),
        "prompt should contain tool description text"
    );
}

#[test]
fn no_tools_means_no_tool_section() {
    let prompt = SystemPromptBuilder::new().build();
    assert!(
        !prompt.contains("Available tools"),
        "no tool section header when no tools provided"
    );
}

// ---------------------------------------------------------------------------
// User system prompt layer
// ---------------------------------------------------------------------------

#[test]
fn user_system_prompt_appended() {
    let prompt = SystemPromptBuilder::new()
        .user_system("Always use British spelling.")
        .build();

    assert!(
        prompt.contains("Always use British spelling."),
        "user system prompt should appear in output"
    );
}

#[test]
fn no_user_system_means_no_user_section() {
    let prompt = SystemPromptBuilder::new().build();
    assert!(
        !prompt.contains("User instructions"),
        "no user section when no user system prompt provided"
    );
}

// ---------------------------------------------------------------------------
// Layer ordering
// ---------------------------------------------------------------------------

#[test]
fn layers_in_correct_order() {
    let tools = vec![tool_def("read", "Read a file")];
    let prompt = SystemPromptBuilder::new()
        .tools(tools)
        .user_system("Custom instructions")
        .build();

    let base_pos = prompt.find("coding").expect("base instructions present");
    let tool_pos = prompt.find("read").expect("tool description present");
    let user_pos = prompt
        .find("Custom instructions")
        .expect("user system present");

    assert!(
        base_pos < tool_pos,
        "base instructions should come before tool descriptions"
    );
    assert!(
        tool_pos < user_pos,
        "tool descriptions should come before user system prompt"
    );
}

// ---------------------------------------------------------------------------
// Tool definitions collected for Request.tools
// ---------------------------------------------------------------------------

#[test]
fn tool_defs_collected_for_request() {
    let tools = vec![
        tool_def("read", "Read a file"),
        tool_def("bash", "Run a command"),
    ];
    let builder = SystemPromptBuilder::new().tools(tools.clone());

    let collected = builder.tool_definitions();
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].name, "read");
    assert_eq!(collected[1].name, "bash");
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_user_system_is_ignored() {
    let prompt = SystemPromptBuilder::new().user_system("").build();

    assert!(
        !prompt.contains("User instructions"),
        "empty user system should not create section"
    );
}

#[test]
fn multiple_tools_all_appear() {
    let tools = vec![
        tool_def("read", "Read a file"),
        tool_def("write", "Write a file"),
        tool_def("edit", "Edit a file"),
        tool_def("bash", "Run a command"),
        tool_def("glob", "Find files"),
        tool_def("grep", "Search content"),
    ];
    let prompt = SystemPromptBuilder::new().tools(tools.clone()).build();

    for tool in &tools {
        assert!(
            prompt.contains(&tool.name),
            "prompt should mention tool '{}'",
            tool.name
        );
    }
}
