use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use std::{fmt, io};
use uuid::Uuid;

const ADAPTER_VERSION: &str = "v3-protocol-runtime-1";
const DEFAULT_BACKPRESSURE_LIMIT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolKind {
    OpenaiResponses,
    OpenaiChatCompletions,
    AnthropicMessages,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolRoute {
    Direct,
    Adapter,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolCompatibility {
    Supported,
    Partial,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProtocolCapabilities {
    pub tools: bool,
    pub images: bool,
    pub streaming: bool,
    pub reasoning: bool,
    pub usage: bool,
}

impl Default for ModelProtocolCapabilities {
    fn default() -> Self {
        Self {
            tools: true,
            images: true,
            streaming: true,
            reasoning: true,
            usage: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolResolution {
    pub native_protocol: ProtocolKind,
    pub upstream_protocol: ProtocolKind,
    pub route: ProtocolRoute,
    pub compatibility: ProtocolCompatibility,
    pub adapter_id: Option<String>,
    pub adapter_version: Option<String>,
    pub limitations: Vec<String>,
    pub diagnostics: Vec<String>,
}

pub fn resolve_protocol(
    native_protocol: ProtocolKind,
    upstream_protocol: ProtocolKind,
    capabilities: &ModelProtocolCapabilities,
) -> ProtocolResolution {
    if matches!(native_protocol, ProtocolKind::Unknown)
        || matches!(upstream_protocol, ProtocolKind::Unknown)
    {
        return ProtocolResolution {
            native_protocol,
            upstream_protocol,
            route: ProtocolRoute::Unavailable,
            compatibility: ProtocolCompatibility::Unknown,
            adapter_id: None,
            adapter_version: None,
            limitations: vec!["unknown protocol cannot be selected implicitly".to_owned()],
            diagnostics: vec!["protocol.capability.unknown".to_owned()],
        };
    }

    let same_protocol = native_protocol == upstream_protocol;
    let adapter_id = if same_protocol {
        None
    } else {
        Some(adapter_id(native_protocol, upstream_protocol))
    };
    let mut limitations = Vec::new();
    if !capabilities.tools {
        limitations.push("model does not advertise tool calling".to_owned());
    }
    if !capabilities.images {
        limitations.push("model does not advertise image input".to_owned());
    }
    if !capabilities.streaming {
        limitations.push("model does not advertise streaming".to_owned());
    }
    if !capabilities.reasoning {
        limitations.push("model does not advertise reasoning controls".to_owned());
    }
    if !capabilities.usage {
        limitations.push("model does not advertise usage reporting".to_owned());
    }

    ProtocolResolution {
        native_protocol,
        upstream_protocol,
        route: if same_protocol {
            ProtocolRoute::Direct
        } else {
            ProtocolRoute::Adapter
        },
        compatibility: if limitations.is_empty() {
            ProtocolCompatibility::Supported
        } else {
            ProtocolCompatibility::Partial
        },
        adapter_id,
        adapter_version: (!same_protocol).then(|| ADAPTER_VERSION.to_owned()),
        limitations,
        diagnostics: vec![if same_protocol {
            "protocol.route.direct".to_owned()
        } else {
            "protocol.route.adapter".to_owned()
        }],
    }
}

fn adapter_id(native: ProtocolKind, upstream: ProtocolKind) -> String {
    format!("{}-to-{}", protocol_name(upstream), protocol_name(native))
}

fn protocol_name(protocol: ProtocolKind) -> &'static str {
    match protocol {
        ProtocolKind::OpenaiResponses => "openai-responses",
        ProtocolKind::OpenaiChatCompletions => "openai-chat-completions",
        ProtocolKind::AnthropicMessages => "anthropic-messages",
        ProtocolKind::Unknown => "unknown",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UnifiedContent {
    Text {
        text: String,
    },
    ImageUrl {
        url: String,
        media_type: Option<String>,
    },
    ImageBase64 {
        data: String,
        media_type: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<UnifiedContent>,
        is_error: bool,
    },
    Reasoning {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedMessage {
    pub role: UnifiedRole,
    pub content: Vec<UnifiedContent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedRequest {
    pub model: Option<String>,
    pub messages: Vec<UnifiedMessage>,
    pub tools: Vec<UnifiedTool>,
    pub reasoning_effort: Option<String>,
    pub stream: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedResponse {
    pub id: Option<String>,
    pub content: Vec<UnifiedContent>,
    pub finish_reason: Option<String>,
    pub usage: Option<UnifiedUsage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UnifiedStreamEvent {
    MessageStarted {
        id: Option<String>,
    },
    TextDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCallDelta {
        id: Option<String>,
        name: Option<String>,
        arguments_delta: String,
    },
    Usage {
        usage: UnifiedUsage,
    },
    Completed {
        finish_reason: Option<String>,
    },
    Error {
        error: ProtocolRuntimeError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolRuntimeError {
    pub code: String,
    pub message: String,
    pub protocol: ProtocolKind,
    pub retryable: bool,
    pub status: Option<u16>,
}

impl ProtocolRuntimeError {
    fn invalid(protocol: ProtocolKind, message: impl Into<String>) -> Self {
        Self {
            code: "PROTOCOL_PAYLOAD_INVALID".to_owned(),
            message: message.into(),
            protocol,
            retryable: false,
            status: None,
        }
    }

    fn unsupported(protocol: ProtocolKind, message: impl Into<String>) -> Self {
        Self {
            code: "PROTOCOL_CAPABILITY_UNSUPPORTED".to_owned(),
            message: message.into(),
            protocol,
            retryable: false,
            status: None,
        }
    }

    fn transport(protocol: ProtocolKind, code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
            protocol,
            retryable: true,
            status: None,
        }
    }
}

impl fmt::Display for ProtocolRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolRuntimeError {}

pub fn openai_chat_to_unified(input: &Value) -> Result<UnifiedRequest, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::OpenaiChatCompletions)?;
    let messages = array(
        root.get("messages"),
        ProtocolKind::OpenaiChatCompletions,
        "messages",
    )?
    .iter()
    .map(parse_openai_chat_message)
    .collect::<Result<Vec<_>, _>>()?;
    let tools = root
        .get("tools")
        .and_then(Value::as_array)
        .map_or(&[] as &[Value], |values| values.as_slice())
        .iter()
        .map(parse_openai_tool)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UnifiedRequest {
        model: string(root.get("model")),
        messages,
        tools,
        reasoning_effort: string(root.get("reasoning_effort")),
        stream: root.get("stream").and_then(Value::as_bool).unwrap_or(false),
    })
}

pub fn unified_to_openai_chat(request: &UnifiedRequest) -> Result<Value, ProtocolRuntimeError> {
    let messages = request
        .messages
        .iter()
        .map(unified_message_to_openai_chat)
        .collect::<Result<Vec<_>, _>>()?;
    let mut root = Map::new();
    if let Some(model) = &request.model {
        root.insert("model".to_owned(), Value::String(model.clone()));
    }
    root.insert("messages".to_owned(), Value::Array(messages));
    if !request.tools.is_empty() {
        root.insert(
            "tools".to_owned(),
            Value::Array(request.tools.iter().map(openai_tool).collect()),
        );
    }
    if let Some(effort) = &request.reasoning_effort {
        root.insert("reasoning_effort".to_owned(), Value::String(effort.clone()));
    }
    if request.stream {
        root.insert("stream".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(root))
}

pub fn anthropic_messages_to_unified(
    input: &Value,
) -> Result<UnifiedRequest, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::AnthropicMessages)?;
    let mut messages = Vec::new();
    if let Some(system) = root.get("system") {
        messages.push(UnifiedMessage {
            role: UnifiedRole::System,
            content: parse_anthropic_content(system)?,
        });
    }
    let raw_messages = array(
        root.get("messages"),
        ProtocolKind::AnthropicMessages,
        "messages",
    )?;
    for value in raw_messages {
        let object = object(value, ProtocolKind::AnthropicMessages)?;
        let role =
            match required_string(object.get("role"), ProtocolKind::AnthropicMessages, "role")? {
                "user" => UnifiedRole::User,
                "assistant" => UnifiedRole::Assistant,
                other => {
                    return Err(ProtocolRuntimeError::invalid(
                        ProtocolKind::AnthropicMessages,
                        format!("unsupported Anthropic role {other}"),
                    ))
                }
            };
        messages.push(UnifiedMessage {
            role,
            content: parse_anthropic_content(required(
                object.get("content"),
                "content",
                ProtocolKind::AnthropicMessages,
            )?)?,
        });
    }
    let tools = root
        .get("tools")
        .and_then(Value::as_array)
        .map_or(&[] as &[Value], |values| values.as_slice())
        .iter()
        .map(parse_anthropic_tool)
        .collect::<Result<Vec<_>, _>>()?;
    let reasoning_effort = root
        .get("thinking")
        .and_then(Value::as_object)
        .and_then(|thinking| thinking.get("budget_tokens"))
        .and_then(Value::as_u64)
        .map(|budget| format!("budget:{budget}"));
    Ok(UnifiedRequest {
        model: string(root.get("model")),
        messages,
        tools,
        reasoning_effort,
        stream: root.get("stream").and_then(Value::as_bool).unwrap_or(false),
    })
}

pub fn unified_to_anthropic_messages(
    request: &UnifiedRequest,
) -> Result<Value, ProtocolRuntimeError> {
    let mut system = Vec::new();
    let mut messages = Vec::new();
    for message in &request.messages {
        if message.role == UnifiedRole::System || message.role == UnifiedRole::Developer {
            system.extend(message.content.iter().filter_map(content_text));
            continue;
        }
        let role = match message.role {
            UnifiedRole::Tool | UnifiedRole::User => "user",
            UnifiedRole::Assistant => "assistant",
            UnifiedRole::System | UnifiedRole::Developer => unreachable!(),
        };
        messages.push(json!({
            "role": role,
            "content": message.content.iter().map(anthropic_content).collect::<Result<Vec<_>, _>>()?
        }));
    }
    let mut root = Map::new();
    if let Some(model) = &request.model {
        root.insert("model".to_owned(), Value::String(model.clone()));
    }
    if !system.is_empty() {
        root.insert("system".to_owned(), Value::String(system.join("\n")));
    }
    root.insert("messages".to_owned(), Value::Array(messages));
    if !request.tools.is_empty() {
        root.insert(
            "tools".to_owned(),
            Value::Array(request.tools.iter().map(anthropic_tool).collect()),
        );
    }
    if let Some(effort) = &request.reasoning_effort {
        if let Some(tokens) = effort
            .strip_prefix("budget:")
            .and_then(|value| value.parse::<u64>().ok())
        {
            root.insert(
                "thinking".to_owned(),
                json!({"type":"enabled", "budget_tokens":tokens}),
            );
        } else {
            root.insert(
                "metadata".to_owned(),
                json!({"vibehub_reasoning_effort":effort}),
            );
        }
    }
    if request.stream {
        root.insert("stream".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(root))
}

pub fn openai_responses_to_unified(input: &Value) -> Result<UnifiedRequest, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::OpenaiResponses)?;
    let mut messages = Vec::new();
    if let Some(instructions) = root.get("instructions").and_then(Value::as_str) {
        messages.push(UnifiedMessage {
            role: UnifiedRole::System,
            content: vec![UnifiedContent::Text {
                text: instructions.to_owned(),
            }],
        });
    }
    match root.get("input") {
        Some(Value::String(text)) => messages.push(UnifiedMessage {
            role: UnifiedRole::User,
            content: vec![UnifiedContent::Text { text: text.clone() }],
        }),
        Some(Value::Array(items)) => {
            for item in items {
                parse_openai_response_input_item(item, &mut messages)?;
            }
        }
        None => {}
        Some(_) => {
            return Err(ProtocolRuntimeError::invalid(
                ProtocolKind::OpenaiResponses,
                "input must be a string or array",
            ))
        }
    }
    let tools = root
        .get("tools")
        .and_then(Value::as_array)
        .map_or(&[] as &[Value], |values| values.as_slice())
        .iter()
        .map(parse_openai_responses_tool)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UnifiedRequest {
        model: string(root.get("model")),
        messages,
        tools,
        reasoning_effort: root
            .get("reasoning")
            .and_then(Value::as_object)
            .and_then(|reasoning| reasoning.get("effort"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        stream: root.get("stream").and_then(Value::as_bool).unwrap_or(false),
    })
}

pub fn unified_to_openai_responses(
    request: &UnifiedRequest,
) -> Result<Value, ProtocolRuntimeError> {
    let mut root = Map::new();
    if let Some(model) = &request.model {
        root.insert("model".to_owned(), Value::String(model.clone()));
    }
    let mut instructions = Vec::new();
    let mut input = Vec::new();
    for message in &request.messages {
        if matches!(message.role, UnifiedRole::System | UnifiedRole::Developer) {
            instructions.extend(message.content.iter().filter_map(content_text));
        } else {
            for content in &message.content {
                input.push(openai_response_input_item(message.role, content)?);
            }
        }
    }
    if !instructions.is_empty() {
        root.insert(
            "instructions".to_owned(),
            Value::String(instructions.join("\n")),
        );
    }
    root.insert("input".to_owned(), Value::Array(input));
    if !request.tools.is_empty() {
        root.insert(
            "tools".to_owned(),
            Value::Array(request.tools.iter().map(openai_responses_tool).collect()),
        );
    }
    if let Some(effort) = &request.reasoning_effort {
        root.insert("reasoning".to_owned(), json!({"effort": effort}));
    }
    if request.stream {
        root.insert("stream".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(root))
}

pub fn openai_chat_response_to_unified(
    input: &Value,
) -> Result<UnifiedResponse, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::OpenaiChatCompletions)?;
    let choice = root
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or_else(|| {
            ProtocolRuntimeError::invalid(ProtocolKind::OpenaiChatCompletions, "choices is missing")
        })?;
    let message = object(
        required(
            choice.get("message"),
            "message",
            ProtocolKind::OpenaiChatCompletions,
        )?,
        ProtocolKind::OpenaiChatCompletions,
    )?;
    let mut content = Vec::new();
    if let Some(text) = message.get("content").and_then(Value::as_str) {
        content.push(UnifiedContent::Text {
            text: text.to_owned(),
        });
    }
    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            let call = object(call, ProtocolKind::OpenaiChatCompletions)?;
            let function = object(
                required(
                    call.get("function"),
                    "function",
                    ProtocolKind::OpenaiChatCompletions,
                )?,
                ProtocolKind::OpenaiChatCompletions,
            )?;
            let arguments = required_string(
                function.get("arguments"),
                ProtocolKind::OpenaiChatCompletions,
                "arguments",
            )?;
            let input = serde_json::from_str(arguments).map_err(|error| {
                ProtocolRuntimeError::invalid(
                    ProtocolKind::OpenaiChatCompletions,
                    format!("tool arguments are not JSON: {error}"),
                )
            })?;
            content.push(UnifiedContent::ToolUse {
                id: string(call.get("id")).unwrap_or_else(|| "tool-call".to_owned()),
                name: required_string(
                    function.get("name"),
                    ProtocolKind::OpenaiChatCompletions,
                    "name",
                )?
                .to_owned(),
                input,
            });
        }
    }
    Ok(UnifiedResponse {
        id: string(root.get("id")),
        content,
        finish_reason: choice
            .get("finish_reason")
            .and_then(Value::as_str)
            .map(str::to_owned),
        usage: root.get("usage").and_then(parse_openai_usage),
    })
}

pub fn anthropic_response_to_unified(
    input: &Value,
) -> Result<UnifiedResponse, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::AnthropicMessages)?;
    let content = parse_anthropic_content(required(
        root.get("content"),
        "content",
        ProtocolKind::AnthropicMessages,
    )?)?;
    Ok(UnifiedResponse {
        id: string(root.get("id")),
        content,
        finish_reason: string(root.get("stop_reason")),
        usage: root.get("usage").and_then(parse_anthropic_usage),
    })
}

pub fn openai_chat_stream_event_to_unified(
    input: &Value,
) -> Result<Vec<UnifiedStreamEvent>, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::OpenaiChatCompletions)?;
    let mut events = Vec::new();
    if let Some(choices) = root.get("choices").and_then(Value::as_array) {
        if let Some(choice) = choices.first() {
            let delta = choice.get("delta").and_then(Value::as_object);
            if let Some(text) = delta
                .and_then(|delta| delta.get("content"))
                .and_then(Value::as_str)
            {
                events.push(UnifiedStreamEvent::TextDelta {
                    text: text.to_owned(),
                });
            }
            if let Some(reasoning) = delta
                .and_then(|delta| delta.get("reasoning_content"))
                .and_then(Value::as_str)
            {
                events.push(UnifiedStreamEvent::ReasoningDelta {
                    text: reasoning.to_owned(),
                });
            }
            if let Some(calls) = delta
                .and_then(|delta| delta.get("tool_calls"))
                .and_then(Value::as_array)
            {
                for call in calls {
                    let function = call.get("function").and_then(Value::as_object);
                    events.push(UnifiedStreamEvent::ToolCallDelta {
                        id: string(call.get("id")),
                        name: function.and_then(|function| string(function.get("name"))),
                        arguments_delta: function
                            .and_then(|function| string(function.get("arguments")))
                            .unwrap_or_default(),
                    });
                }
            }
            if choice
                .get("finish_reason")
                .is_some_and(|reason| !reason.is_null())
            {
                events.push(UnifiedStreamEvent::Completed {
                    finish_reason: string(choice.get("finish_reason")),
                });
            }
        }
    }
    if let Some(usage) = root.get("usage").and_then(parse_openai_usage) {
        events.push(UnifiedStreamEvent::Usage { usage });
    }
    Ok(events)
}

pub fn anthropic_stream_event_to_unified(
    input: &Value,
) -> Result<Vec<UnifiedStreamEvent>, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::AnthropicMessages)?;
    let event_type = required_string(root.get("type"), ProtocolKind::AnthropicMessages, "type")?;
    Ok(match event_type {
        "message_start" => vec![UnifiedStreamEvent::MessageStarted {
            id: root
                .get("message")
                .and_then(Value::as_object)
                .and_then(|message| string(message.get("id"))),
        }],
        "content_block_delta" => {
            let delta = object(
                required(root.get("delta"), "delta", ProtocolKind::AnthropicMessages)?,
                ProtocolKind::AnthropicMessages,
            )?;
            match string(delta.get("type")).as_deref() {
                Some("text_delta") => vec![UnifiedStreamEvent::TextDelta {
                    text: string(delta.get("text")).unwrap_or_default(),
                }],
                Some("thinking_delta") => vec![UnifiedStreamEvent::ReasoningDelta {
                    text: string(delta.get("thinking")).unwrap_or_default(),
                }],
                Some("input_json_delta") => vec![UnifiedStreamEvent::ToolCallDelta {
                    id: None,
                    name: None,
                    arguments_delta: string(delta.get("partial_json")).unwrap_or_default(),
                }],
                _ => {
                    return Err(ProtocolRuntimeError::unsupported(
                        ProtocolKind::AnthropicMessages,
                        "unknown content_block_delta",
                    ))
                }
            }
        }
        "content_block_start" => {
            let block = root
                .get("content_block")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    ProtocolRuntimeError::invalid(
                        ProtocolKind::AnthropicMessages,
                        "content_block_start is missing content_block",
                    )
                })?;
            if string(block.get("type")).as_deref() == Some("tool_use") {
                vec![UnifiedStreamEvent::ToolCallDelta {
                    id: string(block.get("id")),
                    name: string(block.get("name")),
                    arguments_delta: String::new(),
                }]
            } else {
                Vec::new()
            }
        }
        "content_block_stop" => Vec::new(),
        "message_delta" => {
            let delta = root.get("delta").and_then(Value::as_object);
            let mut events = Vec::new();
            if let Some(reason) = delta.and_then(|delta| string(delta.get("stop_reason"))) {
                events.push(UnifiedStreamEvent::Completed {
                    finish_reason: Some(reason),
                });
            }
            if let Some(usage) = root.get("usage").and_then(parse_anthropic_usage) {
                events.push(UnifiedStreamEvent::Usage { usage });
            }
            events
        }
        "message_stop" => vec![UnifiedStreamEvent::Completed {
            finish_reason: None,
        }],
        "error" => vec![UnifiedStreamEvent::Error {
            error: ProtocolRuntimeError::transport(
                ProtocolKind::AnthropicMessages,
                "UPSTREAM_ERROR",
                "Anthropic stream returned an error",
            ),
        }],
        _ => {
            return Err(ProtocolRuntimeError::unsupported(
                ProtocolKind::AnthropicMessages,
                format!("unsupported stream event {event_type}"),
            ))
        }
    })
}

