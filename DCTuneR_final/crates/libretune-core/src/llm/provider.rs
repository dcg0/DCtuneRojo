//! The [`Provider`] trait and a factory that selects a concrete provider.

use crate::llm::types::{ChatRequest, ChatResponse, LlmError};
use async_trait::async_trait;

/// All configuration needed to talk to one provider, in a provider-agnostic
/// form. This is what gets stored in user settings.
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    /// Which provider protocol to use: "openai", "anthropic", "google".
    pub provider: String,
    /// Base URL (e.g. `https://api.openai.com/v1` for OpenAI,
    /// `http://localhost:11434/v1` for a local Ollama exposing an
    /// OpenAI-compatible endpoint).
    pub base_url: String,
    /// API key / bearer token. Empty for local no-auth providers.
    pub api_key: String,
    /// Model identifier (e.g. `gpt-4o`, `claude-3-5-sonnet-...`, `gemini-1.5-pro`).
    pub model: String,
}

/// A chat-completion provider. Implementations translate [`ChatRequest`] to
/// their wire format, call the endpoint, and parse the response back into
/// [`ChatResponse`].
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name (matches [`ProviderConfig::provider`]).
    fn name(&self) -> &str;

    /// Send a chat-completion request.
    async fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError>;
}

/// Construct the concrete provider for a [`ProviderConfig`].
///
/// Returns [`LlmError::Config`] for an unknown provider id. The factory owns
/// its own `reqwest::Client` (built once, reused) so callers don't have to
/// thread one through.
pub fn build_provider(cfg: &ProviderConfig) -> Result<Box<dyn Provider>, LlmError> {
    let client = reqwest::Client::builder()
        .user_agent("LibreTune/0.1 (AI-Assistant)")
        .build()
        .map_err(|e| LlmError::Config(format!("failed to build HTTP client: {e}")))?;

    match cfg.provider.to_lowercase().as_str() {
        "openai" | "" => Ok(Box::new(
            crate::llm::providers::openai::OpenAiProvider::new(
                client,
                cfg.base_url.clone(),
                cfg.api_key.clone(),
                cfg.model.clone(),
            ),
        )),
        "anthropic" | "claude" => Ok(Box::new(
            crate::llm::providers::anthropic::AnthropicProvider::new(
                client,
                cfg.base_url.clone(),
                cfg.api_key.clone(),
                cfg.model.clone(),
            ),
        )),
        "google" | "gemini" => Ok(Box::new(
            crate::llm::providers::google::GoogleProvider::new(
                client,
                cfg.base_url.clone(),
                cfg.api_key.clone(),
                cfg.model.clone(),
            ),
        )),
        other => Err(LlmError::Config(format!(
            "unknown provider '{other}' (expected: openai, anthropic, google)"
        ))),
    }
}
