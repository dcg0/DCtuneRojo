//! OpenAI Chat Completions provider.
//!
//! Speaks the `POST /chat/completions` protocol used by OpenAI and any
//! OpenAI-compatible endpoint (OpenRouter, Ollama's `/v1`, LM Studio, vLLM,
//! etc.). Set [`OpenAiProvider::base_url`] to point at a compatible host.

use crate::llm::provider::Provider;
use crate::llm::types::{
    ChatRequest, ChatResponse, FinishReason, LlmError, Message, MessageRole, ToolCall,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The OpenAI Chat Completions provider.
pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiProvider {
    /// Construct. `base_url` should NOT include the trailing
    /// `/chat/completions` path (it is appended). Default for hosted OpenAI:
    /// `https://api.openai.com/v1`.
    pub fn new(client: reqwest::Client, base_url: String, api_key: String, model: String) -> Self {
        let base_url = if base_url.is_empty() {
            "https://api.openai.com/v1".to_string()
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
        format!("{}/chat/completions", self.base_url)
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // Translate tools into the OpenAI "tools" array shape.
        let tools: Vec<OpenAiTool> = req
            .tools
            .iter()
            .map(|t| OpenAiTool {
                r#type: "function".to_string(),
                function: crate::llm::types::ToolFunction::from(t.clone()),
            })
            .collect();

        // Capture emptiness before `tools` is moved into the request body so
        // we can decide `tool_choice` afterwards.
        let has_tools = !tools.is_empty();

        // Translate messages, dropping fields that OpenAI doesn't want on
        // non-tool messages.
        let messages: Vec<OpenAiMessage> = req
            .messages
            .iter()
            .map(OpenAiMessage::from_generic)
            .collect();

        let body = OpenAiRequest {
            model: self.model.clone(),
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            temperature: req.temperature,
            max_tokens: req.max_tokens,
            tool_choice: if has_tools {
                Some("auto".to_string())
            } else {
                None
            },
        };

        let mut builder = self.client.post(self.endpoint()).json(&body);
        if !self.api_key.is_empty() {
            builder = builder.bearer_auth(&self.api_key);
        }

        let resp = builder.send().await.map_err(LlmError::from)?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status_error(status, text));
        }

        let parsed: OpenAiResponse = resp.json().await.map_err(LlmError::from)?;
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
// Wire types (OpenAI-specific JSON shapes)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenAiTool {
    r#type: String,
    function: crate::llm::types::ToolFunction,
}

#[derive(Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl OpenAiMessage {
    fn from_generic(m: &Message) -> Self {
        let role = match m.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        let tool_calls: Option<Vec<OpenAiToolCall>> = if m.tool_calls.is_empty() {
            None
        } else {
            Some(
                m.tool_calls
                    .iter()
                    .map(|tc| OpenAiToolCall {
                        id: tc.id.clone(),
                        r#type: "function".to_string(),
                        function: OpenAiToolCallFn {
                            name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    })
                    .collect(),
            )
        };
        Self {
            role: role.to_string(),
            content: Some(m.content.clone()),
            tool_calls,
            tool_call_id: None,
            name: m.tool_name.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    r#type: String,
    function: OpenAiToolCallFn,
}

#[derive(Serialize, Deserialize)]
struct OpenAiToolCallFn {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiChoiceMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiChoiceMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

impl OpenAiResponse {
    fn into_generic(self) -> ChatResponse {
        let choice = self.choices.into_iter().next();
        let (content, tool_calls, finish_reason_str) = match choice {
            Some(c) => {
                let tc: Vec<ToolCall> = c
                    .message
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|wt| ToolCall {
                        id: wt.id,
                        name: wt.function.name,
                        arguments: wt.function.arguments,
                    })
                    .collect();
                (
                    c.message.content.unwrap_or_default(),
                    tc,
                    c.finish_reason.unwrap_or_default(),
                )
            }
            None => (String::new(), Vec::new(), String::new()),
        };
        ChatResponse {
            content,
            tool_calls,
            finish_reason: match finish_reason_str.as_str() {
                "stop" => FinishReason::Stop,
                "tool_calls" => FinishReason::ToolCalls,
                "length" => FinishReason::Length,
                "content_filter" => FinishReason::ContentFilter,
                other => FinishReason::Other(other.to_string()),
            },
            usage: self.usage.map(|u| crate::llm::types::Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::Message;

    #[test]
    fn endpoint_appended_correctly() {
        let p = OpenAiProvider::new(
            reqwest::Client::new(),
            "https://api.openai.com/v1".into(),
            "k".into(),
            "gpt-4o".into(),
        );
        assert_eq!(p.endpoint(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn trailing_slash_trimmed() {
        let p = OpenAiProvider::new(
            reqwest::Client::new(),
            "https://api.openai.com/v1/".into(),
            "k".into(),
            "gpt-4o".into(),
        );
        assert_eq!(p.endpoint(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn empty_base_url_defaults() {
        let p = OpenAiProvider::new(reqwest::Client::new(), "".into(), "k".into(), "m".into());
        assert_eq!(p.endpoint(), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn parses_choice_with_tool_call() {
        let json = r#"{
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "propose_constant_change", "arguments": "{\"name\":\"reqFuel\",\"value\":9.5}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120}
        }"#;
        let parsed: OpenAiResponse = serde_json::from_str(json).unwrap();
        let g = parsed.into_generic();
        assert_eq!(g.finish_reason, FinishReason::ToolCalls);
        assert_eq!(g.tool_calls.len(), 1);
        assert_eq!(g.tool_calls[0].name, "propose_constant_change");
        assert!(g.usage.unwrap().total_tokens == 120);
    }

    #[test]
    fn message_translation_drops_empty_tool_calls() {
        let m = OpenAiMessage::from_generic(&Message::user("hello"));
        let s = serde_json::to_string(&m).unwrap();
        // empty tool_calls should be skipped entirely
        assert!(!s.contains("tool_calls"));
    }
}