pub fn openai_responses_stream_event_to_unified(
    input: &Value,
) -> Result<Vec<UnifiedStreamEvent>, ProtocolRuntimeError> {
    let root = object(input, ProtocolKind::OpenaiResponses)?;
    let event_type = required_string(root.get("type"), ProtocolKind::OpenaiResponses, "type")?;
    Ok(match event_type {
        "response.created" | "response.in_progress" => vec![UnifiedStreamEvent::MessageStarted {
            id: root
                .get("response")
                .and_then(Value::as_object)
                .and_then(|response| string(response.get("id"))),
        }],
        "response.output_text.delta" => vec![UnifiedStreamEvent::TextDelta {
            text: string(root.get("delta")).unwrap_or_default(),
        }],
        "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
            vec![UnifiedStreamEvent::ReasoningDelta {
                text: string(root.get("delta")).unwrap_or_default(),
            }]
        }
        "response.function_call_arguments.delta" => vec![UnifiedStreamEvent::ToolCallDelta {
            id: string(root.get("call_id")),
            name: string(root.get("name")),
            arguments_delta: string(root.get("delta")).unwrap_or_default(),
        }],
        "response.completed" => {
            let response = root.get("response").and_then(Value::as_object);
            let mut events = vec![UnifiedStreamEvent::Completed {
                finish_reason: response.and_then(|response| string(response.get("status"))),
            }];
            if let Some(usage) = response
                .and_then(|response| response.get("usage"))
                .and_then(parse_openai_usage)
            {
                events.push(UnifiedStreamEvent::Usage { usage });
            }
            events
        }
        "error" => vec![UnifiedStreamEvent::Error {
            error: parse_upstream_error(ProtocolKind::OpenaiResponses, input, None),
        }],
        other => {
            return Err(ProtocolRuntimeError::unsupported(
                ProtocolKind::OpenaiResponses,
                format!("unsupported Responses stream event {other}"),
            ))
        }
    })
}

