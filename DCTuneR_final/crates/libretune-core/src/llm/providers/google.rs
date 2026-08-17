//! Google Gemini API provider.
//!
//! Speaks `POST /v1beta/models/{model}:generateContent`. Uses the API key as
//! a query parameter (`?key=...`) per Google's convention. Tool calls come
//! back as `functionCall` parts.

use crate::llm::provider::Provider;
use crate::llm::types::{ChatRequest, ChatResponse, FinishReason, LlmError, MessageRole, ToolCall};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct GoogleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl GoogleProvider {
    pub fn new(client: reqwest::Client, base_url: String, api_key: String, model: String) -> Self {
        let base_url = if base_url.is_empty() {
            "https://generativelanguage.googleapis.com".to_string()
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
        format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        )
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        // System instructions go in a separate `systemInstruction` field.
        let mut system_instruction: Option<GeminiContent> = None;
        let mut contents: Vec<GeminiContent> = Vec::new();
        let mut sys_parts: Vec<GeminiPart> = Vec::new();
        for m in &req.messages {
            match m.role {
                MessageRole::System => {
                    sys_parts.push(GeminiPart::Text {
                        text: m.content.clone(),
                    });
                }
                MessageRole::User => contents.push(GeminiContent {
                    role: "user".into(),
                    parts: vec![GeminiPart::Text {
                        text: m.content.clone(),
                    }],
                }),
                MessageRole::Assistant => {
                    let mut parts: Vec<GeminiPart> = Vec::new();
                    if !m.content.is_empty() {
                        parts.push(GeminiPart::Text {
                            text: m.content.clone(),
                        });
                    }
                    for tc in &m.tool_calls {
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.arguments).unwrap_or_default();
                        parts.push(GeminiPart::FunctionCall {
                            name: tc.name.clone(),
                            args,
                        });
                    }
                    contents.push(GeminiContent {
                        role: "model".into(),
                        parts,
                    });
                }
                MessageRole::Tool => {
                    let resp_json = serde_json::Value::String(m.content.clone());
                    contents.push(GeminiContent {
                        role: "function".into(),
                        parts: vec![GeminiPart::FunctionResponse {
                            name: m.tool_name.clone().unwrap_or_default(),
                            response: serde_json::json!({ "result": resp_json }),
                        }],
                    });
                }
            }
        }
        if !sys_parts.is_empty() {
            system_instruction = Some(GeminiContent {
                role: "user".into(),
                parts: sys_parts,
            });
        }

        let tools: Vec<GeminiTool> = req
            .tools
            .iter()
            .map(|t| GeminiTool {
                function_declarations: vec![GeminiFunctionDeclaration {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                }],
            })
            .collect();

        let body = GeminiRequest {
            contents,
            system_instruction,
            tools: if tools.is_empty() { None } else { Some(tools) },
            generation_config: Some(GeminiGenerationConfig {
                temperature: req.temperature,
                max_output_tokens: req.max_tokens,
            }),
        };

        let resp = self
            .client
            .post(self.endpoint())
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .await
            .map_err(LlmError::from)?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status_error(status, text));
        }

        let parsed: GeminiResponse = resp.json().await.map_err(LlmError::from)?;
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
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GeminiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum GeminiPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "functionCall")]
    FunctionCall {
        name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "functionResponse")]
    FunctionResponse {
        name: String,
        response: serde_json::Value,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    #[serde(default)]
    content: Option<GeminiCandidateContent>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiCandidateContent {
    #[serde(default)]
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum GeminiPartResponse {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(alias = "functionCall", rename = "functionCall")]
    FunctionCall {
        name: String,
        #[serde(default)]
        args: serde_json::Value,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: u32,
    #[serde(default)]
    candidates_token_count: u32,
    #[serde(default)]
    total_token_count: u32,
}

impl GeminiResponse {
    fn into_generic(self) -> ChatResponse {
        let cand = self.candidates.into_iter().next();
        let (mut content, mut tool_calls) = (String::new(), Vec::new());
        let mut finish_reason_str = String::new();
        if let Some(c) = cand {
            finish_reason_str = c.finish_reason.unwrap_or_default();
            if let Some(cc) = c.content {
                for part in cc.parts {
                    match part {
                        GeminiPartResponse::Text { text } => content.push_str(&text),
                        GeminiPartResponse::FunctionCall { name, args } => {
                            let arguments = serde_json::to_string(&args).unwrap_or_default();
                            tool_calls.push(ToolCall {
                                id: String::new(),
                                name,
                                arguments,
                            });
                        }
                    }
                }
            }
        }
        let finish_reason = match finish_reason_str.to_uppercase().as_str() {
            "STOP" => FinishReason::Stop,
            "MAX_TOKENS" => FinishReason::Length,
            "SAFETY" | "RECITATION" => FinishReason::ContentFilter,
            _ => FinishReason::Other(finish_reason_str.clone()),
        };
        ChatResponse {
            content,
            tool_calls,
            finish_reason,
            usage: self.usage_metadata.map(|u| crate::llm::types::Usage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_built() {
        let p = GoogleProvider::new(
            reqwest::Client::new(),
            "".into(),
            "k".into(),
            "gemini-1.5-pro".into(),
        );
        assert_eq!(
            p.endpoint(),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent"
        );
    }

    #[test]
    fn parses_function_call_response() {
        let json = r#"{
            "candidates":[{
                "content":{
                    "parts":[
                        {"type":"text","text":"Sure."},
                        {"type":"functionCall","name":"propose_constant_change","args":{"name":"launchEnabled","value":1}}
                    ]
                },
                "finishReason":"STOP"
            }],
            "usageMetadata":{"promptTokenCount":40,"candidatesTokenCount":20,"totalTokenCount":60}
        }"#;
        let parsed: GeminiResponse = serde_json::from_str(json).unwrap();
        let g = parsed.into_generic();
        assert_eq!(g.tool_calls.len(), 1);
        assert_eq!(g.usage.unwrap().total_tokens, 60);
    }
}
