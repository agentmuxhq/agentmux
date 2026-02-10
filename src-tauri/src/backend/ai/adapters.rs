// Copyright 2025, Command Line Inc.
// SPDX-License-Identifier: Apache-2.0

//! Backend adapters: normalize different AI backend output into UnifiedMessage format.
//!
//! Two adapter families:
//! - **ChatAdapter**: Converts Wave AI's existing SSE/streaming events into UnifiedMessage parts
//! - **AgentAdapter**: Converts agent subprocess NDJSON events into UnifiedMessage parts
//!
//! Each adapter translates its native event format into `UnifiedMessagePart` values,
//! which the unified AI pane renders identically regardless of source.

use serde::{Deserialize, Serialize};

use super::unified::{
    UnifiedMessage, UnifiedMessagePart, BACKEND_TYPE_AGENT, BACKEND_TYPE_CHAT, MSG_STATUS_COMPLETE,
    MSG_STATUS_ERROR, MSG_STATUS_STREAMING, TOOL_APPROVAL_AUTO, TOOL_APPROVAL_PENDING,
};
use super::{AIStreamEvent, ToolUseData};

// ---- Adapter events (output from adapters) ----

/// Events produced by adapters, consumed by the unified pane controller.
///
/// These are the normalized "actions" that update the UnifiedMessage state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AdapterEvent {
    /// Start a new assistant message (streaming).
    #[serde(rename = "message_start")]
    MessageStart {
        message_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },

    /// Append text to the current message.
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    /// Append reasoning/thinking text.
    #[serde(rename = "reasoning_delta")]
    ReasoningDelta { text: String },

    /// Tool use started.
    #[serde(rename = "tool_use_start")]
    ToolUseStart {
        call_id: String,
        name: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        summary: String,
    },

    /// Tool use input available.
    #[serde(rename = "tool_use_input")]
    ToolUseInput {
        call_id: String,
        input: serde_json::Value,
    },

    /// Tool use needs approval.
    #[serde(rename = "tool_approval_needed")]
    ToolApprovalNeeded {
        call_id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Tool result received.
    #[serde(rename = "tool_result")]
    ToolResult {
        call_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },

    /// Message complete.
    #[serde(rename = "message_end")]
    MessageEnd {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<super::unified::TokenUsage>,
    },

    /// Error occurred.
    #[serde(rename = "error")]
    Error { message: String },

    /// Session started (from Claude Code system init event).
    #[serde(rename = "session_start")]
    SessionStart {
        session_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default)]
        tools: Vec<String>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        cwd: String,
    },

    /// Session ended (from Claude Code result event).
    #[serde(rename = "session_end")]
    SessionEnd {
        #[serde(default)]
        total_cost_usd: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<super::unified::TokenUsage>,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        num_turns: i32,
        #[serde(default)]
        duration_ms: i64,
    },
}

// ---- Chat adapter: converts AIStreamEvent -> AdapterEvent ----

/// Converts Wave AI's existing `AIStreamEvent` values into `AdapterEvent` values.
///
/// This allows the existing chat backends (OpenAI, Anthropic, etc.) to
/// work with the unified AI pane without modifying their streaming logic.
pub fn adapt_chat_stream_event(event: &AIStreamEvent) -> Vec<AdapterEvent> {
    match event {
        AIStreamEvent::Start { messageid } => {
            vec![AdapterEvent::MessageStart {
                message_id: messageid.clone(),
                model: None,
            }]
        }
        AIStreamEvent::TextDelta { delta, .. } => {
            vec![AdapterEvent::TextDelta {
                text: delta.clone(),
            }]
        }
        AIStreamEvent::ReasoningDelta { delta, .. } => {
            vec![AdapterEvent::ReasoningDelta {
                text: delta.clone(),
            }]
        }
        AIStreamEvent::ToolInputAvailable {
            callid,
            toolname,
            input,
        } => {
            vec![AdapterEvent::ToolUseStart {
                call_id: callid.clone(),
                name: toolname.clone(),
                summary: String::new(),
            }, AdapterEvent::ToolUseInput {
                call_id: callid.clone(),
                input: input.clone(),
            }]
        }
        AIStreamEvent::DataToolUse { data } => {
            if data.approval == super::tools::APPROVAL_NEEDS_APPROVAL {
                vec![AdapterEvent::ToolApprovalNeeded {
                    call_id: data.toolcallid.clone(),
                    name: data.toolname.clone(),
                    input: serde_json::Value::Null,
                }]
            } else {
                vec![]
            }
        }
        AIStreamEvent::Finish { .. } => {
            vec![AdapterEvent::MessageEnd { usage: None }]
        }
        AIStreamEvent::Error { message } => {
            vec![AdapterEvent::Error {
                message: message.clone(),
            }]
        }
        // Events that don't produce adapter events (start-step, text-start, etc.)
        _ => vec![],
    }
}