pub fn parse_upstream_error(
    protocol: ProtocolKind,
    payload: &Value,
    status: Option<u16>,
) -> ProtocolRuntimeError {
    let error = payload.get("error").and_then(Value::as_object);
    let code = error
        .and_then(|error| string(error.get("code")).or_else(|| string(error.get("type"))))
        .unwrap_or_else(|| "UPSTREAM_ERROR".to_owned());
    let message = error
        .and_then(|error| string(error.get("message")))
        .unwrap_or_else(|| "upstream protocol returned an error".to_owned());
    ProtocolRuntimeError {
        code,
        message,
        protocol,
        retryable: status
            .is_some_and(|status| status == 408 || status == 409 || status == 429 || status >= 500),
        status,
    }
}

pub fn enforce_backpressure(
    buffered_events: usize,
    limit: Option<usize>,
) -> Result<(), ProtocolRuntimeError> {
    let limit = limit.unwrap_or(DEFAULT_BACKPRESSURE_LIMIT);
    if buffered_events > limit {
        Err(ProtocolRuntimeError::transport(
            ProtocolKind::Unknown,
            "PROTOCOL_BACKPRESSURE",
            format!("buffered events {buffered_events} exceed limit {limit}"),
        ))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RequestControl {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    deadline: Option<Instant>,
}

impl RequestControl {
    pub fn new(timeout: Option<Duration>) -> Self {
        Self {
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            deadline: timeout.map(|timeout| Instant::now() + timeout),
        }
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn check(&self, protocol: ProtocolKind) -> Result<(), ProtocolRuntimeError> {
        if self.cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ProtocolRuntimeError::transport(
                protocol,
                "REQUEST_CANCELLED",
                "request was cancelled",
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ProtocolRuntimeError::transport(
                protocol,
                "REQUEST_TIMEOUT",
                "request deadline exceeded",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarLaunchPlan {
    pub adapter_id: String,
    pub adapter_version: String,
    pub command: PathBuf,
    pub arguments: Vec<String>,
    pub port: u16,
    pub health_addr: SocketAddr,
    pub lease_id: String,
    pub lock_path: PathBuf,
    pub registry_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidecarState {
    Planned,
    Starting,
    Healthy,
    Failed,
    Stopped,
    Crashed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarRecord {
    pub lease_id: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub state: SidecarState,
}

/// A healthy sidecar endpoint discovered from the local registry.
///
/// This is deliberately not a [`SidecarHandle`]: a reused sidecar is owned by
/// the process that created its lease, so dropping a consumer must not kill the
/// shared child process or remove its lease. Callers may use the endpoint for
/// the duration of a request and let the original owner release it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReusedSidecar {
    pub record: SidecarRecord,
    pub health_addr: SocketAddr,
    pub lock_path: PathBuf,
    pub registry_path: PathBuf,
}

impl ReusedSidecar {
    pub fn lease_id(&self) -> &str {
        &self.record.lease_id
    }

    pub fn port(&self) -> u16 {
        self.record.port
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarRecoveryResult {
    pub lease_id: String,
    pub recovered: bool,
    pub state: SidecarState,
    pub detail: String,
}

#[derive(Debug)]
pub struct SidecarSupervisor {
    runtime_directory: PathBuf,
}

impl SidecarSupervisor {
    pub fn new(runtime_directory: impl Into<PathBuf>) -> Result<Self, ProtocolRuntimeError> {
        let runtime_directory = runtime_directory.into();
        fs::create_dir_all(&runtime_directory).map_err(|error| {
            ProtocolRuntimeError::transport(
                ProtocolKind::Unknown,
                "SIDECAR_RUNTIME_DIRECTORY_FAILED",
                error.to_string(),
            )
        })?;
        Ok(Self { runtime_directory })
    }

    pub fn prepare(
        &self,
        adapter_id: &str,
        command: impl Into<PathBuf>,
        arguments: Vec<String>,
        preferred_port: Option<u16>,
    ) -> Result<SidecarLaunchPlan, ProtocolRuntimeError> {
        validate_sidecar_id(adapter_id)?;
        let lease_id = format!("sidecar-{}-{}", safe_id(adapter_id), unique_suffix());
        let lock_path = self.runtime_directory.join(format!("{lease_id}.lock"));
        let registry_path = self.runtime_directory.join(format!("{lease_id}.json"));
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_LOCK_ACQUIRE_FAILED",
                    error.to_string(),
                )
            })?;
        let port = match select_local_port(preferred_port) {
            Ok(port) => port,
            Err(error) => {
                let _ = fs::remove_file(&lock_path);
                return Err(error);
            }
        };
        let plan = SidecarLaunchPlan {
            adapter_id: adapter_id.to_owned(),
            adapter_version: ADAPTER_VERSION.to_owned(),
            command: command.into(),
            arguments,
            port,
            health_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            lease_id,
            lock_path,
            registry_path,
        };
        if let Err(error) = self.write_record(&plan, SidecarState::Planned, None) {
            let _ = fs::remove_file(&plan.lock_path);
            return Err(error);
        }
        Ok(plan)
    }

    pub fn spawn(&self, plan: &SidecarLaunchPlan) -> Result<SidecarHandle, ProtocolRuntimeError> {
        let mut command = Command::new(&plan.command);
        command
            .args(&plan.arguments)
            .env("VIBEHUB_SIDECAR_PORT", plan.port.to_string())
            .env("VIBEHUB_SIDECAR_ADAPTER", &plan.adapter_id)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = command.spawn().map_err(|error| {
            let _ = self.release(plan);
            ProtocolRuntimeError::transport(
                ProtocolKind::Unknown,
                "SIDECAR_START_FAILED",
                error.to_string(),
            )
        })?;
        if let Err(error) = self.write_record(plan, SidecarState::Starting, Some(child.id())) {
            let mut child = child;
            let _ = child.kill();
            let _ = child.wait();
            let _ = self.release(plan);
            return Err(error);
        }
        Ok(SidecarHandle {
            supervisor: self.clone_for_handle(),
            plan: plan.clone(),
            child,
        })
    }

    pub fn mark_healthy(
        &self,
        plan: &SidecarLaunchPlan,
        pid: Option<u32>,
    ) -> Result<(), ProtocolRuntimeError> {
        self.write_record(plan, SidecarState::Healthy, pid)
    }

    pub fn health_check(
        &self,
        plan: &SidecarLaunchPlan,
        timeout: Duration,
    ) -> Result<(), ProtocolRuntimeError> {
        self.health_check_addr(plan.health_addr, timeout)
    }

    /// Find and reuse one already healthy sidecar for `adapter_id`.
    ///
    /// Registry entries are never trusted just because their state says
    /// `healthy`: the corresponding lock must exist, the record must match its
    /// filename, and a localhost TCP health check must succeed. Failed or
    /// malformed runtime state is cleaned up where it can be identified; no
    /// endpoint is returned unless all checks pass.
    pub fn reuse_healthy(
        &self,
        adapter_id: &str,
    ) -> Result<Option<ReusedSidecar>, ProtocolRuntimeError> {
        self.reuse_healthy_with_timeout(adapter_id, Duration::from_millis(100))
    }

    pub fn reuse_healthy_with_timeout(
        &self,
        adapter_id: &str,
        timeout: Duration,
    ) -> Result<Option<ReusedSidecar>, ProtocolRuntimeError> {
        validate_sidecar_id(adapter_id)?;

        let mut registry_paths = Vec::new();
        let entries = fs::read_dir(&self.runtime_directory).map_err(|error| {
            ProtocolRuntimeError::transport(
                ProtocolKind::Unknown,
                "SIDECAR_REGISTRY_SCAN_FAILED",
                error.to_string(),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_REGISTRY_SCAN_FAILED",
                    error.to_string(),
                )
            })?;
            let file_type = entry.file_type().map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_REGISTRY_SCAN_FAILED",
                    error.to_string(),
                )
            })?;
            if file_type.is_file() && entry.path().extension().is_some_and(|ext| ext == "json") {
                registry_paths.push(entry.path());
            }
        }
        registry_paths.sort();

        for registry_path in registry_paths {
            let bytes = match fs::read(&registry_path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(ProtocolRuntimeError::transport(
                        ProtocolKind::Unknown,
                        "SIDECAR_RECORD_READ_FAILED",
                        error.to_string(),
                    ))
                }
            };
            let record: SidecarRecord = serde_json::from_slice(&bytes).map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_RECORD_PARSE_FAILED",
                    error.to_string(),
                )
            })?;
            if record.adapter_id != adapter_id || record.state != SidecarState::Healthy {
                continue;
            }

            let lock_path = self.lock_path_for(&registry_path, &record.lease_id)?;
            if !lock_path.exists() || record.port == 0 {
                self.cleanup_registry_candidate(&registry_path, &record.lease_id)?;
                continue;
            }
            if record.adapter_version != ADAPTER_VERSION {
                self.cleanup_registry_candidate(&registry_path, &record.lease_id)?;
                continue;
            }

            let health_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), record.port);
            if self.health_check_addr(health_addr, timeout).is_err() {
                self.cleanup_registry_candidate(&registry_path, &record.lease_id)?;
                continue;
            }

            return Ok(Some(ReusedSidecar {
                record,
                health_addr,
                lock_path,
                registry_path,
            }));
        }

        Ok(None)
    }

    pub fn release(&self, plan: &SidecarLaunchPlan) -> Result<(), ProtocolRuntimeError> {
        let _ = fs::remove_file(&plan.registry_path);
        fs::remove_file(&plan.lock_path).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_LOCK_ALREADY_RELEASED",
                    error.to_string(),
                )
            } else {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_LOCK_RELEASE_FAILED",
                    error.to_string(),
                )
            }
        })
    }

    pub fn recover_stale(
        &self,
        plan: &SidecarLaunchPlan,
        process_alive: bool,
    ) -> Result<SidecarRecoveryResult, ProtocolRuntimeError> {
        if process_alive {
            return Ok(SidecarRecoveryResult {
                lease_id: plan.lease_id.clone(),
                recovered: false,
                state: SidecarState::Healthy,
                detail: "sidecar process is still alive; lock retained".to_owned(),
            });
        }
        let _ = fs::remove_file(&plan.registry_path);
        let _ = fs::remove_file(&plan.lock_path);
        Ok(SidecarRecoveryResult {
            lease_id: plan.lease_id.clone(),
            recovered: true,
            state: SidecarState::Crashed,
            detail: "stale sidecar registry and lock removed after process death".to_owned(),
        })
    }

    fn write_record(
        &self,
        plan: &SidecarLaunchPlan,
        state: SidecarState,
        pid: Option<u32>,
    ) -> Result<(), ProtocolRuntimeError> {
        let record = SidecarRecord {
            lease_id: plan.lease_id.clone(),
            adapter_id: plan.adapter_id.clone(),
            adapter_version: plan.adapter_version.clone(),
            port: plan.port,
            pid,
            state,
        };
        let bytes = serde_json::to_vec(&record).map_err(|error| {
            ProtocolRuntimeError::transport(
                ProtocolKind::Unknown,
                "SIDECAR_RECORD_SERIALIZE_FAILED",
                error.to_string(),
            )
        })?;
        let temporary = plan.registry_path.with_extension("json.tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_RECORD_WRITE_FAILED",
                    error.to_string(),
                )
            })?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_RECORD_WRITE_FAILED",
                    error.to_string(),
                )
            })?;
        drop(file);
        fs::rename(temporary, &plan.registry_path).map_err(|error| {
            ProtocolRuntimeError::transport(
                ProtocolKind::Unknown,
                "SIDECAR_RECORD_REPLACE_FAILED",
                error.to_string(),
            )
        })
    }

    fn clone_for_handle(&self) -> Self {
        Self {
            runtime_directory: self.runtime_directory.clone(),
        }
    }

    fn health_check_addr(
        &self,
        health_addr: SocketAddr,
        timeout: Duration,
    ) -> Result<(), ProtocolRuntimeError> {
        TcpStream::connect_timeout(&health_addr, timeout)
            .map(|_| ())
            .map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_HEALTH_CHECK_FAILED",
                    error.to_string(),
                )
            })
    }

    fn lock_path_for(
        &self,
        registry_path: &Path,
        lease_id: &str,
    ) -> Result<PathBuf, ProtocolRuntimeError> {
        validate_sidecar_lease_id(lease_id)?;
        let file_name = registry_path.file_name().and_then(|value| value.to_str());
        let expected_file_name = format!("{lease_id}.json");
        if file_name != Some(expected_file_name.as_str())
            || registry_path.parent() != Some(self.runtime_directory.as_path())
        {
            return Err(ProtocolRuntimeError::invalid(
                ProtocolKind::Unknown,
                "sidecar registry path does not match its lease",
            ));
        }
        Ok(self.runtime_directory.join(format!("{lease_id}.lock")))
    }

    fn cleanup_registry_candidate(
        &self,
        registry_path: &Path,
        lease_id: &str,
    ) -> Result<(), ProtocolRuntimeError> {
        let lock_path = self.lock_path_for(registry_path, lease_id)?;
        for path in [registry_path, lock_path.as_path()] {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(ProtocolRuntimeError::transport(
                        ProtocolKind::Unknown,
                        "SIDECAR_REUSE_CLEANUP_FAILED",
                        error.to_string(),
                    ))
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SidecarHandle {
    supervisor: SidecarSupervisor,
    pub plan: SidecarLaunchPlan,
    child: Child,
}

impl SidecarHandle {
    pub fn id(&self) -> &str {
        &self.plan.lease_id
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn wait_health(&mut self, timeout: Duration) -> Result<(), ProtocolRuntimeError> {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().map_err(|error| {
                ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_WAIT_FAILED",
                    error.to_string(),
                )
            })? {
                let _ = self.supervisor.release(&self.plan);
                return Err(ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_EXITED_BEFORE_HEALTH",
                    status.to_string(),
                ));
            }
            if self
                .supervisor
                .health_check(&self.plan, Duration::from_millis(40))
                .is_ok()
            {
                self.supervisor.mark_healthy(&self.plan, Some(self.pid()))?;
                return Ok(());
            }
            if started.elapsed() >= timeout {
                let _ = self.child.kill();
                let _ = self.child.wait();
                let _ = self.supervisor.release(&self.plan);
                return Err(ProtocolRuntimeError::transport(
                    ProtocolKind::Unknown,
                    "SIDECAR_HEALTH_TIMEOUT",
                    "sidecar did not become healthy before timeout",
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn shutdown(&mut self) -> Result<(), ProtocolRuntimeError> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.supervisor.release(&self.plan)
    }
}

impl Drop for SidecarHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn select_local_port(preferred: Option<u16>) -> Result<u16, ProtocolRuntimeError> {
    if let Some(preferred) = preferred {
        if preferred != 0 && TcpListener::bind((Ipv4Addr::LOCALHOST, preferred)).is_ok() {
            return Ok(preferred);
        }
    }
    TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| {
            ProtocolRuntimeError::transport(
                ProtocolKind::Unknown,
                "SIDECAR_PORT_ALLOCATE_FAILED",
                error.to_string(),
            )
        })
}

fn validate_sidecar_id(adapter_id: &str) -> Result<(), ProtocolRuntimeError> {
    if adapter_id.is_empty()
        || !adapter_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(ProtocolRuntimeError::invalid(
            ProtocolKind::Unknown,
            "sidecar adapter id is not path-safe",
        ));
    }
    Ok(())
}

fn validate_sidecar_lease_id(lease_id: &str) -> Result<(), ProtocolRuntimeError> {
    if lease_id.is_empty()
        || !lease_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(ProtocolRuntimeError::invalid(
            ProtocolKind::Unknown,
            "sidecar lease id is not path-safe",
        ));
    }
    Ok(())
}

fn safe_id(value: &str) -> String {
    value.replace('.', "-").replace('/', "-")
}

fn unique_suffix() -> String {
    Uuid::new_v4().simple().to_string()
}

fn object<'a>(
    value: &'a Value,
    protocol: ProtocolKind,
) -> Result<&'a Map<String, Value>, ProtocolRuntimeError> {
    value
        .as_object()
        .ok_or_else(|| ProtocolRuntimeError::invalid(protocol, "expected JSON object"))
}

