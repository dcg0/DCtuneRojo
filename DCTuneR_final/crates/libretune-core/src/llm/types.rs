//! Wire types for the LLM provider abstraction.
//!
//! These are intentionally generic (not OpenAI-shaped) so they can map onto
//! any provider's chat-completion API. Each concrete provider
//! ([`crate::llm::providers`]) translates to/from its own JSON wire format.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A role for a chat message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    /// A tool-call result returned to the model.
    Tool,
}

/// A single chat message. `content` is the natural-language text; `tool_calls`
/// is populated by assistant messages that requested tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Tool calls emitted by an assistant message. Empty for user/system msgs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// For role=Tool messages: the name of the tool that produced this result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_name: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_name: None,
        }
    }

    /// Create an assistant message with text only.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_name: None,
        }
    }
}

/// A tool/function the model may call. Maps to OpenAI's `tools` /
/// Anthropic's `tools` / Google's `functionDeclarations`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    /// JSON-schema-ish parameter description (serialized as-is to the wire).
    pub parameters: serde_json::Value,
}

/// The function part of a tool definition. Kept for OpenAI-compatible
/// providers that wrap the schema in a `function` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl From<ToolDef> for ToolFunction {
    fn from(t: ToolDef) -> Self {
        Self {
            name: t.name,
            description: t.description,
            parameters: t.parameters,
        }
    }
}

/// A tool call the model wants executed. `arguments` is the raw JSON string
/// the provider returned (parsed lazily by the orchestrator).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Provider-assigned id (used to correlate tool results in multi-turn).
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Raw JSON arguments string.
    pub arguments: String,
}

/// A request to a chat-completion endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Ordered conversation history, ending with the latest user turn.
    pub messages: Vec<Message>,
    /// Tools the model is allowed to call.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDef>,
    /// Sampling temperature (provider-specific defaults when omitted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum output tokens, if the provider supports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            tools: Vec::new(),
            temperature: None,
            max_tokens: None,
        }
    }
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// The model emitted a normal completion.
    Stop,
    /// The model requested one or more tool calls.
    ToolCalls,
    /// Hit the max-tokens limit.
    Length,
    /// Provider content-filter triggered.
    ContentFilter,
    /// Some other provider-specific reason.
    Other(String),
}

/// A chat-completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The assistant's natural-language message (may be empty when only tool
    /// calls were emitted).
    pub content: String,
    /// Tool calls the model requested, if any.
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    /// Model-reported usage stats, when available.
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// Token usage for a response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Errors returned by the provider layer.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum LlmError {
    #[error("network error: {0}")]
    Network(String),
    #[error("authentication failed (check API key): {0}")]
    Auth(String),
    #[error("rate limited by provider: {0}")]
    RateLimit(String),
    #[error("could not parse provider response: {0}")]
    Parse(String),
    #[error("provider returned an error: {0}")]
    ApiError(String),
    #[error("invalid configuration: {0}")]
    Config(String),
}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_status() {
            match e.status() {
                Some(reqwest::StatusCode::UNAUTHORIZED) => LlmError::Auth("HTTP 401".into()),
                Some(reqwest::StatusCode::TOO_MANY_REQUESTS) => {
                    LlmError::RateLimit("HTTP 429".into())
                }
                _ => LlmError::ApiError(e.to_string()),
            }
        } else {
            // Connect / timeout / other transport errors are all network.
            LlmError::Network(e.to_string())
        }
    }
}

impl From<serde_json::Error> for LlmError {
    fn from(e: serde_json::Error) -> Self {
        LlmError::Parse(e.to_string())
    }
}
