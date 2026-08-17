//! LLM provider client (bring-your-own).
//!
//! A minimal, provider-agnostic abstraction over OpenAI-compatible and other
//! chat-completion APIs. The agent orchestrator ([`crate::agent::orchestrator`])
//! only depends on the [`Provider`] trait, so adding a new provider is a matter
//! of implementing it — no orchestrator changes required.
//!
//! # Design
//! - All providers speak plain HTTPS via the existing `reqwest` client (no
//!   heavy vendor SDK crates). This keeps the dependency footprint minimal
//!   and works for both hosted and local (Ollama / LM Studio) endpoints.
//! - Tool/function-calling is modelled generically so each provider maps to
//!   its own wire format internally.
//! - Errors are a typed enum ([`LlmError`]) at this layer; Tauri commands
//!   flatten them to `Result<T, String>` at the boundary.
//!
//! # Modules
//! - [`types`] — request/response/tool message structs.
//! - [`provider`] — the [`Provider`] trait + a factory.
//! - [`client`] — [`LlmClient`] holding a `reqwest::Client` and the selected
//!   provider.
//! - [`providers`] — concrete implementations (OpenAI, Anthropic, Google).

pub mod client;
pub mod provider;
pub mod providers;
pub mod types;

pub use client::LlmClient;
pub use provider::{Provider, ProviderConfig};
pub use types::{
    ChatRequest, ChatResponse, FinishReason, LlmError, Message, MessageRole, ToolCall, ToolDef,
    ToolFunction,
};