fn array<'a>(
    value: Option<&'a Value>,
    protocol: ProtocolKind,
    field: &str,
) -> Result<&'a Vec<Value>, ProtocolRuntimeError> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| ProtocolRuntimeError::invalid(protocol, format!("{field} must be an array")))
}

fn required<'a>(
    value: Option<&'a Value>,
    field: &str,
    protocol: ProtocolKind,
) -> Result<&'a Value, ProtocolRuntimeError> {
    value.ok_or_else(|| ProtocolRuntimeError::invalid(protocol, format!("{field} is required")))
}

fn required_string<'a>(
    value: Option<&'a Value>,
    protocol: ProtocolKind,
    field: &str,
) -> Result<&'a str, ProtocolRuntimeError> {
    required(value, field, protocol)?
        .as_str()
        .ok_or_else(|| ProtocolRuntimeError::invalid(protocol, format!("{field} must be a string")))
}

fn string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_owned)
}

fn parse_openai_chat_message(value: &Value) -> Result<UnifiedMessage, ProtocolRuntimeError> {
    let message = object(value, ProtocolKind::OpenaiChatCompletions)?;
    let role = parse_role(
        required_string(
            message.get("role"),
            ProtocolKind::OpenaiChatCompletions,
            "role",
        )?,
        ProtocolKind::OpenaiChatCompletions,
    )?;
    let mut content = match message.get("content") {
        Some(Value::String(text)) => vec![UnifiedContent::Text { text: text.clone() }],
        Some(Value::Array(parts)) => parts
            .iter()
            .map(parse_openai_content_part)
            .collect::<Result<Vec<_>, _>>()?,
        Some(Value::Null) | None => Vec::new(),
        Some(_) => {
            return Err(ProtocolRuntimeError::invalid(
                ProtocolKind::OpenaiChatCompletions,
                "message content must be string or array",
            ))
        }
    };
    if let Some(calls) = message.get("tool_calls").and_then(Value::as_array) {
        for call in calls {
            let call = object(call, ProtocolKind::OpenaiChatCompletions)?;
            let function = object(
                required(
                    call.get("function"),
                    "function",
                    ProtocolKind::OpenaiChatCompletions,
                )?,
                ProtocolKind::OpenaiChatCompletions,
            )?;
            let args = required_string(
                function.get("arguments"),
                ProtocolKind::OpenaiChatCompletions,
                "arguments",
            )?;
            content.push(UnifiedContent::ToolUse {
                id: string(call.get("id")).unwrap_or_else(|| "tool-call".to_owned()),
                name: required_string(
                    function.get("name"),
                    ProtocolKind::OpenaiChatCompletions,
                    "name",
                )?
                .to_owned(),
                input: serde_json::from_str(args).map_err(|error| {
                    ProtocolRuntimeError::invalid(
                        ProtocolKind::OpenaiChatCompletions,
                        format!("tool arguments invalid: {error}"),
                    )
                })?,
            });
        }
    }
    if role == UnifiedRole::Tool {
        let tool_use_id =
            string(message.get("tool_call_id")).unwrap_or_else(|| "tool-call".to_owned());
        return Ok(UnifiedMessage {
            role,
            content: vec![UnifiedContent::ToolResult {
                tool_use_id,
                content,
                is_error: false,
            }],
        });
    }
    Ok(UnifiedMessage { role, content })
}

