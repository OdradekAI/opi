//! Mock adapter binary for adapter_host tests.
//!
//! This is a `harness = false` test binary that acts as a child process
//! adapter. Controlled via the `OPI_ADAPTER_TEST_MODE` environment variable.
//!
//! Modes:
//! - `capabilities` — full adapter: responds to initialize, tool_call,
//!   command, hook, state_serialize, state_restore, shutdown
//! - `hang` — reads initialize but never responds (timeout test)
//! - `crash` — reads initialize then exits with code 1 (crash test)
//! - `crash_on_request` — sends capabilities, then exits on the next request
//!   (post-handshake crash test)
//! - `hang_request` — responds to initialize, then never responds again
//!   (per-request timeout test)

use std::io::{BufRead, Write};

fn main() {
    let mode = std::env::var("OPI_ADAPTER_TEST_MODE").unwrap_or_else(|_| "capabilities".into());
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    match mode.as_str() {
        "capabilities" => run_capabilities(&mut reader, &mut writer),
        "hang" => run_hang(&mut reader),
        "crash" => run_crash(&mut reader),
        "crash_on_request" => run_crash_on_request(&mut reader, &mut writer),
        "hang_request" => run_hang_request(&mut reader, &mut writer),
        "gate" => run_gate(&mut reader, &mut writer),
        "prepare" => run_prepare(&mut reader, &mut writer),
        "transform" => run_transform(&mut reader, &mut writer),
        "event_backpressure" => run_event_backpressure(&mut reader, &mut writer),
        "shutdown_marker" => run_shutdown_marker(&mut reader, &mut writer),
        "cancel_tool" => run_cancel_tool(&mut reader, &mut writer),
        _ => std::process::exit(1),
    }
}

fn read_line(reader: &mut impl BufRead) -> Option<String> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line.trim().to_string()),
        Err(_) => None,
    }
}

fn write_msg(writer: &mut impl Write, value: &serde_json::Value) {
    let json = serde_json::to_string(value).unwrap();
    writer.write_all(json.as_bytes()).unwrap();
    writer.write_all(b"\n").unwrap();
    writer.flush().unwrap();
}

fn run_capabilities(reader: &mut impl BufRead, writer: &mut impl Write) {
    // Handle initialize handshake
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        if msg["type"].as_str() != Some("initialize") {
            return;
        }
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [{
                    "name": "test_tool",
                    "description": "A test tool",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "input": {"type": "string"}
                        }
                    }
                }],
                "commands": [{
                    "name": "test/status",
                    "description": "Get status"
                }],
                "hooks": ["before_tool_call", "event"],
                "model_overrides": []
            }),
        );
    }

    // Handle subsequent messages
    while let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let msg_type = msg["type"].as_str().unwrap_or("");
        let id = msg["id"].as_str().unwrap_or("").to_string();

        match msg_type {
            "tool_call" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "tool_result",
                        "id": id,
                        "content": [{"type": "text", "text": "mock_result"}],
                        "is_error": false
                    }),
                );
            }
            "command" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "command_result",
                        "id": id,
                        "data": {"status": "ok"}
                    }),
                );
            }
            "hook" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "hook_result",
                        "id": id,
                        "action": "continue",
                        "data": null
                    }),
                );
            }
            "state_serialize" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "state_result",
                        "id": id,
                        "state": {"mock": true}
                    }),
                );
            }
            "state_restore" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "state_result",
                        "id": id,
                        "state": {}
                    }),
                );
            }
            "event" | "cancel" => {
                // Fire-and-forget, no response
            }
            "shutdown" => {
                return;
            }
            _ => {}
        }
    }
}

fn run_hang(reader: &mut impl BufRead) {
    // Read the initialize line but never respond
    let mut line = String::new();
    let _ = reader.read_line(&mut line);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn run_crash(reader: &mut impl BufRead) {
    // Read the initialize line then crash
    let mut line = String::new();
    let _ = reader.read_line(&mut line);
    std::process::exit(1);
}

fn run_crash_on_request(reader: &mut impl BufRead, writer: &mut impl Write) {
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(_) => return,
        };
        if msg["type"].as_str() != Some("initialize") {
            return;
        }
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [],
                "commands": [],
                "hooks": [],
                "model_overrides": []
            }),
        );

        // Keep the process alive until the handshake has completed, then
        // terminate without replying to the first normal request.
        let _ = read_line(reader);
    }
}

fn run_hang_request(reader: &mut impl BufRead, writer: &mut impl Write) {
    // Respond to initialize normally, then hang on subsequent requests
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [],
                "commands": [],
                "hooks": [],
                "model_overrides": []
            }),
        );
    }

    // Never respond to subsequent requests
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn run_prepare(reader: &mut impl BufRead, writer: &mut impl Write) {
    run_hook_mode(reader, writer, "prepare_next_turn");
}

fn run_transform(reader: &mut impl BufRead, writer: &mut impl Write) {
    run_hook_mode(reader, writer, "transform_context");
}

fn run_hook_mode(reader: &mut impl BufRead, writer: &mut impl Write, hook_name: &str) {
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        if msg["type"].as_str() != Some("initialize") {
            return;
        }
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [],
                "commands": [],
                "hooks": [hook_name],
                "model_overrides": []
            }),
        );
    }

    while let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let msg_type = msg["type"].as_str().unwrap_or("");
        let id = msg["id"].as_str().unwrap_or("").to_string();

        match (msg_type, msg["hook"].as_str().unwrap_or("")) {
            ("hook", "prepare_next_turn") => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "hook_result",
                        "id": id,
                        "action": "continue",
                        "data": {
                            "extra_messages": [{
                                "type": "Custom",
                                "kind": "adapter_note",
                                "data": {"text": "next turn"},
                                "include_in_llm_context": false
                            }]
                        }
                    }),
                );
            }
            ("hook", "transform_context") => {
                let mut messages = msg["payload"]["messages"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                messages.push(serde_json::json!({
                    "type": "Custom",
                    "kind": "adapter_transform",
                    "data": {"text": "transformed"},
                    "include_in_llm_context": false
                }));
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "hook_result",
                        "id": id,
                        "action": "continue",
                        "data": {"messages": messages}
                    }),
                );
            }
            ("event", _) | ("cancel", _) => {}
            ("shutdown", _) => return,
            _ => {}
        }
    }
}