// ---- Agent NDJSON event types (Claude Code protocol) ----

/// Top-level events from Claude Code's NDJSON stream-json protocol.
///
/// When invoked with `--output-format stream-json`, Claude Code emits 6 event types:
/// - `system` (init/compact_boundary): Session metadata
/// - `stream_event`: Token-level streaming (with `--include-partial-messages`)
/// - `assistant`: Complete assistant message
/// - `user`: Tool results
/// - `result`: Final cost/usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeCodeEvent {
    /// System event (init with session metadata, or compact_boundary).
    #[serde(rename = "system")]
    System {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        session_id: String,
        #[serde(default)]
        model: String,
        #[serde(default)]
        tools: Vec<String>,
        #[serde(default)]
        cwd: String,
    },

    /// Token-level streaming event (wraps inner Anthropic API stream event).
    /// Only emitted when `--include-partial-messages` is set.
    #[serde(rename = "stream_event")]
    StreamEvent {
        #[serde(default)]
        session_id: String,
        event: ClaudeCodeStreamEvent,
    },

    /// Complete assistant message (emitted after all stream events for a turn).
    #[serde(rename = "assistant")]
    Assistant {
        #[serde(default)]
        session_id: String,
        message: ClaudeCodeMessage,
    },

    /// User message containing tool results (emitted after tool execution).
    #[serde(rename = "user")]
    User {
        #[serde(default)]
        session_id: String,
        message: ClaudeCodeMessage,
    },

    /// Final result event with cost and usage statistics.
    #[serde(rename = "result")]
    Result {
        #[serde(default)]
        subtype: String,
        #[serde(default)]
        session_id: String,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        duration_ms: i64,
        #[serde(default)]
        num_turns: i32,
        #[serde(default)]
        total_cost_usd: f64,
        #[serde(default)]
        result: Option<serde_json::Value>,
        #[serde(default)]
        usage: Option<ClaudeCodeUsage>,
    },
}