fn parse_openai_content_part(value: &Value) -> Result<UnifiedContent, ProtocolRuntimeError> {
    let part = object(value, ProtocolKind::OpenaiChatCompletions)?;
    match required_string(
        part.get("type"),
        ProtocolKind::OpenaiChatCompletions,
        "content.type",
    )? {
        "text" | "input_text" | "output_text" => Ok(UnifiedContent::Text {
            text: required_string(
                part.get("text"),
                ProtocolKind::OpenaiChatCompletions,
                "content.text",
            )?
            .to_owned(),
        }),
        "image_url" | "input_image" => {
            let image = part
                .get("image_url")
                .or_else(|| part.get("image"))
                .ok_or_else(|| {
                    ProtocolRuntimeError::invalid(
                        ProtocolKind::OpenaiChatCompletions,
                        "image content is missing image_url",
                    )
                })?;
            let image = object(image, ProtocolKind::OpenaiChatCompletions)?;
            Ok(UnifiedContent::ImageUrl {
                url: required_string(
                    image.get("url"),
                    ProtocolKind::OpenaiChatCompletions,
                    "image_url.url",
                )?
                .to_owned(),
                media_type: None,
            })
        }
        other => Err(ProtocolRuntimeError::unsupported(
            ProtocolKind::OpenaiChatCompletions,
            format!("unsupported content type {other}"),
        )),
    }
}

