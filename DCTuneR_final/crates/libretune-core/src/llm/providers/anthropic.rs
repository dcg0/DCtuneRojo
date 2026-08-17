//! Anthropic Messages API provider.
//!
//! Speaks `POST /v1/messages` (https://docs.anthropic.com). Differs from
//! OpenAI's shape in several ways: a top-level `system` field instead of a
//! system message, tool calls via a `tool_use` content block, and tool
//! results via a `tool_result` content block.

use crate::llm::provider::Provider;
use crate::llm::types::{ChatRequest, ChatResponse, FinishReason, LlmError, MessageRole, ToolCall};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(client: reqwest::Client, base_url: String, api_key: String, model: String) -> Self {
        let base_url = if base_url.is_empty() {
            "https://api.anthropic.com".to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };
        Self {
            client,
            base_url,
            api_key,
            model,
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // Split out the system prompt (Anthropic wants it top-level).
        let mut system: Option<String> = None;
        let mut messages: Vec<AnthropicMessage> = Vec::new();
        for m in &req.messages {
            match m.role {
                MessageRole::System => {
                    // Concatenate multiple system messages.
                    match &mut system {
                        Some(s) => {
                            s.push('\n');
                            s.push_str(&m.content);
                        }
                        None => system = Some(m.content.clone()),
                    }
                }
                MessageRole::User => messages.push(AnthropicMessage {
                    role: "user".into(),
                    content: vec![AnthropicContent::Text {
                        text: m.content.clone(),
                    }],
                }),
                MessageRole::Assistant => {
                    // Assistant message may carry tool_calls.
                    let mut blocks: Vec<AnthropicContent> =
                        Vec::with_capacity(1 + m.tool_calls.len());
                    if !m.content.is_empty() {
                        blocks.push(AnthropicContent::Text {
                            text: m.content.clone(),
                        });
                    }
                    for tc in &m.tool_calls {
                        blocks.push(AnthropicContent::ToolUse {
                            id: tc.id.clone(),
                            name: tc.name.clone(),
                            input: serde_json::from_str(&tc.arguments).unwrap_or_default(),
                        });
                    }
                    messages.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: blocks,
                    });
                }
                MessageRole::Tool => {
                    // Tool result: Anthropic wraps it in a user message with a
                    // tool_result content block.
                    messages.push(AnthropicMessage {
                        role: "user".into(),
                        content: vec![AnthropicContent::ToolResult {
                            tool_use_id: m.tool_name.clone().unwrap_or_default(),
                            content: m.content.clone(),
                        }],
                    });
                }
            }
        }

        let tools: Vec<AnthropicTool> = req
            .tools
            .iter()
            .map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            })
            .collect();

        let body = AnthropicRequest {
            model: self.model.clone(),
            system,
            messages,
            max_tokens: req.max_tokens.unwrap_or(1024),
            temperature: req.temperature,
            tools: if tools.is_empty() { None } else { Some(tools) },
        };

        let resp = self
            .client
            .post(self.endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(LlmError::from)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status_error(status, text));
        }

        let parsed: AnthropicResponse = resp.json().await.map_err(LlmError::from)?;
        Ok(parsed.into_generic())
    }
}

fn map_status_error(status: reqwest::StatusCode, body: String) -> LlmError {
    match status {
        reqwest::StatusCode::UNAUTHORIZED | reqwest::StatusCode::FORBIDDEN => {
            LlmError::Auth(format!("HTTP {status}: {body}"))
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            LlmError::RateLimit(format!("HTTP {status}: {body}"))
        }
        _ => LlmError::ApiError(format!("HTTP {status}: {body}")),
    }
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicResponseContent>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicResponseContent {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

impl AnthropicResponse {
    fn into_generic(self) -> ChatResponse {
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        for block in self.content {
            match block {
                AnthropicResponseContent::Text { text } => content.push_str(&text),
                AnthropicResponseContent::ToolUse { id, name, input } => {
                    let arguments = serde_json::to_string(&input).unwrap_or_default();
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments,
                    });
                }
            }
        }
        let finish_reason = match self.stop_reason.as_deref() {
            Some("end_turn") => FinishReason::Stop,
            Some("tool_use") => FinishReason::ToolCalls,
            Some("max_tokens") => FinishReason::Length,
            other => FinishReason::Other(other.unwrap_or("").to_string()),
        };
        ChatResponse {
            content,
            tool_calls,
            finish_reason,
            usage: self.usage.map(|u| crate::llm::types::Usage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_built() {
        let p = AnthropicProvider::new(
            reqwest::Client::new(),
            "".into(),
            "k".into(),
            "claude-3-5-sonnet-20241022".into(),
        );
        assert_eq!(p.endpoint(), "https://api.anthropic.com/v1/messages");
    }

    #[test]
    fn parses_tool_use_response() {
        let json = r#"{
            "content": [
                {"type":"text","text":"I'll enable launch control."},
                {"type":"tool_use","id":"tu_1","name":"propose_constant_change","input":{"name":"launchEnabled","value":1}}
            ],
            "stop_reason":"tool_use",
            "usage":{"input_tokens":50,"output_tokens":30}
        }"#;
        let parsed: AnthropicResponse = serde_json::from_str(json).unwrap();
        let g = parsed.into_generic();
        assert_eq!(g.finish_reason, FinishReason::ToolCalls);
        assert_eq!(g.tool_calls.len(), 1);
        assert!(g.content.contains("launch control"));
        assert_eq!(g.usage.unwrap().total_tokens, 80);
    }
}