/// Inner stream events from Claude Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeCodeStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart {
        message: ClaudeCodeMessage,
    },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ClaudeCodeContentBlock,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: ClaudeCodeDelta,
    },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        index: usize,
    },

    #[serde(rename = "message_delta")]
    MessageDelta {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delta: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<ClaudeCodeUsage>,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeMessage {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub content: Vec<ClaudeCodeContentBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<ClaudeCodeUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeCodeContentBlock {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: String,
    },

    #[serde(rename = "tool_use")]
    ToolUse {
        #[serde(default)]
        id: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(default)]
        tool_use_id: String,
        #[serde(default)]
        content: serde_json::Value,
        #[serde(default)]
        is_error: bool,
    },

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeCodeDelta {
    #[serde(rename = "text_delta")]
    TextDelta {
        #[serde(default)]
        text: String,
    },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        #[serde(default)]
        partial_json: String,
    },

    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeCodeUsage {
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cache_creation_input_tokens: i64,
    #[serde(default)]
    pub cache_read_input_tokens: i64,
}

// ---- Agent adapter: converts ClaudeCodeStreamEvent -> AdapterEvent ----

/// Converts a Claude Code stream event into adapter events.
pub fn adapt_claude_code_stream_event(event: &ClaudeCodeStreamEvent) -> Vec<AdapterEvent> {
    match event {
        ClaudeCodeStreamEvent::MessageStart { message } => {
            let mut events = vec![AdapterEvent::MessageStart {
                message_id: String::new(), // ID comes from outer wrapper
                model: if message.model.is_empty() {
                    None
                } else {
                    Some(message.model.clone())
                },
            }];

            // If message has initial content blocks, emit them
            for block in &message.content {
                match block {
                    ClaudeCodeContentBlock::Text { text } if !text.is_empty() => {
                        events.push(AdapterEvent::TextDelta { text: text.clone() });
                    }
                    _ => {}
                }
            }

            events
        }

        ClaudeCodeStreamEvent::ContentBlockStart { content_block, .. } => match content_block {
            ClaudeCodeContentBlock::ToolUse { id, name, .. } => {
                vec![AdapterEvent::ToolUseStart {
                    call_id: id.clone(),
                    name: name.clone(),
                    summary: String::new(),
                }]
            }
            ClaudeCodeContentBlock::Text { text } if !text.is_empty() => {
                vec![AdapterEvent::TextDelta { text: text.clone() }]
            }
            _ => vec![],
        },

        ClaudeCodeStreamEvent::ContentBlockDelta { delta, .. } => match delta {
            ClaudeCodeDelta::TextDelta { text } => {
                vec![AdapterEvent::TextDelta { text: text.clone() }]
            }
            // InputJsonDelta is accumulated by the parser, not emitted as adapter events.
            // The full input arrives via ToolUseInput when content_block_stop fires.
            _ => vec![],
        },

        ClaudeCodeStreamEvent::MessageDelta { usage, .. } => {
            if let Some(u) = usage {
                vec![AdapterEvent::MessageEnd {
                    usage: Some(super::unified::TokenUsage {
                        input_tokens: u.input_tokens,
                        output_tokens: u.output_tokens,
                        cache_read_tokens: u.cache_read_input_tokens,
                        cache_write_tokens: u.cache_creation_input_tokens,
                    }),
                }]
            } else {
                vec![]
            }
        }

        ClaudeCodeStreamEvent::MessageStop => vec![AdapterEvent::MessageEnd { usage: None }],

        _ => vec![],
    }
}

/// Convert a complete assistant message to adapter events.
///
/// Used when we receive the full `assistant` event (after streaming).
/// Extracts text, tool_use, and tool_result blocks from the message content.
pub fn adapt_claude_code_assistant_message(message: &ClaudeCodeMessage) -> Vec<AdapterEvent> {
    let mut events = Vec::new();

    // Emit message start with model info
    events.push(AdapterEvent::MessageStart {
        message_id: String::new(),
        model: if message.model.is_empty() {
            None
        } else {
            Some(message.model.clone())
        },
    });

    for block in &message.content {
        match block {
            ClaudeCodeContentBlock::Text { text } if !text.is_empty() => {
                events.push(AdapterEvent::TextDelta { text: text.clone() });
            }
            ClaudeCodeContentBlock::ToolUse { id, name, input } => {
                events.push(AdapterEvent::ToolUseStart {
                    call_id: id.clone(),
                    name: name.clone(),
                    summary: String::new(),
                });
                if !input.is_null() {
                    events.push(AdapterEvent::ToolUseInput {
                        call_id: id.clone(),
                        input: input.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    // Emit message end with usage if available
    events.push(AdapterEvent::MessageEnd {
        usage: message.usage.as_ref().map(|u| super::unified::TokenUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            cache_read_tokens: u.cache_read_input_tokens,
            cache_write_tokens: u.cache_creation_input_tokens,
        }),
    });

    events
}

/// Convert a user message (tool results) to adapter events.
///
/// Used when we receive the `user` event after Claude Code executes tools.
pub fn adapt_claude_code_user_message(message: &ClaudeCodeMessage) -> Vec<AdapterEvent> {
    let mut events = Vec::new();

    for block in &message.content {
        match block {
            ClaudeCodeContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => {
                let content_str = match content {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Array(arr) => {
                        // Tool results can be an array of content blocks
                        arr.iter()
                            .filter_map(|v| {
                                if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                                    Some(text.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                    other => serde_json::to_string(other).unwrap_or_default(),
                };
                events.push(AdapterEvent::ToolResult {
                    call_id: tool_use_id.clone(),
                    content: content_str,
                    is_error: *is_error,
                });
            }
            _ => {}
        }
    }

    events
}

/// Convert a top-level Claude Code event into adapter events.
///
/// This is the main entry point for the Claude Code adapter. It handles
/// all 6 event types from the stream-json protocol.
pub fn adapt_claude_code_event(event: &ClaudeCodeEvent) -> Vec<AdapterEvent> {
    match event {
        ClaudeCodeEvent::System {
            subtype,
            session_id,
            model,
            tools,
            cwd,
        } => {
            if subtype == "init" {
                vec![AdapterEvent::SessionStart {
                    session_id: session_id.clone(),
                    model: if model.is_empty() {
                        None
                    } else {
                        Some(model.clone())
                    },
                    tools: tools.clone(),
                    cwd: cwd.clone(),
                }]
            } else {
                // compact_boundary or other subtypes — no adapter events needed
                vec![]
            }
        }
        ClaudeCodeEvent::StreamEvent { event, .. } => adapt_claude_code_stream_event(event),
        ClaudeCodeEvent::Assistant { message, .. } => {
            adapt_claude_code_assistant_message(message)
        }
        ClaudeCodeEvent::User { message, .. } => adapt_claude_code_user_message(message),
        ClaudeCodeEvent::Result {
            total_cost_usd,
            usage,
            is_error,
            num_turns,
            duration_ms,
            ..
        } => {
            vec![AdapterEvent::SessionEnd {
                total_cost_usd: *total_cost_usd,
                usage: usage.as_ref().map(|u| super::unified::TokenUsage {
                    input_tokens: u.input_tokens,
                    output_tokens: u.output_tokens,
                    cache_read_tokens: u.cache_read_input_tokens,
                    cache_write_tokens: u.cache_creation_input_tokens,
                }),
                is_error: *is_error,
                num_turns: *num_turns,
                duration_ms: *duration_ms,
            }]
        }
    }
}

/// Apply an adapter event to a UnifiedMessage being built.
///
/// This is the core state machine that updates the message as events arrive.
pub fn apply_adapter_event(msg: &mut UnifiedMessage, event: &AdapterEvent) {
    match event {
        AdapterEvent::MessageStart { model, .. } => {
            if let Some(m) = model {
                msg.model = Some(m.clone());
            }
        }
        AdapterEvent::TextDelta { text } => {
            msg.append_text(text);
        }
        AdapterEvent::ReasoningDelta { text } => {
            msg.append_reasoning(text);
        }
        AdapterEvent::ToolUseStart {
            call_id,
            name,
            summary,
        } => {
            msg.parts.push(UnifiedMessagePart::ToolUse {
                call_id: call_id.clone(),
                name: name.clone(),
                input: serde_json::Value::Null,
                summary: summary.clone(),
                approval: String::new(),
            });
        }
        AdapterEvent::ToolUseInput { call_id, input } => {
            // Find the tool use part and update its input
            for part in &mut msg.parts {
                if let UnifiedMessagePart::ToolUse {
                    call_id: cid,
                    input: existing_input,
                    ..
                } = part
                {
                    if cid == call_id {
                        *existing_input = input.clone();
                        break;
                    }
                }
            }
        }
        AdapterEvent::ToolApprovalNeeded {
            call_id, ..
        } => {
            for part in &mut msg.parts {
                if let UnifiedMessagePart::ToolUse {
                    call_id: cid,
                    approval,
                    ..
                } = part
                {
                    if cid == call_id {
                        *approval = TOOL_APPROVAL_PENDING.to_string();
                        break;
                    }
                }
            }
        }
        AdapterEvent::ToolResult {
            call_id,
            content,
            is_error,
        } => {
            msg.add_tool_result(call_id.clone(), content.clone(), *is_error);
        }
        AdapterEvent::MessageEnd { usage } => {
            if let Some(u) = usage {
                msg.usage = Some(u.clone());
            }
            msg.set_complete();
        }
        AdapterEvent::Error { message } => {
            msg.parts.push(UnifiedMessagePart::Error {
                message: message.clone(),
            });
            msg.status = MSG_STATUS_ERROR.to_string();
        }
        // Session-level events don't modify individual messages
        AdapterEvent::SessionStart { .. } | AdapterEvent::SessionEnd { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Chat adapter tests ----

    #[test]
    fn test_adapt_chat_start() {
        let event = AIStreamEvent::Start {
            messageid: "msg-1".into(),
        };
        let adapted = adapt_chat_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::MessageStart { message_id, .. } => {
                assert_eq!(message_id, "msg-1");
            }
            _ => panic!("Expected MessageStart"),
        }
    }

    #[test]
    fn test_adapt_chat_text_delta() {
        let event = AIStreamEvent::TextDelta {
            id: "t1".into(),
            delta: "hello".into(),
        };
        let adapted = adapt_chat_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::TextDelta { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected TextDelta"),
        }
    }

    #[test]
    fn test_adapt_chat_reasoning() {
        let event = AIStreamEvent::ReasoningDelta {
            id: "r1".into(),
            delta: "thinking...".into(),
        };
        let adapted = adapt_chat_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::ReasoningDelta { text } => assert_eq!(text, "thinking..."),
            _ => panic!("Expected ReasoningDelta"),
        }
    }

    #[test]
    fn test_adapt_chat_tool_available() {
        let event = AIStreamEvent::ToolInputAvailable {
            callid: "c1".into(),
            toolname: "read_file".into(),
            input: serde_json::json!({"path": "/tmp/test"}),
        };
        let adapted = adapt_chat_stream_event(&event);
        assert_eq!(adapted.len(), 2);
        match &adapted[0] {
            AdapterEvent::ToolUseStart { call_id, name, .. } => {
                assert_eq!(call_id, "c1");
                assert_eq!(name, "read_file");
            }
            _ => panic!("Expected ToolUseStart"),
        }
        match &adapted[1] {
            AdapterEvent::ToolUseInput { call_id, input } => {
                assert_eq!(call_id, "c1");
                assert_eq!(input["path"], "/tmp/test");
            }
            _ => panic!("Expected ToolUseInput"),
        }
    }

    #[test]
    fn test_adapt_chat_finish() {
        let event = AIStreamEvent::Finish {
            reason: "done".into(),
            metadata: None,
        };
        let adapted = adapt_chat_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        assert!(matches!(adapted[0], AdapterEvent::MessageEnd { .. }));
    }

    #[test]
    fn test_adapt_chat_error() {
        let event = AIStreamEvent::Error {
            message: "rate limit".into(),
        };
        let adapted = adapt_chat_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::Error { message } => assert_eq!(message, "rate limit"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_adapt_chat_noop_events() {
        let events = vec![
            AIStreamEvent::StartStep,
            AIStreamEvent::FinishStep,
            AIStreamEvent::TextStart { id: "t1".into() },
            AIStreamEvent::TextEnd { id: "t1".into() },
            AIStreamEvent::ReasoningStart { id: "r1".into() },
            AIStreamEvent::ReasoningEnd { id: "r1".into() },
        ];
        for event in events {
            let adapted = adapt_chat_stream_event(&event);
            assert!(adapted.is_empty(), "Expected no events for {:?}", event);
        }
    }

    // ---- Claude Code adapter tests ----

    #[test]
    fn test_adapt_cc_message_start() {
        let event = ClaudeCodeStreamEvent::MessageStart {
            message: ClaudeCodeMessage {
                role: "assistant".into(),
                model: "claude-sonnet-4-5".into(),
                content: vec![],
                usage: None,
            },
        };
        let adapted = adapt_claude_code_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::MessageStart { model, .. } => {
                assert_eq!(model.as_deref(), Some("claude-sonnet-4-5"));
            }
            _ => panic!("Expected MessageStart"),
        }
    }

    #[test]
    fn test_adapt_cc_text_delta() {
        let event = ClaudeCodeStreamEvent::ContentBlockDelta {
            index: 0,
            delta: ClaudeCodeDelta::TextDelta {
                text: "Hello".into(),
            },
        };
        let adapted = adapt_claude_code_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::TextDelta { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected TextDelta"),
        }
    }

    #[test]
    fn test_adapt_cc_tool_start() {
        let event = ClaudeCodeStreamEvent::ContentBlockStart {
            index: 1,
            content_block: ClaudeCodeContentBlock::ToolUse {
                id: "tool-1".into(),
                name: "Bash".into(),
                input: serde_json::Value::Null,
            },
        };
        let adapted = adapt_claude_code_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::ToolUseStart { call_id, name, .. } => {
                assert_eq!(call_id, "tool-1");
                assert_eq!(name, "Bash");
            }
            _ => panic!("Expected ToolUseStart"),
        }
    }

    #[test]
    fn test_adapt_cc_message_delta_usage() {
        let event = ClaudeCodeStreamEvent::MessageDelta {
            delta: None,
            usage: Some(ClaudeCodeUsage {
                input_tokens: 500,
                output_tokens: 200,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 100,
            }),
        };
        let adapted = adapt_claude_code_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        match &adapted[0] {
            AdapterEvent::MessageEnd { usage } => {
                let u = usage.as_ref().unwrap();
                assert_eq!(u.input_tokens, 500);
                assert_eq!(u.output_tokens, 200);
                assert_eq!(u.cache_read_tokens, 100);
            }
            _ => panic!("Expected MessageEnd"),
        }
    }

    #[test]
    fn test_adapt_cc_message_stop() {
        let event = ClaudeCodeStreamEvent::MessageStop;
        let adapted = adapt_claude_code_stream_event(&event);
        assert_eq!(adapted.len(), 1);
        assert!(matches!(adapted[0], AdapterEvent::MessageEnd { .. }));
    }

    // ---- apply_adapter_event tests ----

    #[test]
    fn test_apply_text_delta() {
        let mut msg = UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_CHAT, "openai");

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::TextDelta {
                text: "Hello ".into(),
            },
        );
        apply_adapter_event(
            &mut msg,
            &AdapterEvent::TextDelta {
                text: "world".into(),
            },
        );

        assert_eq!(msg.full_text(), "Hello world");
        assert_eq!(msg.parts.len(), 1); // Merged
    }

    #[test]
    fn test_apply_reasoning_delta() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_CHAT, "anthropic");

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ReasoningDelta {
                text: "Hmm ".into(),
            },
        );
        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ReasoningDelta {
                text: "let me think".into(),
            },
        );
        apply_adapter_event(
            &mut msg,
            &AdapterEvent::TextDelta {
                text: "Answer here".into(),
            },
        );

        assert_eq!(msg.parts.len(), 2);
        match &msg.parts[0] {
            UnifiedMessagePart::Reasoning { text } => assert_eq!(text, "Hmm let me think"),
            _ => panic!("Expected Reasoning"),
        }
    }

    #[test]
    fn test_apply_tool_lifecycle() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_AGENT, "claudecode");

        // Tool start
        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ToolUseStart {
                call_id: "c1".into(),
                name: "read_file".into(),
                summary: "Read /tmp/test".into(),
            },
        );

        // Tool input
        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ToolUseInput {
                call_id: "c1".into(),
                input: serde_json::json!({"path": "/tmp/test"}),
            },
        );

        // Tool result
        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ToolResult {
                call_id: "c1".into(),
                content: "file contents".into(),
                is_error: false,
            },
        );

        assert_eq!(msg.parts.len(), 2); // tool_use + tool_result

        match &msg.parts[0] {
            UnifiedMessagePart::ToolUse {
                call_id,
                name,
                input,
                summary,
                ..
            } => {
                assert_eq!(call_id, "c1");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "/tmp/test");
                assert_eq!(summary, "Read /tmp/test");
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_apply_tool_approval() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_AGENT, "claudecode");

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ToolUseStart {
                call_id: "c1".into(),
                name: "Bash".into(),
                summary: String::new(),
            },
        );

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::ToolApprovalNeeded {
                call_id: "c1".into(),
                name: "Bash".into(),
                input: serde_json::Value::Null,
            },
        );

        match &msg.parts[0] {
            UnifiedMessagePart::ToolUse { approval, .. } => {
                assert_eq!(approval, TOOL_APPROVAL_PENDING);
            }
            _ => panic!("Expected ToolUse"),
        }
    }

    #[test]
    fn test_apply_message_end() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_CHAT, "openai");
        msg.append_text("response");

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::MessageEnd {
                usage: Some(super::super::unified::TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                    ..Default::default()
                }),
            },
        );

        assert_eq!(msg.status, MSG_STATUS_COMPLETE);
        assert_eq!(msg.usage.as_ref().unwrap().input_tokens, 100);
    }

    #[test]
    fn test_apply_error() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_CHAT, "openai");

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::Error {
                message: "API error".into(),
            },
        );

        assert_eq!(msg.status, MSG_STATUS_ERROR);
        assert_eq!(msg.parts.len(), 1);
        match &msg.parts[0] {
            UnifiedMessagePart::Error { message } => assert_eq!(message, "API error"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_apply_message_start_model() {
        let mut msg =
            UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_AGENT, "claudecode");
        assert!(msg.model.is_none());

        apply_adapter_event(
            &mut msg,
            &AdapterEvent::MessageStart {
                message_id: "m1".into(),
                model: Some("claude-sonnet-4-5".into()),
            },
        );

        assert_eq!(msg.model.as_deref(), Some("claude-sonnet-4-5"));
    }

    #[test]
    fn test_full_chat_flow() {
        let mut msg = UnifiedMessage::assistant_streaming("m1".into(), BACKEND_TYPE_CHAT, "openai");

        // Simulate a chat response with tool use
        let events = vec![
            AdapterEvent::MessageStart {
                message_id: "m1".into(),
                model: Some("gpt-5".into()),
            },
            AdapterEvent::TextDelta {
                text: "Let me check ".into(),
            },
            AdapterEvent::TextDelta {
                text: "that file.".into(),
            },
            AdapterEvent::ToolUseStart {
                call_id: "c1".into(),
                name: "read_file".into(),
                summary: String::new(),
            },
            AdapterEvent::ToolUseInput {
                call_id: "c1".into(),
                input: serde_json::json!({"path": "/tmp/test"}),
            },
            AdapterEvent::ToolResult {
                call_id: "c1".into(),
                content: "hello world".into(),
                is_error: false,
            },
            AdapterEvent::TextDelta {
                text: "The file contains: hello world".into(),
            },
            AdapterEvent::MessageEnd {
                usage: Some(super::super::unified::TokenUsage {
                    input_tokens: 200,
                    output_tokens: 100,
                    ..Default::default()
                }),
            },
        ];

        for event in &events {
            apply_adapter_event(&mut msg, event);
        }

        assert_eq!(msg.model.as_deref(), Some("gpt-5"));
        assert_eq!(msg.status, MSG_STATUS_COMPLETE);
        assert_eq!(msg.parts.len(), 4); // text, tool_use, tool_result, text
        assert!(msg.has_tool_use());
        assert_eq!(msg.usage.as_ref().unwrap().input_tokens, 200);
    }

    // ---- AdapterEvent serde ----

    #[test]
    fn test_adapter_event_serde() {
        let events = vec![
            AdapterEvent::MessageStart {
                message_id: "m1".into(),
                model: Some("gpt-5".into()),
            },
            AdapterEvent::TextDelta {
                text: "hello".into(),
            },
            AdapterEvent::ReasoningDelta {
                text: "hmm".into(),
            },
            AdapterEvent::ToolUseStart {
                call_id: "c1".into(),
                name: "Bash".into(),
                summary: "Run ls".into(),
            },
            AdapterEvent::ToolResult {
                call_id: "c1".into(),
                content: "files".into(),
                is_error: false,
            },
            AdapterEvent::MessageEnd { usage: None },
            AdapterEvent::Error {
                message: "oops".into(),
            },
        ];

        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let parsed: AdapterEvent = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&parsed).unwrap();
            assert_eq!(json, json2);
        }
    }

    // ---- Claude Code event serde ----

    #[test]
    fn test_claude_code_content_block_serde() {
        let blocks: Vec<ClaudeCodeContentBlock> = vec![
            ClaudeCodeContentBlock::Text {
                text: "hello".into(),
            },
            ClaudeCodeContentBlock::ToolUse {
                id: "t1".into(),
                name: "Bash".into(),
                input: serde_json::json!({"command": "ls"}),
            },
        ];

        let json = serde_json::to_string(&blocks).unwrap();
        let parsed: Vec<ClaudeCodeContentBlock> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_claude_code_delta_serde() {
        let delta = ClaudeCodeDelta::TextDelta {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains("\"type\":\"text_delta\""));

        let parsed: ClaudeCodeDelta = serde_json::from_str(&json).unwrap();
        match parsed {
            ClaudeCodeDelta::TextDelta { text } => assert_eq!(text, "hello"),
            _ => panic!("Expected TextDelta"),
        }
    }
}