fn parse_anthropic_content(value: &Value) -> Result<Vec<UnifiedContent>, ProtocolRuntimeError> {
    if let Some(text) = value.as_str() {
        return Ok(vec![UnifiedContent::Text {
            text: text.to_owned(),
        }]);
    }
    let parts = value.as_array().ok_or_else(|| {
        ProtocolRuntimeError::invalid(
            ProtocolKind::AnthropicMessages,
            "content must be string or array",
        )
    })?;
    parts.iter().map(parse_anthropic_content_block).collect()
}

fn parse_anthropic_content_block(value: &Value) -> Result<UnifiedContent, ProtocolRuntimeError> {
    let block = object(value, ProtocolKind::AnthropicMessages)?;
    match required_string(
        block.get("type"),
        ProtocolKind::AnthropicMessages,
        "content.type",
    )? {
        "text" => Ok(UnifiedContent::Text {
            text: required_string(
                block.get("text"),
                ProtocolKind::AnthropicMessages,
                "content.text",
            )?
            .to_owned(),
        }),
        "thinking" => Ok(UnifiedContent::Reasoning {
            text: string(block.get("thinking")).unwrap_or_default(),
        }),
        "image" => {
            let source = object(
                required(
                    block.get("source"),
                    "source",
                    ProtocolKind::AnthropicMessages,
                )?,
                ProtocolKind::AnthropicMessages,
            )?;
            match required_string(
                source.get("type"),
                ProtocolKind::AnthropicMessages,
                "source.type",
            )? {
                "base64" => Ok(UnifiedContent::ImageBase64 {
                    data: required_string(
                        source.get("data"),
                        ProtocolKind::AnthropicMessages,
                        "source.data",
                    )?
                    .to_owned(),
                    media_type: required_string(
                        source.get("media_type"),
                        ProtocolKind::AnthropicMessages,
                        "source.media_type",
                    )?
                    .to_owned(),
                }),
                "url" => Ok(UnifiedContent::ImageUrl {
                    url: required_string(
                        source.get("url"),
                        ProtocolKind::AnthropicMessages,
                        "source.url",
                    )?
                    .to_owned(),
                    media_type: None,
                }),
                other => Err(ProtocolRuntimeError::unsupported(
                    ProtocolKind::AnthropicMessages,
                    format!("unsupported image source {other}"),
                )),
            }
        }
        "tool_use" => Ok(UnifiedContent::ToolUse {
            id: required_string(
                block.get("id"),
                ProtocolKind::AnthropicMessages,
                "tool_use.id",
            )?
            .to_owned(),
            name: required_string(
                block.get("name"),
                ProtocolKind::AnthropicMessages,
                "tool_use.name",
            )?
            .to_owned(),
            input: required(
                block.get("input"),
                "tool_use.input",
                ProtocolKind::AnthropicMessages,
            )?
            .clone(),
        }),
        "tool_result" => Ok(UnifiedContent::ToolResult {
            tool_use_id: required_string(
                block.get("tool_use_id"),
                ProtocolKind::AnthropicMessages,
                "tool_result.tool_use_id",
            )?
            .to_owned(),
            content: parse_anthropic_content(required(
                block.get("content"),
                "tool_result.content",
                ProtocolKind::AnthropicMessages,
            )?)?,
            is_error: block
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        other => Err(ProtocolRuntimeError::unsupported(
            ProtocolKind::AnthropicMessages,
            format!("unsupported content type {other}"),
        )),
    }
}

fn parse_openai_tool(value: &Value) -> Result<UnifiedTool, ProtocolRuntimeError> {
    let tool = object(value, ProtocolKind::OpenaiChatCompletions)?;
    let function = object(
        required(
            tool.get("function"),
            "function",
            ProtocolKind::OpenaiChatCompletions,
        )?,
        ProtocolKind::OpenaiChatCompletions,
    )?;
    Ok(UnifiedTool {
        name: required_string(
            function.get("name"),
            ProtocolKind::OpenaiChatCompletions,
            "tool.name",
        )?
        .to_owned(),
        description: string(function.get("description")),
        input_schema: required(
            function.get("parameters"),
            "tool.parameters",
            ProtocolKind::OpenaiChatCompletions,
        )?
        .clone(),
    })
}

fn parse_openai_responses_tool(value: &Value) -> Result<UnifiedTool, ProtocolRuntimeError> {
    let object = object(value, ProtocolKind::OpenaiResponses)?;
    if string(object.get("type")).as_deref() != Some("function") {
        return Err(ProtocolRuntimeError::unsupported(
            ProtocolKind::OpenaiResponses,
            "only function tools are supported",
        ));
    }
    Ok(UnifiedTool {
        name: required_string(
            object.get("name"),
            ProtocolKind::OpenaiResponses,
            "tool.name",
        )?
        .to_owned(),
        description: string(object.get("description")),
        input_schema: required(
            object.get("parameters"),
            "tool.parameters",
            ProtocolKind::OpenaiResponses,
        )?
        .clone(),
    })
}

fn parse_anthropic_tool(value: &Value) -> Result<UnifiedTool, ProtocolRuntimeError> {
    let object = object(value, ProtocolKind::AnthropicMessages)?;
    Ok(UnifiedTool {
        name: required_string(
            object.get("name"),
            ProtocolKind::AnthropicMessages,
            "tool.name",
        )?
        .to_owned(),
        description: string(object.get("description")),
        input_schema: required(
            object.get("input_schema"),
            "tool.input_schema",
            ProtocolKind::AnthropicMessages,
        )?
        .clone(),
    })
}

fn unified_message_to_openai_chat(message: &UnifiedMessage) -> Result<Value, ProtocolRuntimeError> {
    let role = match message.role {
        UnifiedRole::System => "system",
        UnifiedRole::Developer => "developer",
        UnifiedRole::User => "user",
        UnifiedRole::Assistant => "assistant",
        UnifiedRole::Tool => "tool",
    };
    if message.role == UnifiedRole::Tool {
        if let Some(UnifiedContent::ToolResult {
            tool_use_id,
            content,
            ..
        }) = message.content.first()
        {
            return Ok(json!({
                "role":"tool",
                "tool_call_id":tool_use_id,
                "content": content.iter().filter_map(content_text).collect::<Vec<_>>().join("\n")
            }));
        }
    }
    let mut object = Map::new();
    object.insert("role".to_owned(), Value::String(role.to_owned()));
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();
    for content in &message.content {
        match content {
            UnifiedContent::Text { text } => text_parts.push(Value::String(text.clone())),
            UnifiedContent::Reasoning { text } => {
                object.insert("reasoning_content".to_owned(), Value::String(text.clone()));
            }
            UnifiedContent::ToolUse { id, name, input } => tool_calls.push(json!({
                "id":id,
                "type":"function",
                "function":{
                    "name":name,
                    "arguments":serde_json::to_string(input).map_err(|error| ProtocolRuntimeError::invalid(ProtocolKind::OpenaiChatCompletions, error.to_string()))?
                }
            })),
            UnifiedContent::ImageUrl { url, .. } => {
                text_parts.push(json!({"type":"image_url","image_url":{"url":url}}))
            }
            UnifiedContent::ImageBase64 { data, media_type } => text_parts.push(json!({
                "type":"image_url",
                "image_url":{"url":format!("data:{media_type};base64,{data}")}
            })),
            UnifiedContent::ToolResult { .. } => {}
        }
    }
    if tool_calls.is_empty() {
        object.insert(
            "content".to_owned(),
            if text_parts.len() == 1 {
                text_parts.remove(0)
            } else {
                Value::Array(text_parts)
            },
        );
    } else {
        object.insert(
            "content".to_owned(),
            if text_parts.is_empty() {
                Value::Null
            } else {
                Value::Array(text_parts)
            },
        );
        object.insert("tool_calls".to_owned(), Value::Array(tool_calls));
    }
    Ok(Value::Object(object))
}

fn openai_tool(tool: &UnifiedTool) -> Value {
    json!({"type":"function","function":{"name":tool.name,"description":tool.description,"parameters":tool.input_schema}})
}

fn anthropic_tool(tool: &UnifiedTool) -> Value {
    json!({"name":tool.name,"description":tool.description,"input_schema":tool.input_schema})
}

fn anthropic_content(content: &UnifiedContent) -> Result<Value, ProtocolRuntimeError> {
    Ok(match content {
        UnifiedContent::Text { text } => json!({"type":"text","text":text}),
        UnifiedContent::Reasoning { text } => json!({"type":"thinking","thinking":text}),
        UnifiedContent::ImageUrl { url, .. } => {
            json!({"type":"image","source":{"type":"url","url":url}})
        }
        UnifiedContent::ImageBase64 { data, media_type } => {
            json!({"type":"image","source":{"type":"base64","media_type":media_type,"data":data}})
        }
        UnifiedContent::ToolUse { id, name, input } => {
            json!({"type":"tool_use","id":id,"name":name,"input":input})
        }
        UnifiedContent::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => json!({
            "type":"tool_result",
            "tool_use_id":tool_use_id,
            "content":content.iter().map(anthropic_content).collect::<Result<Vec<_>, _>>()?,
            "is_error":is_error
        }),
    })
}

fn openai_responses_tool(tool: &UnifiedTool) -> Value {
    json!({"type":"function","name":tool.name,"description":tool.description,"parameters":tool.input_schema})
}

fn content_text(content: &UnifiedContent) -> Option<String> {
    match content {
        UnifiedContent::Text { text } | UnifiedContent::Reasoning { text } => Some(text.clone()),
        _ => None,
    }
}

fn parse_role(role: &str, protocol: ProtocolKind) -> Result<UnifiedRole, ProtocolRuntimeError> {
    match role {
        "system" => Ok(UnifiedRole::System),
        "developer" => Ok(UnifiedRole::Developer),
        "user" => Ok(UnifiedRole::User),
        "assistant" => Ok(UnifiedRole::Assistant),
        "tool" => Ok(UnifiedRole::Tool),
        other => Err(ProtocolRuntimeError::unsupported(
            protocol,
            format!("unsupported role {other}"),
        )),
    }
}

fn parse_openai_response_input_item(
    value: &Value,
    messages: &mut Vec<UnifiedMessage>,
) -> Result<(), ProtocolRuntimeError> {
    let object = object(value, ProtocolKind::OpenaiResponses)?;
    match required_string(
        object.get("type"),
        ProtocolKind::OpenaiResponses,
        "input.type",
    )? {
        "message" => {
            let role = parse_role(
                required_string(
                    object.get("role"),
                    ProtocolKind::OpenaiResponses,
                    "input.role",
                )?,
                ProtocolKind::OpenaiResponses,
            )?;
            messages.push(UnifiedMessage {
                role,
                content: parse_openai_response_content(object.get("content"))?,
            });
        }
        "function_call_output" => messages.push(UnifiedMessage {
            role: UnifiedRole::Tool,
            content: vec![UnifiedContent::ToolResult {
                tool_use_id: required_string(
                    object.get("call_id"),
                    ProtocolKind::OpenaiResponses,
                    "call_id",
                )?
                .to_owned(),
                content: vec![UnifiedContent::Text {
                    text: string(object.get("output")).unwrap_or_default(),
                }],
                is_error: false,
            }],
        }),
        "function_call" => messages.push(UnifiedMessage {
            role: UnifiedRole::Assistant,
            content: vec![UnifiedContent::ToolUse {
                id: string(object.get("call_id")).unwrap_or_else(|| "tool-call".to_owned()),
                name: required_string(object.get("name"), ProtocolKind::OpenaiResponses, "name")?
                    .to_owned(),
                input: serde_json::from_str(required_string(
                    object.get("arguments"),
                    ProtocolKind::OpenaiResponses,
                    "arguments",
                )?)
                .map_err(|error| {
                    ProtocolRuntimeError::invalid(ProtocolKind::OpenaiResponses, error.to_string())
                })?,
            }],
        }),
        other => {
            return Err(ProtocolRuntimeError::unsupported(
                ProtocolKind::OpenaiResponses,
                format!("unsupported input type {other}"),
            ))
        }
    }
    Ok(())
}

fn parse_openai_response_content(
    value: Option<&Value>,
) -> Result<Vec<UnifiedContent>, ProtocolRuntimeError> {
    match value {
        Some(Value::String(text)) => Ok(vec![UnifiedContent::Text { text: text.clone() }]),
        Some(Value::Array(parts)) => parts.iter().map(parse_openai_content_part).collect(),
        _ => Err(ProtocolRuntimeError::invalid(
            ProtocolKind::OpenaiResponses,
            "message content is missing",
        )),
    }
}

fn openai_response_input_item(
    role: UnifiedRole,
    content: &UnifiedContent,
) -> Result<Value, ProtocolRuntimeError> {
    let role = match role {
        UnifiedRole::System => "system",
        UnifiedRole::Developer => "developer",
        UnifiedRole::User => "user",
        UnifiedRole::Assistant => "assistant",
        UnifiedRole::Tool => "user",
    };
    match content {
        UnifiedContent::Text { text } => {
            Ok(json!({"type":"message","role":role,"content":[{"type":"input_text","text":text}]}))
        }
        UnifiedContent::ImageUrl { url, .. } => Ok(
            json!({"type":"message","role":role,"content":[{"type":"input_image","image_url":url}]}),
        ),
        UnifiedContent::ImageBase64 { data, media_type } => Ok(
            json!({"type":"message","role":role,"content":[{"type":"input_image","image_url":format!("data:{media_type};base64,{data}")}]}),
        ),
        UnifiedContent::ToolUse { id, name, input } => Ok(
            json!({"type":"function_call","call_id":id,"name":name,"arguments":serde_json::to_string(input).map_err(|error| ProtocolRuntimeError::invalid(ProtocolKind::OpenaiResponses, error.to_string()))?}),
        ),
        UnifiedContent::ToolResult {
            tool_use_id,
            content,
            ..
        } => Ok(
            json!({"type":"function_call_output","call_id":tool_use_id,"output":content.iter().filter_map(content_text).collect::<Vec<_>>().join("\n")}),
        ),
        UnifiedContent::Reasoning { text } => Ok(
            json!({"type":"message","role":"assistant","content":[{"type":"output_text","text":text}]}),
        ),
    }
}

fn parse_openai_usage(value: &Value) -> Option<UnifiedUsage> {
    let object = value.as_object()?;
    Some(UnifiedUsage {
        input_tokens: object
            .get("prompt_tokens")
            .or_else(|| object.get("input_tokens"))
            .and_then(Value::as_u64),
        output_tokens: object
            .get("completion_tokens")
            .or_else(|| object.get("output_tokens"))
            .and_then(Value::as_u64),
        cache_read_tokens: object
            .get("prompt_tokens_details")
            .and_then(Value::as_object)
            .and_then(|details| details.get("cached_tokens"))
            .and_then(Value::as_u64),
        cache_creation_tokens: None,
    })
}

fn parse_anthropic_usage(value: &Value) -> Option<UnifiedUsage> {
    let object = value.as_object()?;
    Some(UnifiedUsage {
        input_tokens: object.get("input_tokens").and_then(Value::as_u64),
        output_tokens: object.get("output_tokens").and_then(Value::as_u64),
        cache_read_tokens: object
            .get("cache_read_input_tokens")
            .and_then(Value::as_u64),
        cache_creation_tokens: object
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::net::TcpListener;
    use uuid::Uuid;

    #[test]
    fn resolver_covers_direct_and_required_adapter_matrix() {
        let full = ModelProtocolCapabilities::default();
        assert_eq!(
            resolve_protocol(
                ProtocolKind::OpenaiResponses,
                ProtocolKind::OpenaiResponses,
                &full
            )
            .route,
            ProtocolRoute::Direct
        );
        assert_eq!(
            resolve_protocol(
                ProtocolKind::OpenaiResponses,
                ProtocolKind::OpenaiChatCompletions,
                &full
            )
            .route,
            ProtocolRoute::Adapter
        );
        assert_eq!(
            resolve_protocol(
                ProtocolKind::OpenaiResponses,
                ProtocolKind::AnthropicMessages,
                &full
            )
            .route,
            ProtocolRoute::Adapter
        );
        assert_eq!(
            resolve_protocol(
                ProtocolKind::AnthropicMessages,
                ProtocolKind::AnthropicMessages,
                &full
            )
            .route,
            ProtocolRoute::Direct
        );
        assert_eq!(
            resolve_protocol(
                ProtocolKind::AnthropicMessages,
                ProtocolKind::OpenaiChatCompletions,
                &full
            )
            .route,
            ProtocolRoute::Adapter
        );
        assert_eq!(
            resolve_protocol(
                ProtocolKind::AnthropicMessages,
                ProtocolKind::OpenaiResponses,
                &full
            )
            .route,
            ProtocolRoute::Adapter
        );
        assert_eq!(
            resolve_protocol(ProtocolKind::Unknown, ProtocolKind::OpenaiResponses, &full).route,
            ProtocolRoute::Unavailable
        );
    }

    #[test]
    fn chat_and_anthropic_round_trip_text_tools_images_and_reasoning() {
        let chat = json!({
            "model":"gpt",
            "reasoning_effort":"high",
            "stream":true,
            "messages":[
                {"role":"system","content":"rules"},
                {"role":"user","content":[
                    {"type":"text","text":"look"},
                    {"type":"image_url","image_url":{"url":"https://img.invalid/a.png"}}
                ]},
                {"role":"assistant","tool_calls":[
                    {"id":"call_1","type":"function","function":{"name":"read","arguments":"{\"path\":\"a\"}"}}
                ],"content":null},
                {"role":"tool","tool_call_id":"call_1","content":"done"}
            ],
            "tools":[{"type":"function","function":{"name":"read","description":"read file","parameters":{"type":"object"}}}]
        });
        let unified = openai_chat_to_unified(&chat).unwrap();
        assert_eq!(unified.messages.len(), 4);
        assert_eq!(unified.tools.len(), 1);
        let anthropic = unified_to_anthropic_messages(&unified).unwrap();
        assert_eq!(anthropic["stream"], true);
        assert_eq!(anthropic["tools"][0]["name"], "read");
        let round_trip = anthropic_messages_to_unified(&anthropic).unwrap();
        assert!(round_trip.messages.iter().any(|message| message
            .content
            .iter()
            .any(|content| matches!(content, UnifiedContent::ToolUse { .. }))));
    }

    #[test]
    fn responses_conversion_preserves_system_multi_turn_and_tools() {
        let responses = json!({
            "model":"gpt",
            "instructions":"be concise",
            "input":[
                {"type":"message","role":"user","content":[{"type":"input_text","text":"hello"}]},
                {"type":"function_call","call_id":"c1","name":"search","arguments":"{\"q\":\"v\"}"},
                {"type":"function_call_output","call_id":"c1","output":"result"}
            ],
            "tools":[{"type":"function","name":"search","parameters":{"type":"object"}}]
        });
        let unified = openai_responses_to_unified(&responses).unwrap();
        let output = unified_to_openai_responses(&unified).unwrap();
        assert_eq!(output["instructions"], "be concise");
        assert_eq!(output["tools"][0]["name"], "search");
        assert!(output["input"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["type"] == "function_call_output"));
    }

    #[test]
    fn stream_usage_errors_cancel_timeout_and_backpressure_are_structured() {
        let events = openai_chat_stream_event_to_unified(&json!({
            "choices":[{"delta":{"content":"hi","reasoning_content":"think"},"finish_reason":null}],
            "usage":{"prompt_tokens":2,"completion_tokens":3}
        }))
        .unwrap();
        assert!(events
            .iter()
            .any(|event| matches!(event, UnifiedStreamEvent::TextDelta { .. })));
        assert!(events
            .iter()
            .any(|event| matches!(event, UnifiedStreamEvent::Usage { .. })));
        let responses_events = openai_responses_stream_event_to_unified(&json!({
            "type":"response.output_text.delta",
            "delta":"hello"
        }))
        .unwrap();
        assert!(matches!(
            responses_events[0],
            UnifiedStreamEvent::TextDelta { .. }
        ));
        let upstream_error = parse_upstream_error(
            ProtocolKind::OpenaiResponses,
            &json!({"error":{"type":"rate_limit_error","message":"slow down"}}),
            Some(429),
        );
        assert!(upstream_error.retryable);
        assert_eq!(upstream_error.code, "rate_limit_error");
        let error = enforce_backpressure(3, Some(2)).unwrap_err();
        assert_eq!(error.code, "PROTOCOL_BACKPRESSURE");
        let control = RequestControl::new(Some(Duration::from_millis(0)));
        assert_eq!(
            control
                .check(ProtocolKind::OpenaiResponses)
                .unwrap_err()
                .code,
            "REQUEST_TIMEOUT"
        );
        control.cancel();
        assert_eq!(
            control
                .check(ProtocolKind::OpenaiResponses)
                .unwrap_err()
                .code,
            "REQUEST_CANCELLED"
        );
    }

    #[test]
    fn anthropic_stream_and_response_usage_are_converted() {
        let event = anthropic_stream_event_to_unified(&json!({
            "type":"content_block_delta",
            "delta":{"type":"text_delta","text":"hello"}
        }))
        .unwrap();
        assert!(matches!(event[0], UnifiedStreamEvent::TextDelta { .. }));
        let response = anthropic_response_to_unified(&json!({
            "id":"m1",
            "content":[{"type":"text","text":"done"}],
            "usage":{"input_tokens":5,"output_tokens":2},
            "stop_reason":"end_turn"
        }))
        .unwrap();
        assert_eq!(response.usage.unwrap().output_tokens, Some(2));
    }

    #[test]
    fn sidecar_port_conflict_lock_lifecycle_and_crash_recovery_are_deterministic() {
        let root = env::temp_dir().join(format!("vibehub-sidecar-{}", Uuid::new_v4()));
        let supervisor = SidecarSupervisor::new(&root).unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let occupied = listener.local_addr().unwrap().port();
        let plan = supervisor
            .prepare(
                "anthropic-to-responses",
                "/bin/false",
                Vec::new(),
                Some(occupied),
            )
            .unwrap();
        assert_ne!(plan.port, occupied);
        assert!(plan.lock_path.exists());
        supervisor.mark_healthy(&plan, Some(123)).unwrap();
        let alive = supervisor.recover_stale(&plan, true).unwrap();
        assert!(!alive.recovered);
        let recovered = supervisor.recover_stale(&plan, false).unwrap();
        assert!(recovered.recovered);
        supervisor.release(&plan).unwrap_err();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn healthy_sidecar_is_reused_and_unhealthy_registry_is_cleaned() {
        let root = env::temp_dir().join(format!("vibehub-sidecar-reuse-{}", Uuid::new_v4()));
        let supervisor = SidecarSupervisor::new(&root).unwrap();
        let plan = supervisor
            .prepare("anthropic-to-responses", "/bin/false", Vec::new(), None)
            .unwrap();
        supervisor.mark_healthy(&plan, Some(123)).unwrap();
        let listener = TcpListener::bind(plan.health_addr).unwrap();

        let reused = supervisor
            .reuse_healthy_with_timeout("anthropic-to-responses", Duration::from_millis(100))
            .unwrap()
            .expect("healthy sidecar should be reused");
        assert_eq!(reused.lease_id(), plan.lease_id);
        assert_eq!(reused.port(), plan.port);
        assert_eq!(reused.health_addr, plan.health_addr);
        assert!(reused.lock_path.exists());
        assert!(reused.registry_path.exists());

        drop(listener);
        assert!(supervisor
            .reuse_healthy_with_timeout("anthropic-to-responses", Duration::from_millis(20))
            .unwrap()
            .is_none());
        assert!(!plan.lock_path.exists());
        assert!(!plan.registry_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sidecar_failed_spawn_releases_lock_and_unknown_ids_fail_closed() {
        let root = env::temp_dir().join(format!("vibehub-sidecar-failed-{}", Uuid::new_v4()));
        let supervisor = SidecarSupervisor::new(&root).unwrap();
        let error = supervisor
            .prepare("../escape", "/bin/false", Vec::new(), None)
            .unwrap_err();
        assert_eq!(error.code, "PROTOCOL_PAYLOAD_INVALID");
        let plan = supervisor
            .prepare("test-adapter", "/definitely/missing", Vec::new(), None)
            .unwrap();
        let error = supervisor.spawn(&plan).unwrap_err();
        assert_eq!(error.code, "SIDECAR_START_FAILED");
        assert!(!plan.lock_path.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