fn run_event_backpressure(reader: &mut impl BufRead, writer: &mut impl Write) {
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [],
                "commands": [],
                "hooks": ["event"],
                "model_overrides": []
            }),
        );
    }

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn run_shutdown_marker(reader: &mut impl BufRead, writer: &mut impl Write) {
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [],
                "commands": [],
                "hooks": [],
                "model_overrides": []
            }),
        );
    }

    while let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if msg["type"].as_str() == Some("shutdown") {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(path) = std::env::var("OPI_ADAPTER_SHUTDOWN_MARKER") {
                let _ = std::fs::write(path, "shutdown observed");
            }
            return;
        }
    }
}

/// `cancel_tool` mode: advertises a tool, hangs on every `tool_call` (never
/// responds), records an `OPI_ADAPTER_CANCEL_MARKER` file when it receives a
/// `cancel` message, and exits on `shutdown`. Used to prove the adapter
/// best-effort cancel is actually dispatched to the child process and that the
/// bridged tool returns `ToolError::Cancelled` instead of hanging.
fn run_cancel_tool(reader: &mut impl BufRead, writer: &mut impl Write) {
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        if msg["type"].as_str() != Some("initialize") {
            return;
        }
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [{
                    "name": "hanging_tool",
                    "description": "Hangs on every call until cancelled",
                    "input_schema": {"type": "object"}
                }],
                "commands": [],
                "hooks": [],
                "model_overrides": []
            }),
        );
    }

    while let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };
        match msg["type"].as_str() {
            // tool_call: intentionally do NOT respond, so the host's request
            // stays pending and the cancel path is exercised. Keep reading so
            // the cancel/shutdown lines are still observed.
            Some("tool_call") => {}
            Some("cancel") => {
                if let Ok(path) = std::env::var("OPI_ADAPTER_CANCEL_MARKER") {
                    let id = msg["id"].as_str().unwrap_or("cancel");
                    let _ = std::fs::write(path, id);
                }
            }
            Some("shutdown") => return,
            _ => {}
        }
    }
}

/// Gate mode: advertises before_tool_call hook that blocks bash commands
/// containing "rm -rf", and a "gate/status" command. Also supports
/// state_serialize/restore and event hooks.
fn run_gate(reader: &mut impl BufRead, writer: &mut impl Write) {
    // Handle initialize handshake
    if let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => return,
        };
        if msg["type"].as_str() != Some("initialize") {
            return;
        }
        let id = msg["id"].as_str().unwrap_or("1");
        write_msg(
            writer,
            &serde_json::json!({
                "type": "capabilities",
                "id": id,
                "tools": [],
                "commands": [{"name": "gate/status", "description": "Gate status"}],
                "hooks": ["before_tool_call", "event"],
                "model_overrides": []
            }),
        );
    }

    // Handle subsequent messages
    while let Some(line) = read_line(reader) {
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let msg_type = msg["type"].as_str().unwrap_or("");
        let id = msg["id"].as_str().unwrap_or("").to_string();

        match msg_type {
            "hook" => {
                let hook = msg["hook"].as_str().unwrap_or("");
                match hook {
                    "before_tool_call" => {
                        let payload = &msg["payload"];
                        let tool = payload["tool"].as_str().unwrap_or("");
                        let args_str = serde_json::to_string(&payload["args"]).unwrap_or_default();
                        if tool == "bash" && args_str.contains("rm -rf") {
                            write_msg(
                                writer,
                                &serde_json::json!({
                                    "type": "hook_result",
                                    "id": id,
                                    "action": "block",
                                    "data": {"reason": "destructive command blocked"}
                                }),
                            );
                        } else {
                            write_msg(
                                writer,
                                &serde_json::json!({
                                    "type": "hook_result",
                                    "id": id,
                                    "action": "continue",
                                    "data": null
                                }),
                            );
                        }
                    }
                    _ => {
                        write_msg(
                            writer,
                            &serde_json::json!({
                                "type": "hook_result",
                                "id": id,
                                "action": "continue",
                                "data": null
                            }),
                        );
                    }
                }
            }
            "command" => {
                let name = msg["name"].as_str().unwrap_or("");
                if name == "gate/status" {
                    write_msg(
                        writer,
                        &serde_json::json!({
                            "type": "command_result",
                            "id": id,
                            "data": {"active": true, "blocked": 0}
                        }),
                    );
                } else {
                    write_msg(
                        writer,
                        &serde_json::json!({
                            "type": "error",
                            "id": id,
                            "message": format!("unknown command: {name}")
                        }),
                    );
                }
            }
            "state_serialize" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "state_result",
                        "id": id,
                        "state": {"blocked": 0}
                    }),
                );
            }
            "state_restore" => {
                write_msg(
                    writer,
                    &serde_json::json!({
                        "type": "state_result",
                        "id": id,
                        "state": {}
                    }),
                );
            }
            "event" | "cancel" => {
                // Fire-and-forget, no response
            }
            "shutdown" => {
                return;
            }
            _ => {}
        }
    }
}
